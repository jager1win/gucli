use crate::CommandsConfig;
use crate::LOG_FILE;
use crate::SETTINGS_FILE;
use chrono::Local;
use dirs::home_dir;
use std::fs::{self, OpenOptions, File};
use std::io::{BufRead, BufReader, Write};
use std::path::Path;

/// set settings.toml in user home folder
pub fn full_path_settings() -> std::path::PathBuf {
    let home = home_dir().expect("Could not find home directory");
    home.join(SETTINGS_FILE)
}

/// set gucli.log in user home folder
pub fn full_path_log() -> std::path::PathBuf {
    let home = home_dir().expect("Could not find home directory");
    home.join(LOG_FILE)
}

/// set settings.toml on install app, check on run & reset
pub fn set_config(reset: Option<bool>) -> std::io::Result<String> {
    let reset = reset.unwrap_or(false);
    let setting_path = full_path_settings();

    if !setting_path.exists() || reset {
        println!("File {:?} don`t exist or reset. make...", &setting_path);

        let dir = Path::new(&setting_path)
            .parent()
            .expect("Failed to get parent directory");
        std::fs::create_dir_all(dir)?;
        let mut file = File::create(setting_path)?;

        writeln!(file,"# params: command=string(with args), active=bool(default true), system_notification=bool(default=true)")?;
        writeln!(file, "[[command]]")?;
        writeln!(file, "name = \"hostname -A\"")?;
        writeln!(file, "active = true")?;
        writeln!(file, "system_notification = true")?;

        Ok("File created".to_string())
    } else {
        if !Path::new(&full_path_log()).is_file() {
            set_log("gucli".to_string(),"File gucli.log created".to_string());
        }
        Ok("File exist".to_string())
    }
}

/// read settings.toml
pub fn load_commands() -> Result<CommandsConfig, Box<dyn std::error::Error>> {
    let path = full_path_settings();
    let content = fs::read_to_string(path)?;
    Ok(toml::from_str(&content)?)
}

/// write settings.toml
pub fn save_commands(config: &CommandsConfig) -> Result<(), Box<dyn std::error::Error>> {
    let path = full_path_settings();
    let content = toml::to_string(config)?;
    fs::write(path, content)?;
    Ok(())
}

/// update gucli.log
pub fn set_log(func:String, line: String){
    let path = full_path_log();
    
    // Читаем существующие строки (если файл есть)
    let lines = if let Ok(file) = fs::File::open(&path) {
        BufReader::new(file).lines().map_while(Result::ok).collect()
    } else {
        Vec::new()
    };

    // fix max count strings
    let mut lines = lines;
    if lines.len() >= 100 {
        lines.remove(0);
    }
    
    // add string
    lines.push(format!("{} ** {} ** {}", Local::now().format("%Y-%m-%d %H:%M:%S.%3f"), func, line));

    // write
    let _ = OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .open(&path)
        .map(|mut file| {
            for l in lines {
                let _ = writeln!(file, "{l}"); // ignore errors
            }
        });
}
