use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use serde::{Deserialize, Serialize};
use tauri::ipc::Channel;
use tauri::{AppHandle, State};
use tauri_plugin_shell::process::{CommandChild, CommandEvent};
use tauri_plugin_shell::ShellExt;

const BINARY: &str = if cfg!(windows) { "starnet2.exe" } else { "starnet2" };
const LEGACY_BINARY: &str = if cfg!(windows) { "starnet++.exe" } else { "starnet++" };

#[derive(Serialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum Detection {
  Found { path: String, version: String },
  Legacy { path: String },
  NotFound,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RunRequest {
  binary: PathBuf,
  input: PathBuf,
  output_dir: Option<PathBuf>,
  stride: Option<u32>,
  upsample: bool,
  linear: Option<LinearTuning>,
  highlight_protection: bool,
  mask: bool,
  unscreen: bool,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LinearTuning {
  shadows_clipping: Option<f64>,
  target_background: Option<f64>,
}

#[derive(Clone, Serialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum RunEvent {
  Progress { percent: f64 },
  Log { line: String },
}

#[derive(Serialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum RunOutcome {
  Done { outputs: Vec<String>, warnings: Vec<String> },
  Cancelled,
  Failed { message: String },
}

#[derive(Default)]
struct Runs {
  children: HashMap<u32, CommandChild>,
  reserved: HashSet<PathBuf>,
  stopping: bool,
}

#[derive(Default)]
pub struct ActiveRuns(Mutex<Runs>);

impl ActiveRuns {
  pub fn cancel(&self) {
    let mut runs = self.0.lock().unwrap();
    runs.stopping = true;
    for (_, child) in runs.children.drain() {
      let _ = child.kill();
    }
  }
}

fn standard_dirs() -> Vec<PathBuf> {
  let mut dirs: Vec<PathBuf> = Vec::new();
  if cfg!(windows) {
    let program_files =
      std::env::var_os("ProgramFiles").unwrap_or_else(|| r"C:\Program Files".into());
    dirs.push(Path::new(&program_files).join("StarNet2").join("bin"));
  } else {
    dirs.extend(["/usr/local/bin", "/usr/bin", "/opt/homebrew/bin"].map(PathBuf::from));
  }
  if let Some(path) = std::env::var_os("PATH") {
    dirs.extend(std::env::split_paths(&path));
  }
  dirs
}

fn with_bin_subfolder(dir: &Path) -> [PathBuf; 2] {
  [dir.to_path_buf(), dir.join("bin")]
}

#[derive(Deserialize)]
struct MachineInfo {
  version: String,
  options: Vec<MachineOption>,
}

#[derive(Deserialize)]
struct MachineOption {
  name: String,
}

async fn read_version(app: &AppHandle, binary: &Path) -> Option<String> {
  let output = app
    .shell()
    .command(binary)
    .arg("--machine-info")
    .current_dir(binary.parent()?)
    .output()
    .await
    .ok()?;
  if !output.status.success() {
    return None;
  }
  let info: MachineInfo = serde_json::from_slice(&output.stdout).ok()?;
  info.options.iter().any(|option| option.name == "machine_progress").then_some(info.version)
}

#[tauri::command]
pub async fn detect_starnet(app: AppHandle, custom: Option<PathBuf>) -> Detection {
  let dirs = match &custom {
    Some(dir) => with_bin_subfolder(dir).to_vec(),
    None => standard_dirs(),
  };

  let mut legacy = None;
  for dir in &dirs {
    let binary = dir.join(BINARY);
    if binary.is_file() {
      match read_version(&app, &binary).await {
        Some(version) => {
          return Detection::Found { path: binary.to_string_lossy().into_owned(), version }
        }
        None => legacy = legacy.or(Some(binary)),
      }
    }
    let old = dir.join(LEGACY_BINARY);
    if old.is_file() {
      legacy = legacy.or(Some(old));
    }
  }

  match legacy {
    Some(path) => Detection::Legacy { path: path.to_string_lossy().into_owned() },
    None => Detection::NotFound,
  }
}

fn split_name(input: &Path) -> (String, String) {
  let name = input.file_name().unwrap_or_default().to_string_lossy();
  let name = name.strip_suffix(".gz").or_else(|| name.strip_suffix(".GZ")).unwrap_or(&name);
  match name.rsplit_once('.') {
    Some((stem, ext)) => (stem.to_string(), ext.to_string()),
    None => (name.to_string(), "tif".to_string()),
  }
}

fn reserve_path(
  reserved: &mut HashSet<PathBuf>,
  dir: &Path,
  stem: &str,
  suffix: &str,
  ext: &str,
) -> PathBuf {
  let path = std::iter::once(dir.join(format!("{stem}_{suffix}.{ext}")))
    .chain((2..).map(|n| dir.join(format!("{stem}_{suffix}_{n}.{ext}"))))
    .find(|path| !path.exists() && !reserved.contains(path))
    .expect("an unused file name exists");
  reserved.insert(path.clone());
  path
}

#[derive(Deserialize)]
#[serde(tag = "schema")]
enum Record {
  #[serde(rename = "starnetastro.cli.progress.v1")]
  Progress(ProgressRecord),
  #[serde(rename = "starnetastro.cli.diagnostic.v1")]
  Diagnostic(Diagnostic),
}

#[derive(Deserialize)]
#[serde(tag = "event", rename_all = "lowercase")]
enum ProgressRecord {
  Start { percent: f64 },
  Progress { percent: f64 },
  Finish { percent: f64 },
  Error { message: String },
}

#[derive(Deserialize)]
struct Diagnostic {
  severity: Severity,
  message: String,
}

#[derive(Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
enum Severity {
  Info,
  Warning,
}

fn failure_summary(message: &str) -> String {
  if message.starts_with("Image size is too small") {
    return "This image is too small. StarNet2 needs at least 512 x 512 pixels, or 256 x 256 with Upsample on.".into();
  }
  message.lines().next().unwrap_or_default().to_string()
}

struct StartedRun {
  rx: tauri::async_runtime::Receiver<CommandEvent>,
  id: u32,
  paths: Vec<PathBuf>,
}

fn start_run(
  app: &AppHandle,
  state: &ActiveRuns,
  request: &RunRequest,
) -> Result<Option<StartedRun>, String> {
  let input_dir = request.input.parent().map(Path::to_path_buf).unwrap_or_default();
  let dir = request.output_dir.clone().unwrap_or(input_dir);
  let (stem, ext) = split_name(&request.input);

  let mut runs = state.0.lock().unwrap();
  if runs.stopping {
    return Ok(None);
  }

  let starless = reserve_path(&mut runs.reserved, &dir, &stem, "starless", &ext);
  let mut args: Vec<String> = vec![
    "--machine-progress".into(),
    "-i".into(),
    request.input.to_string_lossy().into_owned(),
    "-o".into(),
    starless.to_string_lossy().into_owned(),
  ];
  let mut paths = vec![starless];

  if request.mask {
    let path = reserve_path(&mut runs.reserved, &dir, &stem, "starmask", &ext);
    args.extend(["-m".into(), path.to_string_lossy().into_owned()]);
    paths.push(path);
  }
  if request.unscreen {
    let path = reserve_path(&mut runs.reserved, &dir, &stem, "stars", &ext);
    args.extend(["-n".into(), path.to_string_lossy().into_owned()]);
    paths.push(path);
  }
  if let Some(stride) = request.stride {
    args.extend(["-s".into(), stride.to_string()]);
  }
  if request.upsample {
    args.push("-u".into());
  }
  if let Some(tuning) = &request.linear {
    args.push("--linear".into());
    if let Some(value) = tuning.shadows_clipping {
      args.extend(["--shadows-clipping".into(), value.to_string()]);
    }
    if let Some(value) = tuning.target_background {
      args.extend(["--target-background".into(), value.to_string()]);
    }
  }
  if !request.highlight_protection {
    args.push("-d".into());
  }

  let binary_dir = request.binary.parent().map(Path::to_path_buf).unwrap_or_default();
  let spawned = app
    .shell()
    .command(&request.binary)
    .args(&args)
    .current_dir(binary_dir)
    .spawn();
  let (rx, child) = match spawned {
    Ok(spawned) => spawned,
    Err(error) => {
      for path in &paths {
        runs.reserved.remove(path);
      }
      return Err(format!("Could not start StarNet2: {error}"));
    }
  };
  let id = child.pid();
  runs.children.insert(id, child);
  Ok(Some(StartedRun { rx, id, paths }))
}

#[tauri::command]
pub async fn run_starnet(
  app: AppHandle,
  state: State<'_, ActiveRuns>,
  request: RunRequest,
  events: Channel<RunEvent>,
) -> Result<RunOutcome, String> {
  let Some(StartedRun { mut rx, id, paths }) = start_run(&app, &state, &request)? else {
    return Ok(RunOutcome::Cancelled);
  };

  let log = |line: String| {
    let _ = events.send(RunEvent::Log { line });
  };
  let mut failure = String::new();
  let mut warnings = Vec::new();
  let mut exit_code = None;
  while let Some(event) = rx.recv().await {
    match event {
      CommandEvent::Stdout(bytes) | CommandEvent::Stderr(bytes) => {
        let line = String::from_utf8_lossy(&bytes).trim().to_string();
        if line.is_empty() {
          continue;
        }
        match serde_json::from_str::<Record>(&line) {
          Ok(Record::Progress(
            ProgressRecord::Start { percent }
            | ProgressRecord::Progress { percent }
            | ProgressRecord::Finish { percent },
          )) => {
            let _ = events.send(RunEvent::Progress { percent });
          }
          Ok(Record::Progress(ProgressRecord::Error { message })) => {
            failure = failure_summary(&message);
            log(message);
          }
          Ok(Record::Diagnostic(diagnostic)) => {
            if diagnostic.severity == Severity::Warning {
              warnings.push(diagnostic.message.clone());
            }
            log(diagnostic.message);
          }
          Err(_) => log(line),
        }
      }
      CommandEvent::Error(message) => failure = message,
      CommandEvent::Terminated(payload) => exit_code = payload.code,
      _ => {}
    }
  }

  let cancelled = {
    let mut runs = state.0.lock().unwrap();
    for path in &paths {
      runs.reserved.remove(path);
    }
    runs.children.remove(&id).is_none()
  };

  if cancelled {
    return Ok(RunOutcome::Cancelled);
  }
  if exit_code == Some(0) {
    let outputs = paths.iter().map(|path| path.to_string_lossy().into_owned()).collect();
    return Ok(RunOutcome::Done { outputs, warnings });
  }
  Ok(RunOutcome::Failed {
    message: if failure.is_empty() {
      format!("StarNet2 stopped unexpectedly (exit code {exit_code:?}).")
    } else {
      failure
    },
  })
}

#[tauri::command]
pub fn begin_batch(state: State<'_, ActiveRuns>) {
  state.0.lock().unwrap().stopping = false;
}

#[tauri::command]
pub fn cancel_starnet(state: State<'_, ActiveRuns>) {
  state.cancel();
}
