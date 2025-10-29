## Gucli - Your personal command center in the system tray
Gucli (from GUI + CLI) is a system tray menu for Linux that converts user console commands into accessible menu items for one-click or keyboard launch. Execution results are displayed in system notifications by default. The application can be useful for both advanced users and people with disabilities.

### Application Limitations
- Execution timeout: 500 ms. For longer operations, add `&` at the end of the command
- Notification limit: 200 characters. Exceeding this may cause shell freezing

### Command Configuration
A configuration file is created on first launch - `~/.config/gucli/commands.toml` with 2 default command examples.
The TOML format is very simple and convenient for editing. The structure is detailed in the initial comments. Here's its content:
```toml
# The application requires at least one command to function.
# Please follow the field structure:
# [[commands]] - defines one element in the commands collection. Required for each command.
# shell - string (default: "sh"), available values: [sh, bash, zsh, fish]. Required when using shell aliases or functions
# command - string (unique), can include arguments and shell-specific syntax
# icon - string (max 8 characters), UTF-8 symbols, text or empty - displays in system tray menu
# sn - boolean (default: true, write without quotes), send command result to system notification

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
```
After editing settings, the application needs to be restarted.
You can also bind your own commands through the GUI: Systray→Gucli→Settings.
Additionally, in the application settings window you can:
- Add the program to autostart
- Open commands.toml & gucli.log files in the default editor with one click
- Reset commands.toml to default values as shown above
- Edit commands and test them immediately
- Get help information for a command by simply entering it - the application will search through console outputs like --help, man, etc

### Usage
Main scenario: select a command from the tray menu → get the result in notification.

NOT RECOMMENDED!!! Using long-running commands (like watch) in the program - use a full terminal for these, as they will hang in processes. ⚠️ The application does not restrict executed commands. Make sure to add only verified commands.

IMPORTANT!!! Always remember the execution time and output limitations, and always test before adding.

Otherwise, it's all individual - systemctl, docker, etc. I recommend moving complex or long sequences to aliases or scripts (bash/zsh/fish) and calling them with short commands, for example `sh my_script.sh --f1`

Execution results are saved in `~/.config/gucli/gucli.log`. The last 100 lines are preserved (log rotation). Timestamp-command-result or application error is written to the beginning of the file.

### ♿ Accessibility
The application includes full support for accessibility features:
- UI themes: Light, Dark, and High-Contrast for visually impaired users
- Full keyboard navigation in all interface elements
- ARIA attributes for proper screen reader compatibility
- One-time setup - permanent convenience of use

### Tech Stack
- **Created:** [Tauri](https://github.com/tauri-apps/tauri) + [Leptos](https://github.com/leptos-rs/leptos)
- **Dependencies:** gtk3, webkit2gtk, libappindicator, libnotify
- **Tested on OS:** Arch(GNOME, KDE), Ubuntu 25.04 (GNOME, KDE)
- **Repository: available on Arch AUR** https://aur.archlinux.org/packages/gucli





