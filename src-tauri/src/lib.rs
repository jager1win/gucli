use serde::{Deserialize, Serialize};
use std::{fs, env, process::Command};
use tauri::{
    Manager, Runtime,
    menu::{MenuBuilder, MenuItem},
    tray::TrayIconBuilder,
};
use tracing::{debug, error, info};
pub mod files;
use crate::files::*;
use std::process::{Stdio};
use std::time::{Duration, Instant};
use std::thread;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserCommand {
    pub id: usize,
    pub command: String,
    pub icon: String,
    pub sn: bool,
}

#[derive(Default, Debug, Serialize, Deserialize)]
pub struct AppCommandsConfig {
    pub commands: Vec<UserCommand>,
}

#[tauri::command]
fn get_man(cmd: &str) -> Result<String, String> {
    if cmd.trim().is_empty() {
        return Err("Enter the command to search for help".to_string());
    }

    const MIN_HELP_LENGTH: usize = 50;// Minimum length for valid help output (short outputs are considered errors)

    // Flags that should be executed as-is (with their original formatting)
    let help_flags = [" --help", " -h", " --usage", " help", " -help", " -?", " --longhelp", " --long-help", " --help-all", " info"];

    // Read & return exactly as entered when help flags are present
    if help_flags.iter().any(|&flag| cmd.contains(flag)) {
        let output = read_man(cmd)?;
        return Ok(process_man_output(output));
    }

    // find varians when help flags are not present
    let mut variants: Vec<String> = help_flags.iter()
    .map(|flag| format!("{}{}", cmd, flag))
    .collect();
    variants.push(format!("MANPAGER=cat man {}", cmd));

    for variant in &variants {
        match read_man(variant) {
            Ok(output) if output.len() >= MIN_HELP_LENGTH => {
                return Ok(process_man_output(output));
            }
            _ => continue, // next variant
        }
    }

    Err(format!("No valid help found for '{}'", cmd))
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
async fn get_commands() -> Result<Vec<UserCommand>, String> {
    let config = load_commands().map_err(|e| e.to_string())?;
    Ok(config.commands)
}

#[tauri::command]
async fn set_commands(commands: Vec<UserCommand>) -> Result<String, String> {
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
async fn run_test(cmd: UserCommand) -> Result<String, String>  {
    match run_command(cmd.command, cmd.sn) {
        Ok(success) => Ok(success),
        Err(error) => Ok(error),
    }
}

#[tauri::command]
fn get_app_info() -> Vec<String> {
    let mut result = Vec::new();
    result.push(format!("Version: {}", env!("CARGO_PKG_VERSION")));
    result.push(format!("Authors: {}", env!("CARGO_PKG_AUTHORS")));
    result.push(format!("License: {}", env!("CARGO_PKG_LICENSE")));
    result.push(env!("CARGO_PKG_REPOSITORY").to_string());
    
    result
}

#[tauri::command]
async fn autostart_toggle() -> Result<String, String> {
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
            let restart = MenuItem::with_id(app, "restart", "🔃   Restart", true, None::<&str>)?;
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
            autostart_toggle,
            autostart_status,
            get_man,
            get_app_info
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

fn open_settings<R: Runtime>(app: &tauri::AppHandle<R>) {
    // Closing the window if it is open
    if let Some(window) = app.get_webview_window("settings") {
        window.close().unwrap();
    } else {
        // Creating a new window
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
            format!("Ok( Command `{}` executed ), Result:\n {}", &cmd, &output),
        ),
        Err(err) => (
            false,
            format!("Err( Command `{}` failed ), Error:\n {}", &cmd, &err),
        ),
    };

    // push to log
    match result {
        Ok(val) => info!("Command `{}` executed, Result: {}", cmd.clone(), val.replace("\n", " ")),
        Err(err) => error!("Command `{}` failed, Error: {}", cmd.clone(), err),
    }

    // send notification if fail or enable sn
    if !is_success || sn {
        let (summary, body) = message.split_at(message.find('\n').unwrap_or(message.len()));
        let limited_body = if body.chars().count() > 200 {
            format!("{}...", body.chars().take(200).collect::<String>())
        } else {
            body.to_string()
        };
        send_notification(summary, &limited_body);
    }

    Ok(message)
}

fn execute_command(command: &str) -> Result<String, String> {
    let timeout_secs = 0.5; // Hard limit of 500 ms
    let check_interval = Duration::from_millis(100); // Check every 100 ms
    let mut child = Command::new("sh")
        .arg("-c")
        .arg(command)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| format!("Failed to spawn process: {}", e))?;

    let start = Instant::now();
    let timeout = Duration::from_secs_f64(timeout_secs);

    // Execution time monitoring with periodic check
    while start.elapsed() < timeout {
        match child.try_wait() {
            Ok(Some(status)) => {
                // Process completed
                let output = child.wait_with_output()
                    .map_err(|e| format!("Failed to get output: {}", e))?;

                return if status.success() {
                    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
                    Ok(stdout)
                } else {
                    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
                    Err(stderr)
                };
            }
            Ok(None) => {
                // process is still running, we are waiting a bit
                thread::sleep(check_interval);
            }
            Err(e) => return Err(format!("Error waiting for process: {}", e)),
        }
    }

    // timeout is exceeded - we kill the process and all child processes
    let _ = child.kill();
    
    // Give the process some time to finish correctly
    thread::sleep(Duration::from_millis(100));
    let _ = child.wait();

    Err(format!("Command timed out after {} seconds", timeout_secs))
}

fn send_notification(summary: &str, body: &str) {
    if Command::new("notify-send").arg("--version").output().is_ok() {
        let _ = Command::new("notify-send")
            .arg(summary)
            .arg(body)
            .arg("--app-name=gucli-tray")
            .arg("--icon=system")
            .status();
    } else {
        error!("notify-send not found. Notification skipped: {} - {}", summary, body);
    }
}

// format help to html
pub fn process_man_output(output: String) -> String {
    let url_regex = regex::Regex::new(r"<(\bhttps?://[^\s>]+)>").unwrap();
    let with_links = url_regex.replace_all(&output, |caps: &regex::Captures| {
        format!(r#"<a href="{}" class="man-link">{}</a>"#, &caps[1], &caps[1])
    });

    let patterns = [
        (r"(?:^|\s)(-{1,2}[a-zA-Z0-9][^\s]*)", "man-dash"),
        (r"\b([A-Z]{2,})\b", "man-uppercase"),
    ];

    let mut result = with_links.to_string();
    for (pattern, class_name) in patterns.iter() {
        if let Ok(re) = regex::Regex::new(pattern) {
            result = re.replace_all(&result, |caps: &regex::Captures| {
                format!(r#"<span class="{}">{}</span>"#, class_name, &caps[0])
            }).to_string();
        }
    }
    
    result
}

fn read_man(cmd: &str)->Result<String, String>{
    let max_chars:usize = 30000;
    let output = Command::new("sh")
        .arg("-c")
        .arg(cmd)
        .output()
        .map_err(|e| format!("Failed read_man: {}", e))?;
    
    // We combine stdout and stderr, since the help can be in any of them
    let result = if !output.stdout.is_empty() {
        let s = String::from_utf8_lossy(&output.stdout).to_string();
         if s.chars().count() <= max_chars {
            s
         }else{
            s.chars().take(max_chars).collect()
         }
    } else {
        String::from_utf8_lossy(&output.stderr).to_string()
    };
    
    Ok(result)
}

/*
eprintln!("status: {}", &output.status);
eprintln!("stdout: {}", String::from_utf8_lossy(&output.stdout));
eprintln!("stderr: {}", String::from_utf8_lossy(&output.stderr));

*/