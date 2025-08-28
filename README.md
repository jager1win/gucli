## Gucli - Your personal command line menu

### Назначение
- Простое меню в системном трее для запуска сохранённых пользовательских CLI-команд по клику.
- Удобно, когда терминал открывать дольше или GUI программы отсутствует/ограничен.
- Упрощает повторный запуск длинных команд

### Возможности
- Автозапуск при входе в сессию (по умолчанию: выключен).
- Показ результата в системных уведомлениях (по умолчанию: включено). Уведомления предназначены для текстового вывода; длинные и бинарные выводы будут усечены или некорректны.
- Логирование результатов и ошибок в ~/.config/gucli/gucli.log — сохраняются последние 100 строк (усечённая запись).
- Cписок команд можно вручную редактировать в текстовом редакторе 
    открыв `/home/$USER/.config/gucli/commands.toml`
- Интерактивные команды и команды с постоянным выводом не рекомендованы.

### Минимальные требования
- gtk3 или gtk4
- webkit2gtk
- libappindicator или его аналоги для трея

### Специфика
- Иконки — Unicode‑символы: обеспечивают гибкость выбора для пользователя, уменьшают объём сборки и исключают дополнительные графические зависимости.
- Отображение полной команды в меню: в меню показывается буквальное имя команды, а не псевдо‑алиас. Это повышает прозрачность и снижает риск случайного выполнения неправильной команды.
- Длинные команды: длина строки в меню не ограничивается системой и очень длинные записи могут занимать всё пространство меню и ухудшать внешний вид. Рекомендуется вынести сложные или длинные последовательности в скрипты (sh/zsh) и вызывать их короткой командой, например `sh my_script.sh --f1`


### Файлы конфигурации
Команды: ~/.config/gucli/commands.toml
Лог: ~/.config/gucli/gucli.log
Пример commands.toml
```toml
# params: command=string(with args), icon=string(<= 8 chars), sn=bool(default=true)
[[commands]]
command = "hostname -A"
icon = "😀"
sn = true

[[commands]]
command = "id"
icon = "🚀"
sn = true
```
### Tested on OS: 
- Ubuntu 25.04 GNOME
### Открывайте Issues для багов и фич.

## Created using [Tauri](https://github.com/tauri-apps/tauri) + [Leptos](https://github.com/leptos-rs/leptos)


---

# Gucli

### Мост между терминалом и рабочим столом

**Gucli** (от **GUI** + **CLI**) — это лёгкое приложение для системного трея, которое даёт вам мгновенный графический доступ к любым консольным командам и скриптам. Запускайте часто используемые действия в один клик, не открывая терминал.


---

### ✨ **Зачем это нужно?**

Создайте своё собственное меню для всего, что умеет запускаться из командной строки:
*   **Управление сервисами и системой:** `systemctl`, `docker`, `podman`, `networkctl`.
*   **Сетевые утилиты:** `ping`, `curl`, `ssh`, `wget`, `rsync`.
*   **Мониторинг и диагностика:** `htop`, `nvidia-smi`, `journalctl`, `df`, `free`.
*   **Скрипты и автоматизация:** Ваши собственные Bash, Python, Ruby скрипты.
*   **Программы без своего GUI:** Многие мощные утилиты предоставляют только CLI-интерфейс — дайте им удобную кнопку!

> **Идеи для вдохновения:**
> `warp-cli connect` · `bluetoothctl power on` · `git pull` · `speedtest-cli` · `brightnessctl set 50%`

> **Совет:** Для сложных команд с длинными параметрами создайте скрипт и вызывайте его через Gucli. Это сохранит меню чистым и читаемым.
> `~/scripts/backup.sh` вместо `rsync -avz --delete /home/user /backup/...`

---

### 🌟 **Ключевые возможности**

*   **Предельная ясность:** В меню отображается сама команда, чтобы вы всегда понимали, что именно будет выполнено.
*   **Два способа настройки:** Редактируйте список команд через удобное графическое окно **или** напрямую правьте файл конфигурации в `~/.config/gucli/commands.toml`.
*   **Гибкие уведомления:** Для каждой команды можно индивидуально включить или отключить всплывающие уведомления с результатом (`sn = true/false`).
*   **Лёгкая персонализация:** Добавляйте emoji 🚀 или короткие символы для визуального оформления пунктов меню (`icon = "🔒"`).
*   **Полный контроль:** Вы решаете, что запускать и какую обратную связь получать.

**Пример структуры конфигурационного файла:**
```toml
# Настройки для каждой команды: command, icon, sn (system notifications)
[[commands]]
command = "systemctl restart nginx"
icon = "🌐"
sn = true

[[commands]]
command = "docker ps -a"
icon = "🐳"
sn = false

[[commands]]
command = "~/.scripts/deploy_project.sh"
icon = "🚀"
sn = true
```

---

### 🛠 **Как начать?**

1.  **Добавьте команды** через интерфейс настроек или отредактируйте TOML-файл конфигурации.
2.  **Ваше меню появится** в системном трее.
3.  **Запускайте** нужные действия в один клик.
4.  **Настраивайте** уровень обратной связи (уведомления) под каждую задачу.

---

### 💡 **Философия проекта**

**Gucli не заменяет ваш терминал.** Он **расширяет** его возможности, превращая частые, рутинные или сложные команды в простые и доступные действия. Это свобода настроить рабочее пространство под себя, без ограничений готовых графических интерфейсов.






# HELP
⚠ Внимание — GUCLI не заменяет терминал. Всегда тестируйте команды в терминале перед добавлением.
## Редактирование вручную и в окне настроек
Файл конфигурации находится `/home/$USER/.config/gucli/commands.toml`
Настройки по умолчанию выглядят следующим образом:
```toml
# params: command=string(with args), icon=string(<= 8 chars), sn=bool(default=true)
[[commands]]
command = "hostname -A"
icon = "😀"
sn = true

[[commands]]
command = "id"
icon = "🚀"
sn = true
```
Поля

- command — строка с командой и аргументами.
- icon — до 8 символов (UTF-8); может быть emoji, короткий текст или пусто.
- sn — показывать системное уведомление (bool, default: true). Уведомления об ошибках показываются всегда.
 

- На вкладке Settings настроек можно изменить очередность команд. Из файла они загружаются в таком же порядке как записаны.
- icon - поле типа String длиной 8 знаков. Можно использовать Unicode-символы как иконки, короткий текст или оставить пустым.
- sn (системное уведомление) всегда будет отображать сообщения в случае ошибок, даже если оно отключено.
- На вкладке "Man pages & --help" можно получить консольный вывод помощи, просто введя команду.


## Типы команд Linux - для корректных уведомлений:
- Обычные (например, `ls -la /home/$USER/Pictures`): можно преобразовать в строку и отобразить вывод в уведомлении
- Без вывода (например, `sleep`): уведомление выведет пустую строку
- Долго выполняющиеся (например, `watch`): нельзя преобразовать в строку. Категорически не рекомендую использовать подобные команды в программе, для этого есть полноценный терминал, они просто повиснут в процессах. Ограничений я никаких не применял - пользователь должен сам понимать что делает и чем это грозит.

### Home project: https://github.com/jager1win/gucli

Возможные проблемы:
- Для работы приложения требуется GTK>=3
- Уведомления: Для отображения системных уведомлений приложение использует утилиту notify-send. В некоторых дистрибутивах (например, в минимальных установках Debian) она может отсутствовать. Для исправления установите пакет libnotify-bin.


⚠ Warning: Not a CLI replacement!

Config file is located at `/home/$USER/.config/gucli/commands.toml`
Default settings (after install or `Reset to default`) look like:
# params: command=string(with args), active=bool(default true), sn=bool(default=true)
[[command]]
name = "hostname -A"
active = true
sn = true
The first line describes the structure - add commands accordingly
Editing settings can be done either in GUI or text editor.
After manual editing, restart the application.

SN (system notification) will always show error messages, even when disabled
Command ID in tray menu is the command string itself - for explicit selection and error prevention.
Use trailing `&` for complex commands (background execution)
In tray menu it may appear as `_` due to system formatting
Linux Command Types:
Regular (e.g., `ls -la /home/$USER/Pictures`): Can be converted to a string and output shown in notification
Long-running (e.g., `watch`): Cannot be converted to a string
No-output (e.g., `sleep`): Notifications can be disabled
Linux Command Types:
Regular (e.g., `ls -la /home/$USER/Pictures`): Can be converted to a string and output shown in notification
Long-running (e.g., `watch`): Cannot be converted to a string
No-output (e.g., `sleep`): Notifications can be disabled

зависимости, которые не стал добавлять в tauri.conf.json:
    "linux": {
      "deb": {
        "files": {},
        "depends": [
          "libc6 (>= 2.35)",
          "libgtk-3-0 (>= 3.24)",
          "libayatana-appindicator-glib | libayatana-appindicator3-1 | libappindicator3-1 | libkf5notifications5"
        ]
      },
      "rpm": {
        "epoch": 0,
        "files": {},
        "depends": [
          "glibc >= 2.35",
          "gtk3 >= 3.24",
          "ayatana-appindicator3 | libappindicator-gtk3 | kf5-knotifications"
        ]
      }
    }

Ограничения в программе:
  - Команды выполняются с ограничением времени в 500 мс. Используйте только быстрые команды, возвращающие результат мгновенно.
  - Длина текста в уведомлении ограничена 200 символами.

## Security and Responsibility

Gucli executes commands with your user privileges. You are responsible for:

- Choosing appropriate commands
- Understanding what each command does  
- Avoiding destructive operations
- Not running background processes that may consume resources

The application includes:
- ⏱️ 500ms timeout protection
- 📋 Output truncation for notifications  
- 🛡️ Basic sanity checks