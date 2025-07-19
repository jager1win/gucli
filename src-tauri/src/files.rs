use chrono::Local;
use dirs::home_dir;
use std::fs;
use std::io::{self, Write};
use std::path::{PathBuf};
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
        let message = String::from_utf8_lossy(buf)
            .trim()
            .to_string();
        
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
/// Полный путь к файлу команд
pub fn full_path_commands() -> PathBuf {
    home_dir().expect("Home directory not found").join(COMMANDS_FILE)
}

/// Полный путь к лог-файлу
pub fn full_path_log() -> PathBuf {
    home_dir().expect("Home directory not found").join(LOG_FILE)
}

/// Инициализация конфига (без изменений)
pub fn set_config(reset: Option<bool>) -> io::Result<String> {
    let reset = reset.unwrap_or(false);
    let commands_path = full_path_commands();

    if !commands_path.exists() || reset {
        fs::create_dir_all(commands_path.parent().unwrap())?;
        
        let default_config = r#"
            # params: command=string(with args), active=bool(default true), system_notification=bool(default=true)
            [[command]]
            name = "hostname -A"
            active = true
            system_notification = true
        "#;

        fs::write(&commands_path, default_config.trim())?;
        Ok("File created".to_string())
    } else {
        Ok("File exists".to_string())
    }
}

/// Загрузка команд (без изменений)
pub fn load_commands() -> Result<crate::CommandsConfig, Box<dyn std::error::Error>> {
    let content = fs::read_to_string(full_path_commands())?;
    Ok(toml::from_str(&content)?)
}

/// Сохранение команд (без изменений)
pub fn save_commands(config: &crate::CommandsConfig) -> Result<(), Box<dyn std::error::Error>> {
    fs::write(full_path_commands(), toml::to_string(config)?)?;
    Ok(())
}


/*use crate::CommandsConfig;
use crate::LOG_FILE;
use crate::COMMANDS_FILE;
use chrono::Local;
use dirs::home_dir;
use std::fs::{self, OpenOptions, File};
use std::io::{self, BufRead, BufReader, Write};
use std::path::Path;
use std::path::PathBuf;
use tracing::{info,error};
use tracing_subscriber::fmt::writer::MakeWriter;

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
        let timestamp = Local::now().format("%Y-%m-%d %H:%M:%S ");
        let message = format!("{}{}", timestamp, String::from_utf8_lossy(buf));
        
        let content = if self.path.exists() {
            fs::read_to_string(&self.path).unwrap_or_default()
        } else {
            String::new()
        };
        let mut lines = content.lines().collect::<Vec<_>>();
        lines.push(&message);
        
        if lines.len() > self.max_lines {
            lines.remove(0);
        }
        
        fs::write(&self.path, lines.join("\n"))?;
        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

/// set commands.toml in user home folder
pub fn full_path_commands() -> std::path::PathBuf {
    let home = home_dir().expect("Could not find home directory");
    home.join(COMMANDS_FILE)
}

/// set gucli.log in user home folder
pub fn full_path_log() -> std::path::PathBuf {
    let home = home_dir().expect("Could not find home directory");
    home.join(LOG_FILE)
}

/// set commands.toml on install app, check on run & reset
pub fn set_config(reset: Option<bool>) -> std::io::Result<String> {
    let reset = reset.unwrap_or(false);
    let commands_path = full_path_commands();

    if !commands_path.exists() || reset {
        println!("File {:?} don`t exist or reset. make...", &commands_path);

        let dir = Path::new(&commands_path)
            .parent()
            .expect("Failed to get parent directory");
        std::fs::create_dir_all(dir)?;
        let mut file = File::create(commands_path)?;

        writeln!(file,"# params: command=string(with args), active=bool(default true), system_notification=bool(default=true)")?;
        writeln!(file, "[[command]]")?;
        writeln!(file, "name = \"hostname -A\"")?;
        writeln!(file, "active = true")?;
        writeln!(file, "system_notification = true")?;

        Ok("File created".to_string())
    } else {
        if !Path::new(&full_path_log()).is_file() {
            info!("File gucli.log created");
        }
        Ok("File exist".to_string())
    }
}

/// read commands.toml
pub fn load_commands() -> Result<CommandsConfig, Box<dyn std::error::Error>> {
    let path = full_path_commands();
    let content = fs::read_to_string(path)?;
    Ok(toml::from_str(&content)?)
}

/// write commands.toml
pub fn save_commands(config: &CommandsConfig) -> Result<(), Box<dyn std::error::Error>> {
    let path = full_path_commands();
    let content = toml::to_string(config)?;
    fs::write(path, content)?;
    Ok(())
}*/
