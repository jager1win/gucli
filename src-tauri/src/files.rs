use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::fs;
use std::io::{self, Write};
use std::path::PathBuf;
use tracing::{error};
use tracing_subscriber::fmt::writer::MakeWriter;
use uuid::Uuid;

pub const COMMANDS_FILE: &str = ".config/gucli/commands.toml";
pub const LOG_FILE: &str = ".config/gucli/gucli.log";

// Structure for TOML (without ID)
#[derive(Serialize, Deserialize)]
pub struct TomlCommand {
    pub shell: String,
    pub command: String,
    pub icon: String,
    pub sn: bool,
}

// Configuration for TOML
#[derive(Serialize, Deserialize)]
pub struct CommandsConfig {
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
        let mut new_content = String::from_utf8_lossy(buf).trim().to_string();
        new_content.push('\n');

        // read content
        let content = if self.path.exists() {
            fs::read_to_string(&self.path)?
        } else {
            String::new()
        };

        // Adding a new entry with a line break
        if !content.is_empty() {
            new_content.push_str(&content);
        }
        //content.push_str(&message);

        // Truncating to max_lines
        let lines: Vec<&str> = new_content.lines().collect();
        let truncated = if lines.len() > self.max_lines {
            lines[lines.len() - self.max_lines..].join("\n")
        } else {
            new_content
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
    get_home_dir()
        .expect("Home dir not found")
        .join(COMMANDS_FILE)
}

/// return full path LOG_FILE
pub fn full_path_log() -> PathBuf {
    get_home_dir().expect("Home dir not found").join(LOG_FILE)
}

/// set commands.toml on install app, check on run & reset
pub fn set_config(reset: Option<bool>) -> io::Result<String> {
    let reset = reset.unwrap_or(false);
    let commands_path = full_path_commands();

    if !commands_path.exists() || reset {
        fs::create_dir_all(commands_path.parent().unwrap())?;
        fs::write(&commands_path, COMMENT.to_string() + EXAMPLE_COMMANDS)?;

        Ok("File commands.toml created".to_string())
    } else {
        Ok("File commands.toml exists".to_string())
    }
}

/// read commands.toml + add id
pub fn load_commands() -> Result<crate::AppCommandsConfig, Box<dyn std::error::Error>> {
    let content = fs::read_to_string(full_path_commands())?;
    let toml_config: CommandsConfig = toml::from_str(&content)
        .map_err(|e| {
            error!("TOML parsing error: {}", e);
            format!("Invalid TOML syntax: {}", e)
        })?;

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
            error!(
                "Icon '{}' at index {} exceeds 8 characters limit",
                cmd.icon, index
            );
            return Err("Icon exceeds 8 characters limit".into());
        }

        // validate shell field
        let valid_shells = ["sh", "bash", "zsh", "fish"];
        if !valid_shells.contains(&cmd.shell.as_str()) {
            error!(
                "Invalid shell '{}' at index {}. Available values: {:?}",
                cmd.shell, index, valid_shells
            );
            return Err(format!("Invalid shell. Available values: {:?}", valid_shells).into());
        }
    }

    let commands_with_id = toml_config
        .commands
        .into_iter()
        .map(|toml_cmd| crate::UserCommand {
            id: Uuid::new_v4().to_string(),
            shell: toml_cmd.shell,
            command: toml_cmd.command,
            icon: toml_cmd.icon,
            sn: toml_cmd.sn,
        })
        .collect();

    Ok(crate::AppCommandsConfig {
        commands: commands_with_id,
    })
}

/// write commands.toml + remove id
pub fn save_commands(config: &crate::AppCommandsConfig) -> Result<(), Box<dyn std::error::Error>> {
    let toml_commands: Vec<TomlCommand> = config
        .commands
        .iter()
        .map(|cmd| TomlCommand {
            shell: cmd.shell.clone(),
            command: cmd.command.clone(),
            icon: cmd.icon.clone(),
            sn: cmd.sn,
        })
        .collect();

    let toml_config = CommandsConfig {
        commands: toml_commands,
    };
    let _ = fs::write(
        full_path_commands(),
        COMMENT.to_string() + &toml::to_string(&toml_config)?,
    );
    Ok(())
}

static COMMENT: &str = r#"# The application requires at least one command to function.
# Please follow the field structure:
# [[commands]] - defines one element in the commands collection. Required for each command.
# shell - string (default: "sh"), available values: [sh, bash, zsh, fish]. Required when using shell aliases or functions
# command - string (unique), can include arguments and shell-specific syntax
# icon - string (max 8 characters), UTF-8 symbols, text or empty - displays in system tray menu
# sn - boolean (default: true, write without quotes), send command result to system notification
"#;

static EXAMPLE_COMMANDS: &str = r#"
[[commands]]
shell = "sh"
command = "echo $SHELL"
icon = "😀"
sn = true

[[commands]]
shell = "sh"
command = "id"
icon = "🚀"
sn = true
"#;
