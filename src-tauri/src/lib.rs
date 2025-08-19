use serde::{Deserialize, Serialize};
use std::{fs, env, process::Command as stdcom};
use tauri::{
    Manager, Runtime,
    menu::{MenuBuilder, MenuItem},
    tray::TrayIconBuilder,
};
use tracing::{debug, error, info};
pub mod files;
use crate::files::*;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Command {
    pub id: usize,
    pub command: String,
    pub icon: String,
    pub sn: bool,
}

#[derive(Default, Debug, Serialize, Deserialize)]
pub struct AppCommandsConfig {
    pub commands: Vec<Command>,
}

#[tauri::command]
fn get_man(cmd: &str) -> Result<String, String> {
    if cmd.trim().is_empty() {
        return Err("Enter the command to search for help".to_string());
    }

    const MIN_HELP_LENGTH: usize = 50;// Minimum length for valid help output (short outputs are considered errors)

    // Flags that should be executed as-is (with their original formatting)
    let help_flags = [" --longhelp ", " --help-all ", " --help ", " help ", " -? ", "man ", " info ", " --usage ", " -help "];
    let has_help_flag = help_flags.iter().any(|&flag| cmd.contains(flag));

    // Try to execute the command (either with flags or help variants)
    let result = if has_help_flag {
        // Execute command exactly as entered when help flags are present
        execute_command(cmd)
    } else {
        // Try different help variants in sequence
        let variants = [
            format!("man -P cat {}", cmd),  // Try man first
            format!("{} --help", cmd),      // Then --help
        ];
        
        // Find the first variant that returns valid output
        variants.iter()
            .find_map(|cmd| {
                execute_command(cmd).ok().filter(|output| 
                    output.len() >= MIN_HELP_LENGTH 
                )
            })
            .ok_or(format!("No valid help found for '{}'", cmd))
    };

    match result {
        Ok(output) => Ok(output),
        Err(e) => Ok(e) // Return error message as normal output
    }
}

#[tauri::command]
async fn ctrl_window(action: &str, app: tauri::AppHandle) -> Result<(), tauri::Error> {
    let window = app.get_webview_window("settings").unwrap();
    let _ = match action {
        "min" => window.minimize(),
        "max0" => window.maximize(),
        "max1" => window.unmaximize(),
        "close" => window.close(),
        &_ => Ok(()),
    };
    Ok(())
}

#[tauri::command]
async fn get_commands() -> Result<Vec<Command>, String> {
    let config = load_commands().map_err(|e| e.to_string())?;
    Ok(config.commands)
}

#[tauri::command]
async fn set_commands(commands: Vec<Command>) -> Result<String, String> {
    let config = AppCommandsConfig { commands };
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
async fn run_test(cmd: Command) -> Result<String, String>  {
    match run_command(cmd.command, cmd.sn) {
        Ok(success) => Ok(success),
        Err(error) => Ok(error),
    }
}

pub fn run() {
    if let Err(e) = set_config(None) {
        error!("Failed to init config: {}",e);
        std::process::exit(1);
    }
    let commands_config = load_commands().unwrap_or_else(|err| {
        error!("Failed to load commands: {}", err);
        std::process::exit(1);
    });

    tauri::Builder::default()
        .setup(|app| {
            // tray menu
            let settings = MenuItem::with_id(app, "settings", "⚙️   Settings", true, None::<&str>)?;
            let restart = MenuItem::with_id(app, "restart", "🔃    Restart", true, None::<&str>)?;
            let quit = MenuItem::with_id(app, "quit", "✝️   Quit", true, None::<&str>)?;

            let mut menu_items = Vec::new();
            for cmd in &commands_config.commands {
                let item = MenuItem::with_id(
                    app,
                    format!("cmd_{}", cmd.command),
                    cmd.icon.clone()+&String::from("    ")+&cmd.command,
                    true,
                    None::<&str>,
                )?;
                menu_items.push(item);
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

            TrayIconBuilder::with_id("main")
                .icon(app.default_window_icon().unwrap().clone())
                .menu(&menu)
                .on_menu_event(move |app, event| match event.id.as_ref() {
                    "settings" => open_settings(app),
                    "restart" => app.restart(),
                    "quit" => app.exit(0),
                    id if id.starts_with("cmd_") => {
                        let cmd_com = id.replace("cmd_", "");
                        if let Some(cmd) = commands_config.commands.iter().find(|c| c.command == cmd_com) {
                            let _ = run_command(cmd_com, cmd.sn);
                        }
                    }
                    _ => {}
                })
                .build(app)?;

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            get_commands,
            set_commands,
            reset_commands,
            run_test,
            request_restart,
            ctrl_window,
            autostart_enable,
            autostart_status,
            get_man
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

fn open_settings<R: Runtime>(app: &tauri::AppHandle<R>) {
    // Закрываем окно, если оно открыто
    if let Some(window) = app.get_webview_window("settings") {
        window.close().unwrap();
    } else {
        // Создаём новое окно
        let _window =
            tauri::WebviewWindowBuilder::new(app, "settings", tauri::WebviewUrl::App("/".into()))
                .title("Settings")
                .inner_size(800.0, 600.0)
                .transparent(true)
                .decorations(false)
                .visible(true)
                .build()
                .unwrap();

        _window.set_focus().unwrap();
    }
}

fn run_command(cmd:String, sn:bool) -> Result<String, String> {
    debug!("Executing command: {}", &cmd);
    let result = execute_command(&cmd);

    let (is_success, message) = match &result {
        Ok(output) => (
            true,
            format!("Ok( Command <{}> executed ), Result:\n {}", &cmd, &output),
        ),
        Err(err) => (
            false,
            format!("Err( Command <{}> failed ), Error:\n {}", &cmd, &err),
        ),
    };

    // push to log
    match result {
        Ok(val) => info!("Command <{}> executed, Result: {}", cmd.clone(), val.replace("\n", " ")),
        Err(err) => error!("Command <{}> failed, Error: {}", cmd.clone(), err),
    }

    // send notification if fail or enable sn
    if !is_success || sn {
        let (summary, body) = message.split_at(message.find('\n').unwrap_or(message.len()));
        send_notification(summary, body);
    }

    Ok(message)
}

fn execute_command(command: &str) -> Result<String, String> {
    let output = stdcom::new("sh")
        .arg("-c")
        .arg(command)
        .output()
        .map_err(|e| e.to_string())?;

    if output.status.success() {
        let out = String::from_utf8(output.stdout).map_err(|e| e.to_string());
        let converted = ansi_to_html::convert(&out.unwrap()).unwrap();
        Ok(converted)
    } else {
        Err(String::from_utf8(output.stderr).unwrap_or_else(|_| "Unknown error".to_string()))
    }
}

fn send_notification(summary: &str, body: &str) {
    if stdcom::new("notify-send").arg("--version").output().is_ok() {
        let _ = stdcom::new("notify-send")
            .arg(summary)
            .arg(body)
            .arg("--app-name=gucli-tray")
            .arg("--icon=system")
            .status();
    } else {
        error!("notify-send not found. Notification skipped: {} - {}", summary, body);
    }
}

/// toggle autostart
#[tauri::command]
async fn autostart_enable() -> Result<String, String> {
    let enabled = autostart_status().await.map_err(|e| {error!(%e, "cannot get autostart status");e})?;
    let home = get_home_dir().map_err(|e| e.to_string())?;
    let desktop_path: std::path::PathBuf = home.join(".config/autostart/gucli.desktop");
    if enabled {// remove
        let _ = fs::remove_file(&desktop_path);
        Ok("autostart disabled".into())
    } else {// add
        let exec_path = env::current_exe().map_err(|e| e.to_string())?;

        if let Some(dir) = desktop_path.parent() {
            fs::create_dir_all(dir).map_err(|e| e.to_string())?;
        }

        let desktop_file = format!(
            "[Desktop Entry]\n\
            Name=Gucli\n\
            Type=Application\n\
            Categories=Utility\n\
            StartupNotify=true\n\
            Exec={}\n\
            X-KDE-autostart-after=panel\n\
            X-LXQt-Need-Tray=true\n\
            X-GNOME-Autostart-enabled=true\n",
            exec_path.display()
        );

        fs::write(&desktop_path, desktop_file).map_err(|e| e.to_string())?;
        Ok("autostart enabled".into())
    }
}

#[tauri::command]
async fn autostart_status() -> Result<bool, String> {
    let autostart_file = get_home_dir()
        .map_err(|e| e.to_string())?
        .join(".config/autostart/gucli.desktop");

    Ok(autostart_file.exists())
}
