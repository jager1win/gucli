use leptos::prelude::*;
use leptos::task::spawn_local;
use serde::{Deserialize, Serialize};
use serde_wasm_bindgen::{from_value, to_value};
use wasm_bindgen::prelude::*;
use chrono::Local;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Command {
    pub name: String,
    pub active: bool,
    pub system_notification: bool,
}

#[derive(Default, Debug, Serialize, Deserialize)]
pub struct CommandsConfig {
    #[serde(rename = "command")]
    pub commands: Vec<Command>,
}

impl Default for Command {
    fn default() -> Self {
        Command {
            name: String::new(),
            active: true,
            system_notification: true,
        }
    }
}

#[derive(Serialize)]
struct RunTestArgs {
    cmd: Command,
}

#[derive(Serialize)]
struct SaveBackArgs {
    commands: Vec<Command>,
}

#[wasm_bindgen]
extern "C" {
    // invoke without arguments
    #[wasm_bindgen(js_namespace = ["window", "__TAURI__", "core"], js_name = invoke)]
    async fn invoke_without_args(cmd: &str) -> JsValue;
    // invoke with arguments (default)
    #[wasm_bindgen(js_namespace = ["window", "__TAURI__", "core"])]
    async fn invoke(cmd: &str, args: JsValue) -> JsValue;
}

#[component]
pub fn App() -> impl IntoView {
    let (commands, set_commands) = signal(Vec::<Command>::new());
    let (status, set_status) = signal(String::from("Ok( Loading configuration )"));
    let (ttime, set_ttime) = signal(String::from(""));
    let (highlight, set_highlight) = signal(false);
    let (is_autostart, set_autostart) = signal(false);
    //+ init commands on open window
    let load = move || {spawn_local(async move {
        let js_value = invoke_without_args("get_commands").await;
        let res: Result<Vec<Command>, String> =
            from_value(js_value).map_err(|e| format!("deserialize failed: {e}"));
        match res {
            Ok(new_commands) => set_commands.set(new_commands),
            Err(e) => set_status.set(e),
        }
    })};
    load();

    //+ Save (check for uniqueness/non-emptiness of names and, if everything is ok, write it to commands & save to settings.toml)
    let save = move || {
        let buf = commands.get();
        // Check "name" - not empty & unique
        let mut names = std::collections::HashSet::new();
        for cmd in &buf {
            if cmd.name.trim().is_empty() {
                set_status.set("Err( Field `command` cannot be empty )".to_string());
                return;
            }
            if !names.insert(cmd.name.clone()) {
                set_status.set("Err( Field `command` must be unique )".to_string());
                return;
            }
        }
        // if ok -> save & restart
        set_commands.update(move |c| *c = buf.clone());
        log::debug!("save->commands: {:?}", commands.get());
        spawn_local(async move {
            let commands = commands.get_untracked();
            let args = to_value(&SaveBackArgs { commands }).unwrap();
            let js = invoke("set_commands", args).await;
            let result: Result<String, String> = from_value(js).map_err(|e| format!("deserialize failed: {e}"));
            match result {
                Ok(_) => { set_status.set("Ok( Commands saved )".to_string());}
                Err(e) => set_status.set(format!("Err( Save failed: {e} )")),
            }
            let _ = invoke("request_restart", JsValue::NULL).await;
        });
    };

    //+ reset settings.toml to default
    let reset_commands = move || {
        spawn_local(async move {
            let js = invoke_without_args("reset_commands").await;
            let result: Result<String, String> = from_value(js).map_err(|e| format!("deserialize failed: {e}"));
            match result {
                Ok(_) => { 
                    set_status.set("Ok( Settings reset to default )".to_string());
                    let _ = invoke("request_restart", JsValue::NULL).await;
                }
                Err(e) => set_status.set(format!("Err( Reset failed: {e}")),
            }
        });
    };

    //+ Add a new row with default values
    let add_command = move || {
        let mut buf = commands.get();
        buf.push(Command::default());
        set_commands.update(move |b| *b = buf.clone());
        set_status.set("Warning( Specify the command and its parameters and test it )".to_string());
    };

    //+ Delete a command by index (+ auto-save)
    let delete_command = move |index: usize| {
        let mut buf = commands.get();
        if index < buf.len() {
            buf.remove(index);
            set_commands.update(move |b| *b = buf.clone());
            
            set_status.set("Warning( After editing the list, save the settings by clicking Save&Restart )".to_string());
        }
    };

    let run_test = move |cmd: Command| {
        log::info!("Testing command: {:?}", &cmd);
        if cmd.name.trim().is_empty() {
            set_status.set("Err( Field `command` cannot be empty )".to_string());
            return;
        }
        spawn_local(async move {
            let args = to_value(&RunTestArgs { cmd }).unwrap();
            let js = invoke("run_test", args).await;
            match from_value::<String>(js) {
                Ok(success_msg) => set_status.set(success_msg),
                Err(e) => {
                    set_status.set(format!("Err( Command execution failed {e:?} )"));
                }
            }
        });
    };

    // autostart check
    let check_autostart = move || {
        spawn_local(async move {
            let js = invoke_without_args("is_autostart_enabled").await;
            if let Ok(enabled) = from_value::<bool>(js) {
                set_autostart.set(enabled);
            }
        });
    };
    check_autostart();

    let toggle_autostart = move || {
        spawn_local(async move {
            // Проверяем текущий статус
            let status_js = invoke_without_args("is_autostart_enabled").await;
            let current_status = from_value::<Result<String, String>>(status_js)
                .unwrap_or(Err("Err( Autostart status unknown )".to_string()));

            // Определяем действие
            let action = match current_status {
                Ok(msg) if msg.contains("enabled") => "disable_autostart",
                _ => "enable_autostart",
            };

            // Выполняем действие
            let result_js = invoke_without_args(action).await;
            let result = from_value::<Result<String, String>>(result_js)
                .unwrap_or(Err("Err( Autostart action failed )".to_string()));

            // Обновляем статус
            match result {
                Ok(msg) => set_status.set(format!("Ok( Autostart status: {msg} )")),
                Err(e) => set_status.set(format!("Err( {e} )")),
            }
        });
    };

    Effect::new(move |_| {
        status.track(); // Tracking status changes
        set_ttime.set(Local::now().format("%Y-%m-%d %H:%M:%S.%3f").to_string()); // set new time
        set_highlight.set(true);
        set_timeout(move || set_highlight.set(false), std::time::Duration::from_secs(1));
    });

    view! {
        <main class="container">
            <div class="main_settings gridd topline buttons">
                <button
                    on:click=move |_| toggle_autostart()
                    class=move || if is_autostart.get() { "ok-bg" } else { "" }
                >
                    {move || if is_autostart.get() { "Autostart: ON" } else { "Autostart: OFF" }}
                </button>
                <button on:click=move |_| reset_commands() class="err-bg">"Reset & Restart"</button>
            </div>

            <div class="status">
                <div><span>"STATUS"</span><br /><span>"count: " {move || commands.get().len()}</span></div>
                <div><span>{move || ttime.get()}</span></div>
                <div><span class="status-block" class:highlight=move || highlight.get() id=move || ttime.get().to_string() data-update=move || ttime.get().to_string()>{move || status.get()}</span></div>
            </div>

            <div class="commands form">
                <div class="row head">
                    <span>"command"</span>
                    <span>"active"</span>
                    <span>"SN"</span>
                    <span>"delete"</span>
                    <span>"test"</span>
                </div>

                    <For
                        each=move || commands.get().into_iter().enumerate()
                        key=|(i, _)| *i
                        children=move |(i, cmd)| {
                            let (name, set_name) = signal(cmd.name.clone());
                            let (active, set_active) = signal(cmd.active);
                            let (sysn, set_sysn) = signal(cmd.system_notification);
                            // When any local signal changes, we push the changes
                            let sync_to_buffer = move || {
                                let mut buf = commands.get();
                                if let Some(slot) = buf.get_mut(i) {
                                    slot.name = name.get();
                                    slot.active = active.get();
                                    slot.system_notification = sysn.get();
                                }
                                set_commands.update(move |b| *b = buf.clone());
                            };
                            view! {
                                <div class="row">
                                    <input
                                        type="text"
                                        placeholder="Danger zone! Verify commands before adding..." 
                                        value={ move || name.get()}
                                        on:change=move |ev| {
                                            set_name.set(event_target_value(&ev));
                                            sync_to_buffer();
                                        }
                                    />
                                    <div class="chb">
                                        <input
                                            type="checkbox"
                                            checked={ move || active.get()}
                                            on:change=move |ev| {
                                                set_active.set(event_target_checked(&ev));
                                                sync_to_buffer();
                                            }
                                        />
                                    </div>
                                    <div class="chb">
                                        <input
                                            type="checkbox"
                                            checked={ move || sysn.get()}
                                            on:change=move |ev| {
                                                set_sysn.set(event_target_checked(&ev));
                                                sync_to_buffer();
                                            }
                                        />
                                    </div>
                                    <div><button on:click=move |_| delete_command(i) class="err-bg">"Delete"</button></div>
                                    <div><button on:click=move |_| run_test(commands.get()[i].clone()) class="warn-bg">"Run test"</button></div>
                                </div>
                            }
                        }
                    />
            </div>
            <div class="buttons bb">
                <button class="ok-bg" on:click=move |_| add_command() >"Add row"</button>
                <button class="ok-bg" on:click=move |_| save()>"Save & Restart"</button>
            </div>

            <p class="tc">"GUCLI Warning: Not a CLI replacement!"<br />
              "Always test commands in terminal first, only add to GUCLI when 110% certain."<br />
              "Your future self will either thank you or curse you 😈"
            </p>

            /*<details class="help">
                <summary><span>"Help"</span></summary>
                <div>
                    <ul>
                        <li><b class="stt">"Файл настроек"</b>" программы находится в /home/$USER/.config/gucli/settings.toml"<br />
                        "По умолчанию - при установке и после сброса кнопкой `Reset` - выглядит так:"<br />
                            <pre>"# params: command=string(with args), active=bool(default true), system_notification=bool(default=true)"<br />
                            "[[command]]"<br />
                            "name = \"hostname -A\""<br />
                            "active = true"<br />
                            "system_notification = true"</pre>
                            "В первой строке комментария указана структура, соответствуя которой нужно добавлять команды"
                        </li>
                        <li><b class="stt">"Редактировать настройки"</b>" можно как в интерфейсе, так и в текстовом редакторе."<br />
                            "После редактирования в текстовом редакторе программу нужно перезапустить."<br />
                            "Можно добавить команду <xdg-open /home/$USER/.config/gucli/settings.toml>"<br />
                            "<xdg-open> конечно можно заменить на ваш предпочтительный текстовый редактор"
                        </li>
                        <li><b class="stt">"SN (system notification)"</b>" даже при выключенном состоянии будут выводить сообщения в случае ошибки"</li>
                        <li>"Идентификатором команды в меню трея является сама строка команды - для четкого осознанного выбора, исключающего ошибки."</li>
                        <li><b class="stt">"Используйте ` &`"</b>" для сложных команд, добавляя в конце строки амперсанд"<br />
                            "В меню трея, в зависимости от системы, он может отображаться как `_`"<br />
                        </li>
                        <li>
                            <strong>"Linux Command Types:"</strong>
                            <ul>
                                <li><b class="stt">"Regular"</b>" (e.g., "<b class="stt">"ls -la"</b>"): Output can shown in notification."</li>
                                <li><b class="stt">"Long-running"</b>" (e.g., "<b class="stt">"watch"</b>"): Add "<b class="stt">"&"</b>" to run in background ("<b class="stt">"watch -n 1 "date" &"</b>")."</li>
                                <li><b class="stt">"No-output"</b>" (e.g., "<b class="stt">"sleep"</b>"): Can disable notifications in settings."</li>
                            </ul>
                        </li>
                    </ul>
                </div>
            </details>*/

            <details class="help">
                <summary><span>"Help"</span></summary>
                <div>
                    <ul>
                        <li><b class="stt">"Config file"</b>" is located at "<b class="stt">"`/home/$USER/.config/gucli/settings.toml`"</b><br />
                        "Default settings (after install or `Reset to default`) look like:"<br />
                            <pre>"# params: command=string(with args), active=bool(default true), system_notification=bool(default=true)"<br />
                                "[[command]]"<br />
                                "name = \"hostname -A\""<br />
                                "active = true"<br />
                                "system_notification = true"
                            </pre>
                            "The first line describes the structure - add commands accordingly"
                        </li>
                        <li><b class="stt">"Editing settings"</b>" can be done either in GUI or text editor."<br />
                            "After manual editing, restart the application."<br />
                            "You can add command: `xdg-open /home/$USER/.config/gucli/settings.toml`"<br />
                            "& replace `xdg-open` with your preferred text editor"
                        </li>
                        <li><b class="stt">"SN (system notification)"</b>" will always show error messages, even when disabled"</li>
                        <li>"Command ID in tray menu is the command string itself - for explicit selection and error prevention."</li>
                        <li><b class="stt">"Use trailing `&`"</b>" for complex commands (background execution)"<br />
                            "In tray menu it may appear as `_` due to system formatting"<br />
                        </li>
                        <li>
                            <strong>"Linux Command Types:"</strong>
                            <ul>
                                <li><b class="stt">"Regular"</b>" (e.g., `ls -la`): Can be converted to a string and output shown in notification"</li>
                                <li><b class="stt">"Long-running"</b>" (e.g., `watch`): Add `&` (`watch -n 1 "date" &`)"</li>
                                <li><b class="stt">"No-output"</b>" (e.g., `sleep`): Notifications can be disabled"</li>
                            </ul>
                        </li>
                    </ul>
                </div>
            </details>
            
        </main>
    }
}
