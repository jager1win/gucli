use std::fs;
use std::fs::File;
use std::io::Write;
use std::path::Path;
use crate::CommandsConfig;
use crate::SETTINGS_FILE;

/// set settings.toml in user home folder
pub fn full_path_settings() -> std::path::PathBuf {
    let home = std::env::home_dir().unwrap();
    home.join(SETTINGS_FILE)
}

/// set settings.toml on install app, check on run & reset 
pub fn set_config(reset: Option<bool>) -> std::io::Result<String> {
    let reset = reset.unwrap_or(false);
    let file_path = full_path_settings();

    if !file_path.exists() || reset {
        println!("File {:?} don`t exist or reset. make...", &file_path);

        let dir = Path::new(&file_path)
            .parent()
            .expect("Failed to get parent directory");
        std::fs::create_dir_all(dir)?;
        let mut file = File::create(file_path)?;

        writeln!(file, "# params: command=string(with args), active=bool(default true), system_notification=bool(default=true)")?;
        writeln!(file, "[[command]]")?;
        writeln!(file, "name = \"hostname -A\"")?;
        writeln!(file, "active = true")?;
        writeln!(file, "system_notification = true")?;

        println!("File created");
        Ok("File created".to_string())
    } else {
        println!("File exist");
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
