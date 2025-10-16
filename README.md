## Gucli - Your personal command center in the system tray
Gucli (от GUI + CLI) — это меню в системном трее Linux, которое преобразует пользовательские консольные команды в доступные пункты меню для запуска одним щелчком или через клавиатуру. Результат выполнения по умолчанию выводится в системные уведомления. Приложение может быть полезно как опытным пользователям, так и людям с ограниченными возможностями.

### Ограничения приложения
  - Таймаут выполнения: 500 мс. Для более длительных операций добавьте & в конец команды
  - Ограничение уведомлений: 200 символов. Превышение может вызвать зависание оболочки

### Настройка команд
Файл создается при первом запуске - `~/.config/gucli/commands.toml` с 2 дефолтными примерами команд.
Формат TOML очень прост и удобен для редактирования. В начальном комментарии подробно описана структура. Вот его содержимое:
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
После редактирования настроек приложение нужно перезапустить.
Так же свои команды можно забиндить в программу через GUI: Systray->Gucli->Settings.
Плюс в окне настроек приложения можно:
- добавить программу в автозагрузку
- открыть по клику файлы в редакторе по умолчанию файлы `commands.toml` & `gucli.log`
- сбросить `commands.toml` до дефолтных значений, указанных выше
- редактировать команды и сразу тестировать их
- получить справочную информацию по команде, просто введя команду, а приложение поищет по консольным выводам `--help`, `man` etc.

### Использование
Основной сценарий: выберите команду в меню → получите результат в уведомлении 

НЕ РЕКОМЕНДУЕТСЯ!!! Использовать долго выполняющиеся команды (например `watch`) в программе, для этого есть полноценный терминал, они просто повиснут в процессах. ⚠️ Приложение не ограничивает выполняемые команды. Убедитесь, что добавляете только проверенные команды.  

ВАЖНО!!! Не забывайте об ограничениях по времени выполнения и выводу, и так же всегда тестируйте перед добавлением.  

В остальном конечно все индивидуально - `systemctl`, `docker` etc. Рекомендую выносить сложные или длинные последовательности в `aliases` или скрипты (bash/zsh/fish) и вызывать их короткой командой, например `sh my_script.sh --f1`

### Логи
`~/.config/gucli/gucli.log` — сохраняются последние 100 строк (ротация логов). В начало файла записывается время-команда-результат или ошибка приложения.


### Tech Stack
- **Created:** [Tauri](https://github.com/tauri-apps/tauri) + [Leptos](https://github.com/leptos-rs/leptos)
- **Dependencies:** gtk3, webkit2gtk, libappindicator, libnotify
- **Tested on OS:** Arch(GNOME, KDE), Ubuntu 25.04 (GNOME, KDE)





