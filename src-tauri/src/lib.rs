use serde::{Deserialize, Serialize};
use tauri::{
    menu::{MenuItem, MenuBuilder},
    tray::{TrayIconBuilder},
    Manager, Runtime,
};
use notify_rust::*;
use std::process::Command as stdcom;
mod files;
use crate::files::*;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Command {
    pub name: String,
    pub active: bool,
    pub system_notification: bool,
}

#[derive(Default, Debug, Serialize, Deserialize)]
pub struct CommandsConfig {
    #[serde(rename = "command")]
    pub commands: Vec<Command>,
}

pub const SETTINGS_FILE: &str = ".config/gucli/settings.toml";

#[tauri::command]
async fn get_commands() -> Result<Vec<Command>, String> {
    let config = load_commands().map_err(|e| e.to_string())?;
    Ok(config.commands)
}

#[tauri::command]
async fn set_commands(commands: Vec<Command>) -> Result<String, String> {
    let config = CommandsConfig { commands };
    let result = save_commands(&config).map_err(|e| e.to_string());
    match result {
        Ok(_) => Ok("Commands saved".to_string()),
        Err(e) => Err(format!("Commands save return Err: {e}"))
    }
}

#[tauri::command]
async fn reset_commands() -> Result<String, String> {
    let res = set_config(Some(true)).map_err(|e| e.to_string());
    if res.is_ok() {
        Ok("The reset is done.".to_string())
    } else {
        Err(res?)
    }
}

#[tauri::command]
async fn request_restart(app: tauri::AppHandle) {
    app.cleanup_before_exit();
    app.restart();
}

#[tauri::command]
async fn run_test(cmd: Command) -> String {
    match run_command(&cmd) {
        Ok(success) => success,
        Err(error) => error
    }
}

pub fn run() {
    // init config on start
    if let Err(e) = set_config(None) {
        log::error!("Failed to init config: {e}");
    }

    tauri::Builder::default()
        .setup(|app| {
            let window = app.get_webview_window("main").unwrap();
            // add close for window
            window.clone().on_window_event(move |event| {
                if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                    api.prevent_close();
                    let _ = &window.hide();
                }
            });

            let commands_config = load_commands().unwrap();

            // base point menu
            let settings = MenuItem::with_id(app, "main", "Settings", true, None::<&str>)?;
            let restart = MenuItem::with_id(app, "restart", "Restart", true, None::<&str>)?;
            let quit = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;

            // create vector active command
            let mut command_items = Vec::new();
            for cmd in &commands_config.commands {
                if cmd.active {
                    let item = MenuItem::with_id(
                        app,
                        format!("cmd_{}", cmd.name),
                        &cmd.name,
                        true,
                        None::<&str>,
                    )?;
                    command_items.push(item);
                }
            }

            // replace `command_items` for menu
            let mut menu_refs: Vec<&dyn tauri::menu::IsMenuItem<_>> = Vec::new();
            for item in &command_items {
                menu_refs.push(item);
            }

            let menu = MenuBuilder::new(app)
                .items(&menu_refs)
                .separator()
                .item(&settings)
                .item(&restart)
                .item(&quit)
                .build()
                .unwrap();

            TrayIconBuilder::new()
                .on_menu_event(move |app, event| match event.id.as_ref() {
                    "main" => toggle_window(app),
                    "restart" => app.request_restart(),
                    "quit" => app.exit(0),
                    id if id.starts_with("cmd_") => {
                        let cmd_name = id.replace("cmd_", "");
                        if let Some(cmd) =
                            commands_config.commands.iter().find(|c| c.name == cmd_name)
                        {
                            let _ = run_command(cmd);
                        }
                    }
                    _ => {}
                })
                .icon(app.default_window_icon().unwrap().clone())
                .menu(&menu)
                .build(app)?;
            Ok(())
        })
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![get_commands,set_commands,reset_commands,run_test,request_restart])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

fn toggle_window<R: Runtime>(app: &tauri::AppHandle<R>) {
    if let Some(window) = app.get_webview_window("main") {
        if window.is_visible().unwrap_or(false) {
            let _ = window.close();
        } else {
            let _ = window.show();
            let _ = window.set_focus();
        }
    }
}

fn run_command(cmd: &Command) -> Result<String, String> {
    log::debug!("Executing command:[run_command] - {}", &cmd.name);

    let result = execute_command(&cmd.name);
    
    // Формируем сообщение для результата
    let (is_success, message) = match result {
        Ok(output) => {
            println!("Result: [run_command] - {}", &output);
            let msg = format!("Ok( Command <{}> executed )\nResult: {}", &cmd.name, &output);
            (true, msg)
        }
        Err(err) => {
            println!("Error: [run_command] - {}", &err);
            let msg = format!("Err( executing command <{}> )\nError: {}", &cmd.name, &err);
            (false, msg)
        }
    };

    // Show SN if:
    // 1. Error (always)
    // 2. If success & SN = true
    if !is_success || cmd.system_notification {
        let (summary, body) = message.split_at(message.find('\n').unwrap_or(message.len()));
        Notification::new()
            .summary(summary)
            .body(body.trim_start_matches('\n'))
            .icon("system")
            .timeout(Timeout::Milliseconds(200))
            .show()
            .unwrap();
    }

    if is_success {
        Ok(message)
    } else {
        Err(message)
    }
}

fn execute_command(command: &str) -> Result<String, String> {
    let output = stdcom::new("sh") // use "sh" for run command
        .arg("-c") // Set command as string
        .arg(command) 
        .output() 
        .map_err(|e| e.to_string())?; 

    if output.status.success() {
        String::from_utf8(output.stdout).map_err(|e| e.to_string())
    } else {
        Err(String::from_utf8(output.stderr).unwrap_or_else(|_| "Unknown error".to_string()))
    }
}
