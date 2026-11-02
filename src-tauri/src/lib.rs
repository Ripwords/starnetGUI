mod starnet;

use tauri::{Manager, RunEvent};

pub fn run() {
  tauri::Builder::default()
    .plugin(tauri_plugin_dialog::init())
    .plugin(tauri_plugin_opener::init())
    .plugin(tauri_plugin_process::init())
    .plugin(tauri_plugin_shell::init())
    .plugin(tauri_plugin_updater::Builder::new().build())
    .manage(starnet::ActiveRuns::default())
    .invoke_handler(tauri::generate_handler![
      starnet::detect_starnet,
      starnet::begin_batch,
      starnet::run_starnet,
      starnet::cancel_starnet,
    ])
    .build(tauri::generate_context!())
    .expect("error while building tauri application")
    .run(|app, event| {
      if let RunEvent::Exit = event {
        app.state::<starnet::ActiveRuns>().cancel();
      }
    });
}
