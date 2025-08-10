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
    pub sn: bool,
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
            sn: true,
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

#[derive(Serialize)]
struct CtrlWindow<'a> {
    action: &'a str,
}

#[derive(Serialize)]
struct ManHelp {
    cmd: String,
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
    let (active_tab, set_active_tab) = signal(0);
    let (commands, set_commands) = signal(Vec::<Command>::new());
    let (status, set_status) = signal(String::from("Ok( Configuration loaded )"));
    let (ttime, set_ttime) = signal(String::from(""));
    let (highlight, set_highlight) = signal(false);
    let (autostart, set_autostart) = signal(false);
    let (is_maximized, set_is_maximized) = signal("max0");
    let (reset, set_reset) = signal(false);
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
        if !reset.get(){
            set_reset.set(true);
            set_status.set("Warn( Click again to reset )".to_string());
        }else{
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
        }
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

    let ctrl_window = move |ctrl| {
        if ctrl == "max0"{
            set_is_maximized.set("max1");
        }else {set_is_maximized.set("max0");}

        spawn_local(async move {
            let args = to_value(&CtrlWindow { action: ctrl }).unwrap();
            let _ = invoke("ctrl_window",args).await;
        });
    };

    let autostart_status = move || {
        spawn_local(async move {
            let js = invoke_without_args("autostart_status").await;
            if let Ok(enabled) = from_value::<bool>(js) {
                set_autostart.set(enabled);
            }else{
                set_autostart.set(false);
            }
        });
    };
    autostart_status();

    let toggle_autostart = move || {
        spawn_local(async move {
            // Проверяем текущий статус
            let status_js = invoke_without_args("autostart_enable").await;
            let _current_status = from_value::<Result<String, String>>(status_js)
                .unwrap_or(Err("Err( Autostart status unknown )".to_string()));
            autostart_status();
            set_status.set("Ok( Autostart updated )".to_string());
        });
    };

    Effect::new(move |_| {
        status.track(); // Tracking status changes
        set_ttime.set(Local::now().format("%Y-%m-%d %H:%M:%S.%3f").to_string()); // set new time
        set_highlight.set(true);
        set_timeout(move || set_highlight.set(false), std::time::Duration::from_secs(2));
    });

    view! {
        <div data-tauri-drag-region class="titlebar">
            <div class="titlebar-title">"$_"</div>
            <button
                class:active=move || active_tab.get() == 0
                class="tabs-header"
                on:click=move |_| set_active_tab.set(0)
            >
                "Settings"
            </button>
            <button
                class:active=move || active_tab.get() == 1
                class="tabs-header"
                on:click=move |_| set_active_tab.set(1)
            >
                "Man pages"
            </button>
            <button
                class:active=move || active_tab.get() == 2
                class="tabs-header"
                on:click=move |_| set_active_tab.set(2)
            >
                "Help"
            </button>

            <div class="titlebar-controls">
                <button on:click=move |_| ctrl_window("min") id="titlebar-minimize">
                    "─"
                </button>
                <button on:click=move |_| ctrl_window(is_maximized.get()) id="titlebar-maximize">
                    {move || if is_maximized.get() == "max1" { "❐" } else { "□" }}
                </button>
                <button on:click=move |_| ctrl_window("close") id="titlebar-close">
                    "x"
                </button>
            </div>
        </div>

        <main class="container">

            <Show when=move || active_tab.get() == 0>
                <div class="topline">
                    <button
                        on:click=move |_| toggle_autostart()
                        class=move || if autostart.get() { "ok-bg" } else { "" }
                    >
                        {move || if autostart.get() { "Autostart: ON" } else { "Autostart: OFF" }}
                    </button>
                    <button on:click=move |_| reset_commands() class="err-bg">
                        {move || match reset.get() {
                            true => "Really reset?",
                            false => "Reset & Restart",
                        }}
                    </button>
                </div>

                <div class="status">
                    <div>
                        <span>"STATUS"</span>
                        <br />
                        <span>"count: " {move || commands.get().len()}</span>
                    </div>
                    <div>
                        <span>{move || ttime.get()}</span>
                    </div>
                    <div>
                        <span
                            class="status-block text-bg"
                            class:highlight=move || highlight.get()
                            id=move || ttime.get().to_string()
                            data-update=move || ttime.get().to_string()
                            inner_html=move || status.get()
                        ></span>
                    </div>
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
                            let (sysn, set_sysn) = signal(cmd.sn);
                            let sync_to_buffer = move || {
                                let mut buf = commands.get();
                                if let Some(slot) = buf.get_mut(i) {
                                    slot.name = name.get();
                                    slot.active = active.get();
                                    slot.sn = sysn.get();
                                }
                                set_commands.update(move |b| *b = buf.clone());
                            };
                            // When any local signal changes, we push the changes
                            view! {
                                <div class="row">
                                    <input
                                        type="text"
                                        placeholder="Danger zone! Verify commands before adding..."
                                        value=move || name.get()
                                        on:change=move |ev| {
                                            set_name.set(event_target_value(&ev));
                                            sync_to_buffer();
                                        }
                                    />
                                    <div class="chb">
                                        <input
                                            type="checkbox"
                                            checked=move || active.get()
                                            on:change=move |ev| {
                                                set_active.set(event_target_checked(&ev));
                                                sync_to_buffer();
                                            }
                                        />
                                    </div>
                                    <div class="chb">
                                        <input
                                            type="checkbox"
                                            checked=move || sysn.get()
                                            on:change=move |ev| {
                                                set_sysn.set(event_target_checked(&ev));
                                                sync_to_buffer();
                                            }
                                        />
                                    </div>
                                    <div>
                                        <button on:click=move |_| delete_command(i) class="err-bg">
                                            "Delete"
                                        </button>
                                    </div>
                                    <div>
                                        <button
                                            on:click=move |_| run_test(commands.get()[i].clone())
                                            class="warn-bg"
                                        >
                                            "Run test"
                                        </button>
                                    </div>
                                </div>
                            }
                        }
                    />
                </div>
                <div class="buttons bb">
                    <button class="ok-bg" on:click=move |_| add_command()>
                        "Add row"
                    </button>
                    <button class="ok-bg" on:click=move |_| save()>
                        "Save & Restart"
                    </button>
                </div>

            </Show>
            <Show when=move || active_tab.get() == 1>
                <div class="tab-pane">
                    <ManSearch />
                </div>
            </Show>
            <Show when=move || active_tab.get() == 2>
                <div class="tab-pane">
                    <Help />
                </div>
            </Show>

            <div></div>

        </main>
    }
}

#[component]
pub fn ManSearch() -> impl IntoView {
    use leptos::{ev::SubmitEvent};
    static HELP: &str = "Можно в input написать просто название команды - будет произведен поиск по `man` && `--help`. 
Если нужна справка со специфическими ключами [--longhelp, --help-all] - введите нужную команду полностью";
    let (man, set_man) = signal(HELP.to_string());
    let (input_value, set_input_value) = signal("".to_string());

    let on_submit = move |ev: SubmitEvent| {
        ev.prevent_default();
        let trimmed_value = input_value.get().trim().to_string();
        set_input_value.set(trimmed_value.clone());

        if trimmed_value.is_empty() {
            set_man.set(HELP.to_string());
        } else {
            spawn_local(async move {
                let args = to_value(&ManHelp {cmd: trimmed_value}).unwrap();
                let js_value = invoke("get_man", args).await;
                let result: Result<String, String> = from_value(js_value).map_err(|e| format!("man pages get failed: {e}"));
                match result {
                    Ok(man) => set_man.set(man),
                    Err(e) => set_man.set(e),
                }
            });
        }
    };

    Effect::new(move |_| {
        if input_value.get().is_empty() {
            set_man.set(HELP.to_string());
        }
    });

    view! {
        <h4 class="tc">"Get console help with command: man or built-in --help"</h4>

        <form on:submit=on_submit class="man_form">
            <input
                type="text"
                placeholder="e.g. `whoami`"
                size=40
                prop:value=move || input_value.get()
                on:input=move |ev| set_input_value.set(event_target_value(&ev))
            />
            <button type="submit" class="ok-bg">
                "Find"
            </button>
        </form>

        <pre class="f8 text-bg" inner_html=move || man.get()></pre>
    }
}

#[component]
pub fn Help() -> impl IntoView { 
    view! {
        <div class="help text-bg">
            <p class="tc">
                "GUCLI Primarily designed for external installed programs, but you can run anything — even `rm -rf /xxx`"
                <br /> "⚠ Warning: Not a CLI replacement!"<br />
                "Always test commands in terminal first, only add to GUCLI when 110% certain."<br />
                "Your future self will either thank you or curse you 😈"
            </p>
            <ul>
                <li>
                    <b class="stt">"Config file"</b>
                    " is located at "
                    <b class="stt">"`/home/$USER/.config/gucli/settings.toml`"</b>
                    <br />
                    "Default settings (after install or `Reset to default`) look like:"
                    <br />
                    <pre>
                        "# params: command=string(with args), active=bool(default true), sn=bool(default=true)"
                        <br /> "[[command]]"<br /> "name = \"hostname -A\""<br /> "active = true"
                        <br /> "sn = true"<br />
                    </pre>
                    "The first line describes the structure - add commands accordingly"
                </li>
                <li>
                    <b class="stt">"Editing settings"</b>
                    " can be done either in GUI or text editor."
                    <br />
                    "After manual editing, restart the application."
                    <br />
                    "You can add command: `xdg-open /home/$USER/.config/gucli/settings.toml`"
                    <br />
                    "& replace `xdg-open` with your preferred text editor"
                </li>
                <li>
                    <b class="stt">"SN (system notification)"</b>
                    " will always show error messages, even when disabled"
                </li>
                <li>
                    "Command ID in tray menu is the command string itself - for explicit selection and error prevention."
                </li>
                <li>
                    <b class="stt">"Use trailing `&`"</b>
                    " for complex commands (background execution)"
                    <br />
                    "In tray menu it may appear as `_` due to system formatting"
                    <br />
                </li>
                <li>
                    <strong>"Linux Command Types:"</strong>
                    <ul>
                        <li>
                            <b class="stt">"Regular"</b>
                            " (e.g., `ls -la /home/$USER/Pictures`): Can be converted to a string and output shown in notification"
                        </li>
                        <li>
                            <b class="stt">"Long-running"</b>
                            " (e.g., `watch`): Cannot be converted to a string"
                        </li>
                        <li>
                            <b class="stt">"No-output"</b>
                            " (e.g., `sleep`): Notifications can be disabled"
                        </li>
                    </ul>
                </li>
            </ul>
        </div>
    }
}