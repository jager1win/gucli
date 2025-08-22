use std::fs;
use std::io::{self, Write};
use std::path::PathBuf;
use serde::{Deserialize, Serialize};
use tracing_subscriber::fmt::writer::MakeWriter;
use std::collections::HashSet;
use tracing::{error};

pub const COMMANDS_FILE: &str = ".config/gucli/commands.toml";
pub const LOG_FILE: &str = ".config/gucli/gucli.log";

// Structure for TOML (without ID)
#[derive(Serialize, Deserialize)]
pub struct TomlCommand {
    pub command: String,
    pub icon: String,
    pub sn: bool,
}

// Configuration for TOML
#[derive(Serialize, Deserialize)]
pub struct CommandsConfig {
    #[serde(rename = "commands")]
    pub commands: Vec<TomlCommand>,
}

pub struct LineLimitedWriter {
    path: PathBuf,
    max_lines: usize,
}

impl LineLimitedWriter {
    pub fn new(path: PathBuf, max_lines: usize) -> Self {
        Self { path, max_lines }
    }
}

impl<'a> MakeWriter<'a> for LineLimitedWriter {
    type Writer = LineLimitedFile;

    fn make_writer(&'a self) -> Self::Writer {
        LineLimitedFile {
            path: self.path.clone(),
            max_lines: self.max_lines,
        }
    }
}

pub struct LineLimitedFile {
    path: PathBuf,
    max_lines: usize,
}

impl Write for LineLimitedFile {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        let message = String::from_utf8_lossy(buf).trim().to_string();

        // read content
        let mut content = if self.path.exists() {
            fs::read_to_string(&self.path)?
        } else {
            String::new()
        };

        // Adding a new entry with a line break
        if !content.is_empty() {
            content.push('\n');
        }
        content.push_str(&message);

        // Truncating to max_lines
        let lines: Vec<&str> = content.lines().collect();
        let truncated = if lines.len() > self.max_lines {
            lines[lines.len() - self.max_lines..].join("\n")
        } else {
            content
        };

        fs::write(&self.path, truncated)?;
        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

/// return linux home dir
pub fn get_home_dir() -> Result<PathBuf, String> {
    std::env::var("HOME")
        .map(PathBuf::from)
        .map_err(|_| "Failed to get $HOME".to_string())
}

/// return full path COMMANDS_FILE
pub fn full_path_commands() -> PathBuf {
    get_home_dir().expect("Home dir not found").join(COMMANDS_FILE)
}

/// return full path LOG_FILE
pub fn full_path_log() -> PathBuf {
    get_home_dir().expect("Home dir not found").join(LOG_FILE)
}

/// set commands.toml on install app, check on run & reset
pub fn set_config(reset: Option<bool>) -> io::Result<String> {
    let reset = reset.unwrap_or(false);
    let commands_path = full_path_commands();
    let example_commands = get_home_dir().expect("Home dir not found").join(".config/gucli/example.commands.toml");

    if !commands_path.exists() || reset {
        fs::create_dir_all(commands_path.parent().unwrap())?;

        let default_config = r#"# params: command=string(with args), icon=string(<= 8 chars), sn=bool(default=true)
[[commands]]
command = "hostname -A"
icon = "😀"
sn = true

[[commands]]
command = "id"
icon = "🚀"
sn = true"#;
        fs::write(&commands_path, default_config.trim())?;
        fs::write(&example_commands, default_config.trim())?;
        Ok("File commands.toml created".to_string())
    } else {
        Ok("File commands.toml exists".to_string())
    }
}

/// read commands.toml + add id
pub fn load_commands() -> Result<crate::AppCommandsConfig, Box<dyn std::error::Error>> {
    let content = fs::read_to_string(full_path_commands())?;
    let toml_config: CommandsConfig = toml::from_str(&content)?;
    
    let mut unique_commands = HashSet::new();
    
    for (index, cmd) in toml_config.commands.iter().enumerate() {
        // check empty command
        if cmd.command.trim().is_empty() {
            error!("Command at index {} is empty", index);
            return Err("Command cannot be empty".into());
        }

        // check unique command
        if !unique_commands.insert(&cmd.command) {
            error!("Command '{}' at index {} is not unique", cmd.command, index);
            return Err("Command is not unique".into());
        }
        
        // check len icon (<= 8 char))
        if cmd.icon.chars().count() > 8 {
            error!("Icon '{}' at index {} exceeds 8 characters limit", cmd.icon, index);
            return Err("Icon exceeds 8 characters limit".into());
        }
    }
    
    let commands_with_id = toml_config.commands
        .into_iter()
        .enumerate()
        .map(|(id, toml_cmd)| crate::UserCommand {
            id,
            command: toml_cmd.command,
            icon: toml_cmd.icon,
            sn: toml_cmd.sn,
        })
        .collect();
    
    Ok(crate::AppCommandsConfig { commands: commands_with_id })
}

/// write commands.toml + remove id
pub fn save_commands(config: &crate::AppCommandsConfig) -> Result<(), Box<dyn std::error::Error>> {
    let toml_commands: Vec<TomlCommand> = config.commands
        .iter()
        .map(|cmd| TomlCommand {
            command: cmd.command.clone(),
            icon: cmd.icon.clone(),
            sn: cmd.sn,
        })
        .collect();
    
    let toml_config = CommandsConfig { commands: toml_commands };
    let _ = fs::write(full_path_commands(), toml::to_string(&toml_config)?);
    Ok(())
}
