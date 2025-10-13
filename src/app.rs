use leptos::prelude::*;
use leptos::task::spawn_local;
use serde::{Deserialize, Serialize};
use serde_wasm_bindgen::{from_value, to_value};
use wasm_bindgen::prelude::*;
use chrono::Local;
use leptos::ev::KeyboardEvent;
use web_sys::window;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Command {
    pub id: String,
    pub shell: String,
    pub command: String,
    pub icon: String,
    pub sn: bool,
}

impl Command {
    pub fn new(id: String) -> Self {
        Command {
            id,
            shell: String::from("sh"),
            command: String::from("new"),
            icon: String::from(""),
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

#[derive(Serialize)]
struct OpenFile<'a> {
    name: &'a str,
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

static SETTINGS_HELP: &str = 
"<ul>
    <li>Program executes shell commands that return either nothing or string-convertible output</li>
    <li>Command settings can be edited in <code>/home/$USER/.config/gucli/commands.toml</code> without opening this window (restart required)</li>
    <li>Errors and results are logged to <code>/home/$USER/.config/gucli/gucli.log</code> (100 line limit, no rotation needed)</li>
    <li>Interactive commands or commands with continuous output are not recommended</li>
    <li>Command execution timeout: 500ms (add <code>&</code> to bypass)</li>
    <li>Notification text limited to 200 characters (long messages may freeze GTK)</li>
    <li>Commands in tray menu display as-is - use <code>aliases</code> or shell scripts for long entries</li>
    <li>icon: up to 8 UTF-8 characters (emoji, short text, or empty)</li>
    <li>sn (show notification): show system notification (default: true). Error notifications always show</li>
    <li>Always test commands first, even if you know what you're doing</li>
</ul>";

static SEARCH_HELP: &str = 
"<ul>
    <li>If you need an exact reference with specific attributes, write it in full</li>
    <li>Otherwise, the program will iterate through this set:<br />
    <code>[ --help, -h, --usage, help, -help, -?, --longhelp, --long-help, --help-all, info]</code><br />
    then check the man pages, and return the first matching option found.</li>
    <li>To prevent the window from freezing, the maximum result length is limited to 30,000 characters</li>
</ul>";

#[component]
pub fn App() -> impl IntoView {
    let (commands0, set_commands0) = signal(Vec::<Command>::new());
    let (commands, set_commands) = signal(Vec::<Command>::new());
    let (is_maximized, set_is_maximized) = signal("max0");
    let (autostart, set_autostart) = signal(false);
    let (status, set_status) = signal(String::from(""));

    let reset = RwSignal::new(false);
    let active_tab = RwSignal::new(0);
    let unsaved_changes = RwSignal::new("");
    let highlight = RwSignal::new(false);
    let ttime = RwSignal::new(String::from(""));

    // Theme switcher
        let mut initial_theme = "light".to_string();
        let local_storage_theme: Option<String> = window()//get value from localStorage 
            .and_then(|w| w.local_storage().ok())
            .and_then(|s| s?.get("theme").ok())
            .flatten();
        // 2. get system theme IF localStorage None or empty
        let prefer = match local_storage_theme.as_deref() {
            Some(theme) if !theme.is_empty() => None,
            _ => {
                window()
                    .and_then(|w| w.match_media("(prefers-color-scheme: dark)").ok())
                    .flatten()
                    .map(|mql| if mql.matches() { "dark" } else { "light" })
            }
        };
        // 3. set theme
        if let Some(theme) = local_storage_theme {
            if !theme.is_empty() {
                initial_theme = theme;
            }
        } else if let Some(pref) = prefer {
            initial_theme = pref.to_string();
        }
        let (theme, set_theme) = signal(initial_theme);
        let toggle_theme = move || {
            set_theme.update(|t| {
                *t = match t.as_str() {
                    "light" => "dark".into(),
                    "dark" => "a11y".into(),
                    "a11y" => "light".into(),
                    _ => "light".into(),
                };
            });
        };

        
    //+ init commands on open window
    spawn_local(async move {
        let js_value = invoke_without_args("get_commands").await;
        let res: Result<Vec<Command>, String> =
            from_value(js_value).map_err(|e| format!("deserialize failed: {e}"));
        log::debug!("load: {:?}", &res);
        match res {
            Ok(new_commands) => {set_commands.set(new_commands.clone());set_commands0.set(new_commands);},
            Err(e) => set_status.set(e),
        }
    });

    //+ Save (check for uniqueness/non-emptiness of names and, if everything is ok, write it to commands & save to commands.toml)
    let save = move |buf: Vec<Command>| {
        // Check "name" - not empty & unique
        let mut names = std::collections::HashSet::new();
        for cmd in &buf {
            if cmd.command.trim().is_empty() {
                set_status.set("Err( Field `command` cannot be empty )".to_string());
                return;
            }
            if !names.insert(cmd.command.clone()) {
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

    //+ reset commands.toml to default
    let reset_commands = move || {
        if !reset.get(){
            reset.set(true);
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
        buf.push(Command::new(gen_id()));
        set_commands.update(move |b| *b = buf.clone());
        set_status.set("Warning( Specify the command and its parameters and test it )".to_string());
    };

    //+ Delete a command by index (+ auto-save)
    let delete_command = move |index: usize| {
        let mut buf = commands.get();
        if index < buf.len() {
            buf.remove(index);
            set_commands.update(move |b| *b = buf.clone());
            set_status.set("Ok( Command deleted )".to_string());
        }
    };

    let run_test = move |cmd: Command| {
        log::debug!("Testing command: {:?}", &cmd);
        if cmd.command.trim().is_empty() {
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
        
        if ctrl == "close" {
            let has_unsaved_changes = commands0.get() != commands.get();
            let warn = "Warning( Are there unsaved changes, really quit? )".to_string();
            if has_unsaved_changes && !status.get().starts_with(&warn) {
                set_status.set(warn);
                return;
            };
        }

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
            // Check current status for autostart
            let status_js = invoke_without_args("autostart_toggle").await;
            let _current_status = from_value::<Result<String, String>>(status_js)
                .unwrap_or(Err("Err( Autostart status unknown )".to_string()));
            autostart_status();
            set_status.set("Ok( Autostart updated )".to_string());
        });
    };
 
    //+ move command in vec - up & down id
    let move_command = move |up: bool,n:usize| {
        let mut buf = commands.get();
        let dir = if up { n-1 } else { n+1 };
        buf.swap(dir, n);
        set_commands.set(buf);
        set_status.set("Ok( Order updated )".to_string());
    };

    let set_shell = move |n:usize| {
        let mut buf = commands.get();
        let shells = ["sh", "bash", "zsh", "fish"];
        let cur = buf[n].shell.clone();
        let idx = shells
                    .iter()
                    .position(|s| s == &cur.as_str())
                    .unwrap();
        let new = shells[(idx + 1)  % shells.len()].to_string();
        buf[n].shell = new;

        set_commands.set(buf);
    };

    //+ Обработчик глобальных горячих клавиш
    let handle_global_keydown = move |ev: KeyboardEvent| {
        match &ev.key()[..] {
            "F1" => active_tab.set(0),
            "F2" => active_tab.set(1),
            "F3" => active_tab.set(2),
            "F4" => toggle_theme(),
            "Escape" => ctrl_window("close"),
            "F11" => ctrl_window(if is_maximized.get() == "max1" { "max0" } else { "max1" }),
            _ => {}
        }
    };

    let open_file = move |file| {
        spawn_local(async move {
            let args = to_value(&OpenFile {name: file}).unwrap();
            let _js_value = invoke("open_file", args).await;
        });
    };

    // monitored status changes update the time of the last operation
    Effect::new(move |_| {
        status.track();
        ttime.set(Local::now().format("%Y-%m-%d %H:%M:%S.%3f").to_string());
        highlight.set(true);
        set_timeout(move || highlight.set(false), std::time::Duration::from_millis(300));
        log::debug!("effect 1 status: {:?}", status.get());
    });

    // compare commands0 != commands & adds a line about unsaved changes to the unsaved_changes
    Effect::new(move |_| {
        commands.track();
        if commands0.get() != commands.get(){
            unsaved_changes.set("Unsaved changes")
        }else{ unsaved_changes.set("")}
    });

    // for toggle_theme
    Effect::new(move |_| {
        if let Some(window) = window() {
            if let Some(html_el) = window.document().and_then(|d| d.document_element()) {
                let current_theme = theme.get();
                let _ = html_el.set_attribute("data-theme", &current_theme);
            }

            if let Ok(Some(storage)) = window.local_storage() {
                let _ = storage.set("theme", &theme.get());
            }
        }
    });

    view! {
        <div
            data-tauri-drag-region
            class="titlebar"
            role="application"
            aria-label="Gucli Application"
            on:keydown=handle_global_keydown
        >
            <div class="titlebar-title">
                "$_"
                <button
                    on:click=move |_| toggle_theme()
                    class="theme-switcher"
                    aria-label=move || format!("Switch theme. Current: {}", theme.get())
                    title=move || format!("Current theme: {}", theme.get())
                >
                    {move || match theme.get().as_str() {
                        "light" => "🌞[F4]",
                        "dark" => "🌙[F4]",
                        "a11y" => "♾️[F4]",
                        _ => "🌞[F4]",
                    }}
                </button>
            </div>
            <button
                class:active=move || active_tab.get() == 0
                class="tabs-header"
                on:click=move |_| active_tab.set(0)
                autofocus=move || active_tab.get() == 0
                aria-keyshortcuts="F1"
            >
                "Commands [F1]"
            </button>
            <button
                class:active=move || active_tab.get() == 1
                class="tabs-header"
                on:click=move |_| active_tab.set(1)
                autofocus=move || active_tab.get() == 1
                aria-keyshortcuts="F2"
            >
                "Find help || man [F2]"
            </button>
            <button
                class:active=move || active_tab.get() == 2
                class="tabs-header"
                on:click=move |_| active_tab.set(2)
                autofocus=move || active_tab.get() == 2
                aria-keyshortcuts="F3"
            >
                "About [F3]"
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

        <main class="container" role="main">
            <div hidden=move || active_tab.get() != 0>
                <div class="topline">
                    <button
                        on:click=move |_| toggle_autostart()
                        class=move || if autostart.get() { "ok-bg" } else { "" }
                    >
                        {move || if autostart.get() { "Autostart: ON" } else { "Autostart: OFF" }}
                    </button>
                    <button on:click=move |_| open_file("log") class="warn-bg">
                        "Open Log"
                    </button>
                    <button on:click=move |_| open_file("commands") class="ok-bg">
                        "Open Config"
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
                        <span class="ttime">{move || ttime.get()}</span>
                    </div>
                    <div role="status" aria-live="polite" aria-atomic="true">
                        <span
                            class="status-block"
                            class:ok-text=move || status.get().starts_with("Ok")
                            class:err-text=move || status.get().starts_with("Er")
                            class:warn-text=move || status.get().starts_with("Warn")
                            class:highlight=move || highlight.get()
                            id=move || ttime.get().to_string()
                            data-update=move || ttime.get().to_string()
                            inner_html=move || status.get()
                        ></span>
                    </div>
                </div>

                <div class="commands form">
                    <div class="row head">
                        <span>"#"</span>
                        <span>"shell"</span>
                        <span>"command"</span>
                        <span>"icon"</span>
                        <span>"sn"</span>
                        <span>"delete"</span>
                        <span>"test"</span>
                    </div>

                    <ForEnumerate
                        each=move || commands.get()
                        key=|command| command.id.clone()
                        let(i,
                        command)
                    >
                        <div class="row">
                            <div class="order">
                                <button
                                    on:click=move |_| move_command(true, i.get())
                                    prop:disabled=move || i.get() == 0
                                    aria-label=move || format!("Move command '{}' up", commands.get()[i.get()].command.clone())
                                >
                                    "↑"
                                </button>
                                <span class="nn">{i}</span>
                                <button
                                    on:click=move |_| move_command(false, i.get())
                                    prop:disabled=move || i.get() == commands.get().len() - 1
                                    aria-label=move || format!("Move command '{}' down", commands.get()[i.get()].command.clone())
                                >
                                    "↓"
                                </button>
                            </div>
                            <button
                                class="shell-switch"
                                on:click=move |_| set_shell(i.get())
                                aria-live="polite"
                                aria-atomic="true"
                                aria-label=move || {
                                    format!(
                                        "Switch shell. Current: {}",
                                        commands.get()[i.get()].clone().shell,
                                    )
                                }
                            >
                                "▶|"
                                <span>{move || commands.get()[i.get()].clone().shell}</span>
                            </button>
                            <input
                                class="coma"
                                type="text"
                                placeholder="Danger zone! Verify commands before adding..."
                                value=move || command.command.clone()
                                on:input=move |ev| {
                                    let value = event_target_value(&ev);
                                    set_commands
                                        .update(|cmds| {
                                            cmds[i.get()].command = value;
                                        });
                                }
                                aria-description="Warning: Commands execute with user permissions. Test commands first."
                            />
                            <input
                                class="iicon"
                                type="text"
                                placeholder="8 chars"
                                size="8"
                                maxlength="8"
                                value=move || command.icon.clone()
                                on:input=move |ev| {
                                    let value = event_target_value(&ev);
                                    set_commands
                                        .update(|cmds| {
                                            cmds[i.get()].icon = value;
                                        });
                                }
                            />
                            <label class="chb">
                                <input
                                    type="checkbox"
                                    checked=move || commands.get()[i.get()].clone().sn
                                    on:change=move |ev| {
                                        let checked = event_target_checked(&ev);
                                        set_commands
                                            .update(|cmds| {
                                                cmds[i.get()].sn = checked;
                                            });
                                    }
                                    on:keydown=move |ev: KeyboardEvent| {
                                        if ev.key() == "Enter" || ev.key() == " " {
                                            ev.prevent_default();
                                            set_commands
                                                .update(|cmds| {
                                                    cmds[i.get()].sn = !cmds[i.get()].sn;
                                                });
                                        }
                                    }
                                    aria-label=move || {
                                        format!(
                                            "Show system notification for command '{}'. Currently: {}",
                                            commands.get()[i.get()].command.clone(),
                                            if commands.get()[i.get()].clone().sn {
                                                "on"
                                            } else {
                                                "off"
                                            },
                                        )
                                    }
                                />
                            </label>
                            <div>
                                <button on:click=move |_| delete_command(i.get()) class="err-bg">
                                    "Delete"
                                </button>
                            </div>
                            <div>
                                <button
                                    on:click=move |_| run_test(commands.get()[i.get()].clone())
                                    class="warn-bg"
                                >
                                    "Run test"
                                </button>
                            </div>
                        </div>
                    </ForEnumerate>

                    <div class="buttons tc">
                        <div>
                            <button class="ok-bg" on:click=move |_| add_command()>
                                "Add command"
                            </button>
                        </div>
                        <span class="warn-text tc" inner_html=unsaved_changes></span>
                        <div>
                            <button class="ok-bg" on:click=move |_| save(commands.get())>
                                "Save & Restart"
                            </button>
                        </div>
                    </div>
                </div>

                <details>
                    <summary>Help</summary>
                    <div class="text-bg" inner_html=SETTINGS_HELP></div>
                </details>

            </div>
            <div hidden=move || active_tab.get() != 1>
                <ManSearch />
            </div>
            <div hidden=move || active_tab.get() != 2>
                <About />
            </div>
        </main>
    }
}

#[component]
pub fn ManSearch() -> impl IntoView {
    use leptos::{ev::SubmitEvent};
    let (man, set_man) = signal(String::new());
    let (input_value, set_input_value) = signal("".to_string());

    let on_submit = move |ev: SubmitEvent| {
        ev.prevent_default();
        let trimmed_value = input_value.get().trim().to_string();
        set_input_value.set(trimmed_value.clone());

        if trimmed_value.is_empty() {
            set_man.set("".to_string());
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

    view! {
        <div role="search" aria-label="Command help search">
            <h4 class="tc" id="man-search-title">
                "Get console help with command: man or built-in --help"
            </h4>

            <form on:submit=on_submit class="man_form" role="search">
                <input
                    type="text"
                    placeholder="e.g. `id` or `mpg123 -?`"
                    size=40
                    prop:value=move || input_value.get()
                    on:input=move |ev| set_input_value.set(event_target_value(&ev))
                    aria-labelledby="man-search-title"
                    aria-describedby="search-help"
                />
                <button type="submit" class="ok-bg" aria-label="Run search">
                    "Search"
                </button>
            </form>

            <pre
                class="man_result"
                inner_html=move || man.get()
                hidden=move || { man.get().is_empty() }
                aria-live="polite"
                aria-atomic="true"
            ></pre>

            <details id="search-help">
                <summary>Help</summary>
                <div class="text-bg" inner_html=SEARCH_HELP></div>
            </details>
        </div>
    }
}

#[component]
pub fn About() -> impl IntoView { 
    let (info, set_info) = signal(Vec::<String>::new());
    Effect::new(move |_| {
        spawn_local(async move {
            let js_value = invoke_without_args("get_app_info").await;
            if let Ok(vec_info) = from_value(js_value) {
                set_info.set(vec_info);
        }
    });
    });
    view! {
        <div class="help tc">
            <p class="text-bg">
                <h4>"Your personal command center in the system tray"</h4>

                <p>
                    "Gucli (from GUI + CLI) is a simple system tray application"<br />
                    "that turns your frequent console commands into menu items for one-click launching."
                </p>
                <p class="err-text">"⚠ Warning: Not a CLI replacement!"</p>

                {move || {
                    info.get()
                        .into_iter()
                        .map(|n| {
                            if n.starts_with("http") {
                                view! {
                                    <p>
                                        "Homepage: "<a href=n target="_blank">
                                            {n.clone()}
                                        </a>
                                    </p>
                                }
                                    .into_any()
                            } else {
                                view! { <p>{n}</p> }.into_any()
                            }
                        })
                        .collect_view()
                }}

                <p>
                    "For information on compatibility, dependencies, or to report issues, please visit the homepage."
                </p>
                <p>"♾️ Accessibility"</p>
            </p>
        </div>
    }
}

fn gen_id() -> String {
    Local::now().timestamp_nanos_opt()
        .unwrap_or(0)
        .to_string()
}

#[component]
pub fn ShellSwitch() -> impl IntoView {
    let shells = vec!["sh", "bash", "zsh", "fish"];
    let (current_index, set_current_index) = signal(0);

    let next_shell = {
        // захватываем shells и set_current_index
        let shells = shells.clone(); // если хочется ownership в замыкании
        move |_| {
            set_current_index.update(|idx| *idx = (*idx + 1) % shells.len());
        }
    };

    view! {
        <div class="shell-switch">
            <span class="current-shell">{shells[current_index.get()]}</span>
            <button class="shell-button" on:click=next_shell>
                "▶"
            </button>
        </div>
    }
}