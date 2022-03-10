#![cfg_attr(
  all(not(debug_assertions), target_os = "windows"),
  windows_subsystem = "windows"
)]

use tauri::api::process::{Command, CommandEvent};
use tauri::Manager;

#[derive(serde::Serialize)]
struct Payload {
  message: String,
}

fn rem_first_and_last(value: &str) -> &str {
  let mut chars = value.chars();
  chars.next();
  chars.next_back();
  chars.as_str()
}

fn main() {
  tauri::Builder::default()
    .setup(|app| {
      let handle = app.handle();
      let handle_kill = app.handle();
      app.listen_global("kill-command", move |event| {
        let handle_kill = handle_kill.clone();
        tauri::async_runtime::spawn(async move {
          let payload = event.payload().unwrap().split('"').collect::<Vec<&str>>();
          println!("{}", payload[1]);
          println!("{}", rem_first_and_last(payload[2]));
          if payload[1] == "kill" {
            Command::new("kill")
              .args(["-s", "9", rem_first_and_last(payload[2])])
              .spawn()
              .expect("Failed to kill starnet++");
          } else {
            Command::new("taskkill")
            .args(["/f", "/pid", rem_first_and_last(payload[2])])
            .spawn()
            .expect("Failed to kill starnet++");
          }
          handle_kill.emit_all("starnet-killed", "starnet_killed").expect("Failed to kill starnet-killed");
        });
      });
      app.listen_global("starnet-command",  move |event| {
        handle.emit_all("starnet-command-init", "starnet init").expect("starnet init failed");
        let handle = handle.clone();
        tauri::async_runtime::spawn(async move {
          let payload = event.payload().unwrap().split('"').collect::<Vec<&str>>();
          let (mut rx, mut cmd) = Command::new(payload[1])
            .current_dir(payload[3].into())
            .args([
              "input.tiff",
              payload[5].into(),
              payload[7].into(),
            ])
            .spawn()
            .expect("Failed to spawn command");
          handle.emit_all("get-pid", cmd.pid()).expect("Failed to get pid");
          
          let mut i = 0;
          while let Some(event) = rx.recv().await {
            if let CommandEvent::Stdout(line) = event {
              println!("got: {}", line);
              handle.emit_all("starnet-command-stdout", &line).expect("Failed to emit event");
              i += 1;
              if i == 4 {
                cmd.write("message from Rust\n".as_bytes()).unwrap();
                i = 0;
              }
            } else if let CommandEvent::Error(line) = event {
              println!("ERROR: {}", line);
              handle.emit_all("starnet-command-err", &line).expect("Failed to emit event");
              i += 1;
              if i == 4 {
                cmd.write("message from Rust\n".as_bytes()).unwrap();
                i = 0;
              }
            } else if let CommandEvent::Terminated(line) = event {
              println!("terminated: {:?}", line);
              handle.emit_all("starnet-command-terminated", &line).expect("Failed to emit event");
              i += 1;
              if i == 4 {
                cmd.write("message from Rust\n".as_bytes()).unwrap();
                i = 0;
              }
            }
          }
        });
        
      });

      Ok(())
    })
    .run(tauri::generate_context!())
    .expect("error while running tauri application");
}
