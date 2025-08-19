use std::fs;
use std::io::{self, Write};
use std::path::PathBuf;
use tracing_subscriber::fmt::writer::MakeWriter;

pub const COMMANDS_FILE: &str = ".config/gucli/commands.toml";
pub const LOG_FILE: &str = ".config/gucli/gucli.log";

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

        // Читаем текущее содержимое файла
        let mut content = if self.path.exists() {
            fs::read_to_string(&self.path)?
        } else {
            String::new()
        };

        // Добавляем новую запись с переносом строки
        if !content.is_empty() {
            content.push('\n');
        }
        content.push_str(&message);

        // Обрезаем до max_lines
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

    if !commands_path.exists() || reset {
        fs::create_dir_all(commands_path.parent().unwrap())?;

        let default_config = r#"# params: command=string(with args), active=bool(default true), sn=bool(default=true)
[[commands]]
command = "hostname -A"
icon = "😀"
sn = true

[[commands]]
command = "id"
icon = "⤁"
sn = true"#;
        fs::write(&commands_path, default_config.trim())?;
        Ok("File commands.toml created".to_string())
    } else {
        Ok("File commands.toml exists".to_string())
    }
}

/// read commands.toml
pub fn load_commands() -> Result<crate::CommandsConfig, Box<dyn std::error::Error>> {
    let content = fs::read_to_string(full_path_commands())?;
    Ok(toml::from_str(&content)?)
}

/// write commands.toml
pub fn save_commands(config: &crate::CommandsConfig) -> Result<(), Box<dyn std::error::Error>> {
    fs::write(full_path_commands(), toml::to_string(config)?)?;
    Ok(())
}

/// return linux home dir
pub fn get_home_dir() -> Result<PathBuf, String> {
    std::env::var("HOME")
        .map(PathBuf::from)
        .map_err(|_| "Failed to get $HOME".to_string())
}