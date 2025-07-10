use serde::{Deserialize, Serialize};
use tauri::{
    menu::{MenuItem, MenuBuilder},
    tray::TrayIconBuilder,
    Manager, Runtime,
};
use notify_rust::{Notification, Timeout};
use std::process::Command as stdcom;
mod files;
use crate::files::*;
use tauri_plugin_autostart::*;

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
    save_commands(&config).map_err(|e| e.to_string())?;
    Ok("Commands saved".to_string())
}

#[tauri::command]
async fn reset_commands() -> Result<String, String> {
    set_config(Some(true)).map_err(|e| e.to_string())?;
    Ok("Settings reset to default".to_string())
}

#[tauri::command]
async fn request_restart(app: tauri::AppHandle) {
    app.restart();
}

#[tauri::command]
async fn run_test(cmd: Command) -> String {
    match run_command(&cmd) {
        Ok(success) => success,
        Err(error) => error
    }
}

#[tauri::command]
async fn enable_autostart(app: tauri::AppHandle) -> Result<String, String> {
    app.autolaunch()
        .enable()
        .map(|_| "Autostart enabled".to_string())
        .map_err(|e| format!("Failed to enable autostart: {e}"))
}

#[tauri::command]
async fn disable_autostart(app: tauri::AppHandle) -> Result<String, String> {
    app.autolaunch()
        .disable()
        .map(|_| "Autostart disabled".to_string())
        .map_err(|e| format!("Failed to disable autostart: {e}"))
}

#[tauri::command]
async fn is_autostart_enabled(app: tauri::AppHandle) -> Result<String, String> {
    app.autolaunch()
        .is_enabled()
        .map(|enabled| {
            if enabled {
                "Autostart is enabled".to_string()
            } else {
                "Autostart is disabled".to_string()
            }
        })
        .map_err(|e| format!("Failed to check autostart status: {e}"))
}

pub fn run() {
    if let Err(e) = set_config(None) {
        log::error!("Failed to init config: {e}");
    }

    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, _argv, _cwd| {
            // Фокусируем окно при попытке запуска второй копии
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.set_focus();
            }
        }))
        .setup(|app| {
            let window = app.get_webview_window("main").unwrap();
            let window_clone = window.clone();
            window.on_window_event(move |event| {
                if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                    api.prevent_close();
                    window_clone.hide().unwrap();
                }
            });

            let commands_config = load_commands().unwrap();
            
            let settings = MenuItem::with_id(app, "main", "Settings", true, None::<&str>)?;
            let restart = MenuItem::with_id(app, "restart", "Restart", true, None::<&str>)?;
            let quit = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;

            let mut menu_items = Vec::new();
            for cmd in &commands_config.commands {
                if cmd.active {
                    let item = MenuItem::with_id(
                        app,
                        format!("cmd_{}", cmd.name),
                        &cmd.name,
                        true,
                        None::<&str>,
                    )?;
                    menu_items.push(item);
                }
            }

            let mut builder = MenuBuilder::new(app);
            for item in menu_items {
                builder = builder.item(&item);
            }
            let menu = builder
                .separator()
                .item(&settings)
                .item(&restart)
                .item(&quit)
                .build()?;

            TrayIconBuilder::new()
                .icon(app.default_window_icon().unwrap().clone())
                .menu(&menu)
                .on_menu_event(move |app, event| {
                    match event.id.as_ref() {
                        "main" => toggle_window(app),
                        "restart" => app.restart(),
                        "quit" => app.exit(0),
                        id if id.starts_with("cmd_") => {
                            let cmd_name = id.replace("cmd_", "");
                            if let Some(cmd) = commands_config.commands.iter().find(|c| c.name == cmd_name) {
                                let _ = run_command(cmd);
                            }
                        }
                        _ => {}
                    }
                })
                .build(app)?;

            Ok(())
        })
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_autostart::Builder::new().build())
        .invoke_handler(tauri::generate_handler![
            get_commands,
            set_commands,
            reset_commands,
            run_test,
            request_restart,
            enable_autostart,
            disable_autostart,
            is_autostart_enabled
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

fn toggle_window<R: Runtime>(app: &tauri::AppHandle<R>) {
    if let Some(window) = app.get_webview_window("main") {
        if window.is_visible().unwrap_or(false) {
            window.hide().unwrap();
        } else {
            window.show().unwrap();
            window.set_focus().unwrap();
        }
    }
}

fn run_command(cmd: &Command) -> Result<String, String> {
    log::debug!("Executing command: {}", &cmd.name);
    let result = execute_command(&cmd.name);
    
    let (is_success, message) = match result {
        Ok(output) => (true, format!("Ok( Command <{}> executed )\nResult: {}", &cmd.name, &output)),
        Err(err) => (false, format!("Err( executing command <{}> )\nError: {}", &cmd.name, &err)),
    };

    if !is_success || cmd.system_notification {
        let (summary, body) = message.split_at(message.find('\n').unwrap_or(message.len()));
        if let Err(e) = Notification::new()
            .summary(summary)
            .body(body.trim_start_matches('\n'))
            .icon("system")
            .timeout(Timeout::Milliseconds(200))
            .show()
        {
            log::error!("Failed to show notification: {e}");
        }
    }

    if is_success { Ok(message) } else { Err(message) }
}

fn execute_command(command: &str) -> Result<String, String> {
    let output = stdcom::new("sh")
        .arg("-c")
        .arg(command)
        .output()
        .map_err(|e| e.to_string())?;

    if output.status.success() {
        String::from_utf8(output.stdout).map_err(|e| e.to_string())
    } else {
        Err(String::from_utf8(output.stderr).unwrap_or_else(|_| "Unknown error".to_string()))
    }
}