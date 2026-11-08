//! Native GTK4 launcher for the Rust port: readiness, settings pages backed by
//! `settings.json` and `bbport.ini`, and a Launch Game button that starts `run.sh`.

use std::cell::RefCell;
use std::path::PathBuf;
use std::rc::Rc;

use bbport_launcher::ini::{Ini, CHEATS_AND_TWEAKS, EFFECTS, ENHANCEMENTS};
use bbport_launcher::{assets, coop, process, readiness, sfo, trophies, Paths, Settings};
use gtk::prelude::*;
use gtk::{
    gdk, gio, glib, Application, ApplicationWindow, Box as GtkBox, Button, CssProvider, DropDown,
    Entry, FileDialog, Label, Orientation, ScrolledWindow, SpinButton, Stack, StackSidebar, Switch,
};
use gtk4 as gtk;
use serde_json::Value;

const APP_ID: &str = "io.github.bbport.LauncherRs";

const CSS: &str = r#"
window { background-color: #0f1115; color: #e7eaf0; }
headerbar { background-color: #12151b; border-bottom: 1px solid #242a35; }
.sidebar { background-color: #141821; border-right: 1px solid #252b36; }
.sidebar-brand { padding: 22px 18px 8px 18px; }
.sidebar-title { font-size: 20px; font-weight: 800; }
.sidebar-subtitle { color: #8b93a3; font-size: 11px; }
.sidebar list { background-color: transparent; }
.sidebar row { border-radius: 8px; margin: 2px 10px; padding: 8px 10px; }
.sidebar row:selected { background-color: #2b3548; }
.page { background-color: #0f1115; }
.card { background-color: #171b23; border: 1px solid #252b36; border-radius: 14px; padding: 18px; }
.section-title { color: #9aa5b8; font-size: 11px; font-weight: 700; }
.row { border-bottom: 1px solid #222834; padding: 8px 0; }
.note { color: #8b93a3; }
.hero-title { font-size: 30px; font-weight: 800; }
.folder-row { background-color: #1c222c; border-radius: 10px; padding: 10px 12px; }
.check-card { background-color: #1c222c; border-radius: 10px; padding: 12px; }
.status-badge { border-radius: 999px; padding: 3px 9px; font-size: 10px; font-weight: 700; }
.status-ready { background-color: #163b2b; color: #65d49a; }
.status-warning { background-color: #3b3217; color: #e6c15c; }
.status-error { background-color: #3b1a1d; color: #ef7c86; }
.actionbar { background-color: #12151b; border-top: 1px solid #242a35; padding: 10px 14px; }
.status-text { color: #8b93a3; font-size: 11px; }
.trophy-row { background-color: #1c222c; border-radius: 8px; padding: 8px 10px; }
.trophy-row.trophy-locked { opacity: 0.55; }
.trophy-grade { color: #e6c15c; font-weight: 700; }
"#;

const HERO_ART: &[u8] = include_bytes!("launcher/art/hero.png");

fn install_css() {
    let provider = CssProvider::new();
    provider.load_from_data(CSS);
    if let Some(display) = gdk::Display::default() {
        gtk::style_context_add_provider_for_display(
            &display,
            &provider,
            gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
        );
    }
}

const PRESENT_MODES: &[(&str, &str)] = &[
    ("Mailbox", "Mailbox"),
    ("FIFO (VSync)", "Fifo"),
    ("FIFO Relaxed", "FifoRelaxed"),
    ("Immediate", "Immediate"),
];
const FPS_MODES: &[(&str, &str)] = &[
    ("Uncapped (patch)", "uncap"),
    ("30 (PS4)", "30"),
    ("60", "60"),
    ("90", "90"),
];
const UPSCALERS: &[(&str, &str)] = &[
    ("DLSS (RTX)", "dlss"),
    ("FSR 3.1", "fsr3"),
    ("FSR 4 (experimental)", "fsr4"),
    ("FSR 4.1.1 (experimental)", "fsr411"),
    ("TAA (native AA)", "taa"),
    ("Off", "off"),
];
const PRESETS: &[(&str, &str)] = &[
    ("Native AA", "0"),
    ("Quality (x1.5)", "1"),
    ("Balanced (x1.7)", "2"),
    ("Performance (x2)", "3"),
    ("Ultra Performance (x3)", "4"),
];
const OUTPUT_RES: &[(&str, &str)] = &[
    ("1280×720 (Steam Deck)", "1280x720"),
    ("1920×1080", "1920x1080"),
    ("2560×1440", "2560x1440"),
    ("3840×2160", "3840x2160"),
    ("1280×800 (16:10)", "1280x800"),
    ("2560×1080 (21:9)", "2560x1080"),
    ("3440×1440 (21:9)", "3440x1440"),
    ("3840×1600 (ultrawide)", "3840x1600"),
    ("5120×1440 (32:9)", "5120x1440"),
];
const LIVE_RESOLUTION: &[(&str, &str)] = &[
    ("Auto (by GPU)", "auto"),
    ("Off (faster)", "0"),
    ("On", "1"),
];
const DRAW_PIPE: &[(&str, &str)] = &[
    ("Auto (8+ threads)", ""),
    ("On", "1"),
    ("Off (more stable)", "0"),
];
const READBACKS: &[(&str, &str)] = &[("Relaxed (default)", ""), ("Off", "0"), ("Precise", "2")];
const UI_LANGUAGES: &[(&str, &str)] = &[
    ("English", "en"),
    ("Français", "fr"),
    ("Русский", "ru"),
    ("简体中文", "zh"),
    ("한국어", "ko"),
    ("日本語", "ja"),
];
const GAME_LANGUAGES: &[(&str, &str)] = &[
    ("English", "1"),
    ("Russian", "8"),
    ("Japanese", "0"),
    ("French", "2"),
    ("Spanish", "3"),
    ("German", "4"),
    ("Italian", "5"),
];
const CONTROL_SCHEMES: &[(&str, &str)] = &[
    ("Keyboard and mouse", "pc"),
    ("Xbox gamepad", "xbox"),
    ("PlayStation gamepad", "playstation"),
];

const KEYBINDINGS: &[(&str, &str, &str)] = &[
    ("move_forward", "Move Forward", "W"),
    ("move_back", "Move Back", "S"),
    ("move_left", "Move Left", "A"),
    ("move_right", "Move Right", "D"),
    ("walk", "Walk", "Left Alt"),
    ("look_up", "Look Up", "I"),
    ("look_down", "Look Down", "K"),
    ("look_left", "Look Left", "J"),
    ("look_right", "Look Right", "L"),
    ("roll", "Dodge / Sprint", "Space"),
    ("lock_on", "Lock On", "Q"),
    ("attack", "Attack", "Mouse1"),
    ("strong_attack", "Strong Attack", "V"),
    ("strong_modifier", "Strong Attack Modifier", "Left Shift"),
    ("transform", "Transform", "C, Mouse3"),
    ("firearm", "Firearm / Parry", "X, Mouse2"),
    ("blood_vial", "Blood Vial", "R"),
    ("use_item", "Use Item", "F"),
    ("interact", "Interact", "E"),
    ("switch_item", "Switch Item", "Down"),
    ("switch_right", "Switch Right Weapon", "Right"),
    ("switch_left", "Switch Left Weapon", "Left"),
    ("dpad_up", "D-pad Up", "Up"),
    ("gestures", "Gestures", "G"),
    ("personal_effects", "Personal Effects", "T"),
    ("menu", "Menu", "Escape"),
    ("confirm", "Menu Confirm", "Return"),
    ("back", "Menu Back", "Backspace"),
    ("l3", "Left Stick Press", "Left Ctrl"),
];

struct AppState {
    paths: Paths,
    settings: Settings,
    ini: Ini,
    ini_path: PathBuf,
    coop: coop::Host,
}

type Shared = Rc<RefCell<AppState>>;

#[cfg(windows)]
fn configure_windows_runtime() {
    let Ok(executable) = std::env::current_exe() else {
        return;
    };
    let Some(root) = executable.parent() else {
        return;
    };
    std::env::set_var(
        "GSETTINGS_SCHEMA_DIR",
        root.join("share").join("glib-2.0").join("schemas"),
    );
    std::env::set_var(
        "GDK_PIXBUF_MODULEDIR",
        root.join("lib")
            .join("gdk-pixbuf-2.0")
            .join("2.10.0")
            .join("loaders"),
    );
    std::env::set_var("XDG_DATA_DIRS", root.join("share"));
}

#[cfg(not(windows))]
fn configure_windows_runtime() {}

fn main() -> glib::ExitCode {
    configure_windows_runtime();
    let application = Application::builder().application_id(APP_ID).build();
    application.connect_activate(build_window);
    application.run()
}

/// Find the port tree. The Rust launcher often runs from `target/release`, so walk up to the
/// repository root and then check its sibling launcher trees.
fn art_picture(bytes: &'static [u8], height: i32) -> Option<gtk::Picture> {
    let bytes = glib::Bytes::from_static(bytes);
    let texture = gdk::Texture::from_bytes(&bytes).ok()?;
    let picture = gtk::Picture::for_paintable(&texture);
    picture.set_content_fit(gtk::ContentFit::Cover);
    picture.set_size_request(-1, height);
    picture.set_valign(gtk::Align::Start);
    Some(picture)
}

fn resolve_paths() -> Paths {
    let discovered = Paths::discover();
    if discovered.port_dir.join("run.sh").is_file() {
        return discovered;
    }
    let mut ancestor = Some(discovered.port_dir.as_path());
    while let Some(dir) = ancestor {
        if dir.join("run.sh").is_file() {
            return Paths::for_port_dir(dir.to_path_buf());
        }
        if let Some(parent) = dir.parent() {
            for sibling in ["linux_launcher", "windows_laucnher"] {
                let candidate = parent.join(sibling);
                if candidate.join("run.sh").is_file() {
                    return Paths::for_port_dir(candidate);
                }
            }
        }
        ancestor = dir.parent();
    }
    discovered
}

fn build_window(application: &Application) {
    install_css();
    let paths = resolve_paths();
    let settings = Settings::load(&paths);
    let ini_path = paths.ini_path();
    let ini = Ini::load(&ini_path);
    let saves = readiness::saves_dir(&settings, &paths.data_dir);
    let _ = readiness::ensure_baked_trophy_key(&paths, &saves);
    let state: Shared = Rc::new(RefCell::new(AppState {
        paths,
        settings,
        ini,
        ini_path,
        coop: coop::Host::new(),
    }));

    let window = ApplicationWindow::builder()
        .application(application)
        .title("Bloodborne PC")
        .default_width(1100)
        .default_height(760)
        .build();
    window.set_titlebar(Some(
        &gtk::HeaderBar::builder()
            .title_widget(&Label::new(Some("Bloodborne PC")))
            .build(),
    ));

    let stack = Stack::new();
    stack.set_transition_type(gtk::StackTransitionType::Crossfade);
    stack.set_transition_duration(180);
    stack.set_hexpand(true);
    stack.set_vexpand(true);

    let (play_page, checks, folder_label, title_label) = build_play_page(&state, &window);
    add_page(
        &stack,
        &play_page,
        "play",
        "Play",
        "media-playback-start-symbolic",
    );
    add_page(
        &stack,
        &build_display_page(&state),
        "display",
        "Display",
        "video-display-symbolic",
    );
    add_page(
        &stack,
        &build_graphics_page(&state),
        "graphics",
        "Graphics",
        "applications-graphics-symbolic",
    );
    add_page(
        &stack,
        &build_upscaling_page(&state),
        "upscaling",
        "Upscaling",
        "image-x-generic-symbolic",
    );
    add_page(
        &stack,
        &build_performance_page(&state),
        "performance",
        "Performance",
        "speedometer-symbolic",
    );
    add_page(
        &stack,
        &build_compatibility_page(&state),
        "compatibility",
        "Compatibility",
        "preferences-system-symbolic",
    );
    add_page(
        &stack,
        &build_controller_page(&state),
        "controller",
        "Controller",
        "input-gaming-symbolic",
    );
    add_page(
        &stack,
        &build_coop_page(&state),
        "coop",
        "Co-op",
        "emblem-shared-symbolic",
    );
    add_page(
        &stack,
        &build_game_page(&state),
        "game",
        "Game & Data",
        "folder-open-symbolic",
    );
    add_page(
        &stack,
        &build_launcher_page(&state),
        "launcher",
        "Launcher",
        "applications-system-symbolic",
    );
    add_page(
        &stack,
        &build_trophies_page(&state),
        "trophies",
        "Trophies",
        "starred-symbolic",
    );
    add_page(
        &stack,
        &build_diagnostics_page(&state),
        "diagnostics",
        "Diagnostics",
        "utilities-terminal-symbolic",
    );

    let sidebar = StackSidebar::new();
    sidebar.set_stack(&stack);
    sidebar.set_vexpand(true);

    let side = GtkBox::new(Orientation::Vertical, 0);
    side.set_width_request(230);
    side.add_css_class("sidebar");
    let brand = GtkBox::new(Orientation::Vertical, 2);
    brand.add_css_class("sidebar-brand");
    let brand_title = Label::new(Some("Bloodborne PC"));
    brand_title.set_xalign(0.0);
    brand_title.add_css_class("sidebar-title");
    let brand_subtitle = Label::new(Some(&format!("v{} · GTK4", bbport_launcher::VERSION)));
    brand_subtitle.set_xalign(0.0);
    brand_subtitle.add_css_class("sidebar-subtitle");
    brand.append(&brand_title);
    brand.append(&brand_subtitle);
    side.append(&brand);
    side.append(&sidebar);

    let main = GtkBox::new(Orientation::Vertical, 0);
    main.set_hexpand(true);
    main.append(&stack);

    let bottom = GtkBox::new(Orientation::Vertical, 6);
    bottom.add_css_class("actionbar");
    let actions = GtkBox::new(Orientation::Horizontal, 10);
    let refresh = Button::with_label("Refresh");
    let launch = Button::with_label("Launch Game");
    launch.add_css_class("suggested-action");
    launch.set_hexpand(true);
    let status = Label::new(None);
    status.set_xalign(0.0);
    status.set_wrap(true);
    status.add_css_class("status-text");
    actions.append(&refresh);
    actions.append(&launch);
    bottom.append(&actions);
    bottom.append(&status);
    main.append(&bottom);

    let shell = GtkBox::new(Orientation::Horizontal, 0);
    shell.append(&side);
    shell.append(&main);
    window.set_child(Some(&shell));

    refresh_view(&state.borrow(), &title_label, &folder_label, &checks);

    {
        let state = state.clone();
        let title_label = title_label.clone();
        let folder_label = folder_label.clone();
        let checks = checks.clone();
        refresh.connect_clicked(move |_| {
            refresh_view(&state.borrow(), &title_label, &folder_label, &checks);
        });
    }

    {
        let state = state.clone();
        let status = status.clone();
        launch.connect_clicked(move |_| {
            let state = state.borrow();
            let game_dir = state.settings.game_dir();
            if !game_dir.join("eboot.bin").is_file() {
                status.set_text("Choose the extracted game folder containing eboot.bin first.");
                return;
            }
            let run_sh = state.paths.port_dir.join("run.sh");
            if !run_sh.is_file() {
                status.set_text(&format!(
                    "No run.sh at {}. Set BB_PORT_DIR or run from the port tree.",
                    state.paths.port_dir.display()
                ));
                return;
            }
            let saves = readiness::saves_dir(&state.settings, &state.paths.data_dir);
            let _ = readiness::ensure_baked_trophy_key(&state.paths, &saves);
            match process::game_command(&state.settings, &state.paths).spawn() {
                Ok(child) => status.set_text(&format!("Game started (pid {}).", child.id())),
                Err(error) => status.set_text(&format!("Could not start the game: {error}")),
            }
        });
    }

    window.present();
}

fn add_page(stack: &Stack, child: &impl IsA<gtk::Widget>, name: &str, title: &str, icon: &str) {
    stack.add_titled(child, Some(name), title);
    stack.page(child).set_icon_name(icon);
}

fn scrolled_page() -> (ScrolledWindow, GtkBox) {
    let outer = GtkBox::new(Orientation::Vertical, 0);
    outer.set_margin_top(20);
    outer.set_margin_bottom(20);
    outer.set_margin_start(22);
    outer.set_margin_end(22);
    let content = GtkBox::new(Orientation::Vertical, 8);
    content.add_css_class("card");
    outer.append(&content);
    let scroll = ScrolledWindow::new();
    scroll.set_policy(gtk::PolicyType::Never, gtk::PolicyType::Automatic);
    scroll.set_child(Some(&outer));
    (scroll, content)
}

fn section(content: &GtkBox, title: &str) {
    let label = Label::new(Some(&title.to_uppercase()));
    label.set_xalign(0.0);
    label.set_margin_top(14);
    label.add_css_class("section-title");
    content.append(&label);
}

fn note(content: &GtkBox, text: &str) {
    let label = Label::new(Some(text));
    label.set_xalign(0.0);
    label.set_wrap(true);
    label.add_css_class("note");
    content.append(&label);
}

fn row(label_text: &str, widget: &impl IsA<gtk::Widget>) -> GtkBox {
    let row = GtkBox::new(Orientation::Horizontal, 14);
    row.add_css_class("row");
    let label = Label::new(Some(label_text));
    label.set_xalign(0.0);
    label.set_hexpand(true);
    label.set_wrap(true);
    row.append(&label);
    row.append(widget);
    row
}

fn settings_switch(content: &GtkBox, label: &str, key: &'static str, state: &Shared) {
    let switch = Switch::new();
    switch.set_active(state.borrow().settings.get_bool(key));
    switch.set_valign(gtk::Align::Center);
    let state = state.clone();
    switch.connect_active_notify(move |switch| {
        let paths = state.borrow().paths.clone();
        if state
            .borrow_mut()
            .settings
            .set(key, Value::Bool(switch.is_active()))
            .is_ok()
        {
            let _ = state.borrow().settings.save(&paths);
        }
    });
    content.append(&row(label, &switch));
}

fn ini_switch(content: &GtkBox, label: &str, key: &'static str, state: &Shared) {
    let switch = Switch::new();
    switch.set_active(
        state
            .borrow()
            .ini
            .get(key)
            .map(|value| value != "0")
            .unwrap_or(false),
    );
    switch.set_valign(gtk::Align::Center);
    let state = state.clone();
    switch.connect_active_notify(move |switch| {
        let path = state.borrow().ini_path.clone();
        state
            .borrow_mut()
            .ini
            .set(key, if switch.is_active() { "1" } else { "0" });
        let _ = state.borrow().ini.save(&path);
    });
    content.append(&row(label, &switch));
}

fn settings_combo(
    content: &GtkBox,
    label: &str,
    key: &'static str,
    options: &'static [(&'static str, &'static str)],
    state: &Shared,
) {
    let current = state.borrow().settings.get_str(key);
    let labels: Vec<&str> = options.iter().map(|(name, _)| *name).collect();
    let values: Vec<&'static str> = options.iter().map(|(_, value)| *value).collect();
    let dropdown = DropDown::from_strings(&labels);
    if let Some(index) = values.iter().position(|value| *value == current) {
        dropdown.set_selected(index as u32);
    }
    let state = state.clone();
    dropdown.connect_selected_notify(move |dropdown| {
        let Some(value) = values.get(dropdown.selected() as usize) else {
            return;
        };
        let paths = state.borrow().paths.clone();
        if state
            .borrow_mut()
            .settings
            .set(key, Value::String((*value).to_string()))
            .is_ok()
        {
            let _ = state.borrow().settings.save(&paths);
        }
    });
    content.append(&row(label, &dropdown));
}

fn ini_combo(
    content: &GtkBox,
    label: &str,
    key: &'static str,
    options: &'static [(&'static str, &'static str)],
    state: &Shared,
) {
    let current = state.borrow().ini.get(key).unwrap_or("").to_string();
    let labels: Vec<&str> = options.iter().map(|(name, _)| *name).collect();
    let values: Vec<&'static str> = options.iter().map(|(_, value)| *value).collect();
    let dropdown = DropDown::from_strings(&labels);
    if let Some(index) = values.iter().position(|value| *value == current) {
        dropdown.set_selected(index as u32);
    }
    let state = state.clone();
    dropdown.connect_selected_notify(move |dropdown| {
        let Some(value) = values.get(dropdown.selected() as usize) else {
            return;
        };
        let path = state.borrow().ini_path.clone();
        state.borrow_mut().ini.set(key, *value);
        let _ = state.borrow().ini.save(&path);
    });
    content.append(&row(label, &dropdown));
}

fn controller_scheme_row(content: &GtkBox, state: &Shared) {
    let current = state.borrow().settings.get_str("control_scheme");
    let labels: Vec<&str> = CONTROL_SCHEMES.iter().map(|(name, _)| *name).collect();
    let values: Vec<&'static str> = CONTROL_SCHEMES.iter().map(|(_, value)| *value).collect();
    let dropdown = DropDown::from_strings(&labels);
    if let Some(index) = values.iter().position(|value| *value == current) {
        dropdown.set_selected(index as u32);
    }
    let state = state.clone();
    dropdown.connect_selected_notify(move |dropdown| {
        let Some(value) = values.get(dropdown.selected() as usize) else {
            return;
        };
        let (paths, mods_dir, config) = {
            let state = state.borrow();
            (
                state.paths.clone(),
                state.paths.mods_dir(),
                state.paths.mods_config(),
            )
        };
        if state
            .borrow_mut()
            .settings
            .set("control_scheme", Value::String((*value).to_string()))
            .is_ok()
        {
            let _ = state.borrow().settings.save(&paths);
            let _ = assets::apply_control_scheme(value, &mods_dir, &config, &paths.port_dir);
        }
    });
    content.append(&row("Control scheme", &dropdown));
}

#[allow(clippy::too_many_arguments)]
fn settings_spin(
    content: &GtkBox,
    label: &str,
    key: &'static str,
    low: f64,
    high: f64,
    step: f64,
    digits: u32,
    state: &Shared,
) {
    let spin = SpinButton::with_range(low, high, step);
    spin.set_digits(digits);
    spin.set_value(state.borrow().settings.get_i64(key) as f64);
    let state = state.clone();
    spin.connect_value_changed(move |spin| {
        let paths = state.borrow().paths.clone();
        let value = Value::Number(serde_json::Number::from(spin.value() as i64));
        if state.borrow_mut().settings.set(key, value).is_ok() {
            let _ = state.borrow().settings.save(&paths);
        }
    });
    content.append(&row(label, &spin));
}

#[allow(clippy::too_many_arguments)]
fn ini_spin(
    content: &GtkBox,
    label: &str,
    key: &'static str,
    low: f64,
    high: f64,
    step: f64,
    digits: u32,
    state: &Shared,
) {
    let spin = SpinButton::with_range(low, high, step);
    spin.set_digits(digits);
    spin.set_value(state.borrow().ini.int(key, 0) as f64);
    let state = state.clone();
    spin.connect_value_changed(move |spin| {
        let path = state.borrow().ini_path.clone();
        let value = if digits == 0 {
            format!("{}", spin.value() as i64)
        } else {
            format!("{:.2}", spin.value())
        };
        state.borrow_mut().ini.set(key, value);
        let _ = state.borrow().ini.save(&path);
    });
    content.append(&row(label, &spin));
}

fn settings_entry(content: &GtkBox, label: &str, key: &'static str, state: &Shared) {
    let entry = Entry::new();
    entry.set_text(&state.borrow().settings.get_str(key));
    entry.set_hexpand(true);
    let state = state.clone();
    entry.connect_changed(move |entry| {
        let paths = state.borrow().paths.clone();
        if state
            .borrow_mut()
            .settings
            .set(key, Value::String(entry.text().to_string()))
            .is_ok()
        {
            let _ = state.borrow().settings.save(&paths);
        }
    });
    content.append(&row(label, &entry));
}

fn ini_entry(content: &GtkBox, label: &str, key: String, placeholder: &str, state: &Shared) {
    let entry = Entry::new();
    entry.set_text(state.borrow().ini.get(&key).unwrap_or(""));
    entry.set_placeholder_text(Some(placeholder));
    entry.set_hexpand(true);
    let state = state.clone();
    entry.connect_changed(move |entry| {
        let path = state.borrow().ini_path.clone();
        state.borrow_mut().ini.set(&key, entry.text().to_string());
        let _ = state.borrow().ini.save(&path);
    });
    content.append(&row(label, &entry));
}

fn build_play_page(
    state: &Shared,
    window: &ApplicationWindow,
) -> (ScrolledWindow, GtkBox, Label, Label) {
    let (scroll, content) = scrolled_page();
    let title = Label::new(None);
    title.set_xalign(0.0);
    title.add_css_class("hero-title");
    content.append(&title);
    let subtitle = Label::new(Some("Native Vulkan port · Rust / GTK4 launcher"));
    subtitle.set_xalign(0.0);
    subtitle.add_css_class("note");
    content.append(&subtitle);
    if let Some(hero) = art_picture(HERO_ART, 170) {
        content.append(&hero);
    }

    section(&content, "Game folder");
    let folder_row = GtkBox::new(Orientation::Horizontal, 12);
    folder_row.add_css_class("folder-row");
    let folder = Label::new(None);
    folder.set_xalign(0.0);
    folder.set_hexpand(true);
    folder.set_wrap(true);
    folder.add_css_class("dim-label");
    let choose = Button::with_label("Choose game folder…");
    folder_row.append(&folder);
    folder_row.append(&choose);
    content.append(&folder_row);
    let port = state.borrow().paths.port_dir.display().to_string();
    note(&content, &format!("Port: {port}"));

    section(&content, "Readiness");
    let checks = GtkBox::new(Orientation::Vertical, 8);
    content.append(&checks);

    let state_for_choose = state.clone();
    let title_for_choose = title.clone();
    let folder_for_choose = folder.clone();
    let checks_for_choose = checks.clone();
    let window = window.clone();
    choose.connect_clicked(move |_| {
        let dialog = FileDialog::builder()
            .title("Choose the extracted game folder (contains eboot.bin)")
            .modal(true)
            .build();
        let state = state_for_choose.clone();
        let title = title_for_choose.clone();
        let folder = folder_for_choose.clone();
        let checks = checks_for_choose.clone();
        let window = window.clone();
        dialog.select_folder(Some(&window), gio::Cancellable::NONE, move |result| {
            let Ok(file) = result else {
                return;
            };
            let Some(path) = file.path() else {
                return;
            };
            let paths = state.borrow().paths.clone();
            if state
                .borrow_mut()
                .settings
                .set(
                    "game_dir",
                    Value::String(path.to_string_lossy().into_owned()),
                )
                .is_ok()
            {
                let _ = state.borrow().settings.save(&paths);
                refresh_view(&state.borrow(), &title, &folder, &checks);
            }
        });
    });

    (scroll, checks, folder, title)
}

fn build_display_page(state: &Shared) -> ScrolledWindow {
    let (scroll, content) = scrolled_page();
    section(&content, "Output and presentation");
    settings_switch(&content, "Fullscreen", "fullscreen", state);
    settings_switch(&content, "Allow HDR", "hdr", state);
    settings_combo(
        &content,
        "Present mode",
        "present_mode",
        PRESENT_MODES,
        state,
    );
    settings_combo(&content, "Frame-rate mode", "fps_mode", FPS_MODES, state);
    section(&content, "Resolution and HUD");
    ini_combo(
        &content,
        "Output resolution",
        "output_res",
        OUTPUT_RES,
        state,
    );
    ini_spin(
        &content,
        "HUD position (0 = 16:9 centre)",
        "hud_position",
        0.0,
        100.0,
        1.0,
        0,
        state,
    );
    scroll
}

fn build_graphics_page(state: &Shared) -> ScrolledWindow {
    let (scroll, content) = scrolled_page();
    section(&content, "Preset and sharpening");
    ini_combo(&content, "Preset", "preset", PRESETS, state);
    ini_switch(&content, "Sharpening", "sharpen", state);
    ini_spin(&content, "Sharpness", "sharpness", 0.0, 1.0, 0.05, 2, state);
    section(&content, "Effects");
    for (key, _) in EFFECTS {
        ini_switch(&content, key, key, state);
    }
    scroll
}

fn build_upscaling_page(state: &Shared) -> ScrolledWindow {
    let (scroll, content) = scrolled_page();
    section(&content, "Reconstruction");
    ini_combo(&content, "Upscaler", "upscaler", UPSCALERS, state);
    ini_combo(
        &content,
        "Live resolution",
        "live_resolution",
        LIVE_RESOLUTION,
        state,
    );
    ini_switch(&content, "Object motion vectors", "object_motion", state);
    ini_spin(&content, "Model LOD", "model_lod", 0.0, 3.0, 1.0, 0, state);
    section(&content, "Overlay");
    ini_switch(&content, "Show FPS", "show_fps", state);
    note(
        &content,
        "FSR 4/4.1.1 assets are validated by the renderer at startup; missing assets fall back to the selected preset.",
    );
    scroll
}

fn build_performance_page(state: &Shared) -> ScrolledWindow {
    let (scroll, content) = scrolled_page();
    section(&content, "Frame pacing");
    settings_spin(
        &content,
        "Frame-rate limit (0 = off)",
        "fps_limit",
        0.0,
        480.0,
        1.0,
        0,
        state,
    );
    settings_spin(
        &content,
        "Frames ahead",
        "frames_ahead",
        0.0,
        8.0,
        1.0,
        0,
        state,
    );
    section(&content, "Runtime");
    settings_switch(&content, "Precompile shaders", "precompile_shaders", state);
    settings_switch(&content, "Frame statistics", "frame_stats", state);
    settings_switch(&content, "GPU profiling", "gpu_profile", state);
    settings_switch(&content, "MangoHud", "mangohud", state);
    settings_switch(&content, "Discord RPC", "discord_rpc", state);
    scroll
}

fn build_compatibility_page(state: &Shared) -> ScrolledWindow {
    let (scroll, content) = scrolled_page();
    section(&content, "Vulkan pipeline");
    ini_combo(&content, "Draw pipeline", "draw_pipe", DRAW_PIPE, state);
    ini_combo(&content, "Readbacks", "readbacks", READBACKS, state);
    settings_switch(&content, "Vulkan validation", "vk_validation", state);
    note(
        &content,
        "Draw-pipeline auto uses the threaded path on 8 or more CPU threads; readbacks relaxed is the default.",
    );
    scroll
}

fn build_controller_page(state: &Shared) -> ScrolledWindow {
    let (scroll, content) = scrolled_page();
    section(&content, "Controller");
    controller_scheme_row(&content, state);
    let install = Button::with_label("Install keyboard prompts");
    let note_label = Label::new(None);
    note_label.set_xalign(0.0);
    note_label.set_wrap(true);
    note_label.add_css_class("dim-label");
    {
        let state = state.clone();
        let note_label = note_label.clone();
        install.connect_clicked(move |_| {
            let state = state.borrow();
            let mods_dir = state.paths.mods_dir();
            match assets::install_keyboard_prompts(&mods_dir, &state.paths.port_dir) {
                Ok(path) => note_label.set_text(&format!("Installed in {}", path.display())),
                Err(error) => note_label.set_text(&format!("Could not install: {error}")),
            }
        });
    }
    content.append(&install);
    content.append(&note_label);
    section(&content, "Keyboard bindings");
    note(
        &content,
        "Values live in bbport.ini as key_<action>; empty means the default below. \
         While locked on, tap Look Left/Right or use the mouse wheel to switch targets.",
    );
    for (action, label, default) in KEYBINDINGS {
        ini_entry(&content, label, format!("key_{action}"), default, state);
    }
    scroll
}

fn coop_probe(paths: &Paths) -> PathBuf {
    for relative in ["out/bb-probe", "bin/bb-probe"] {
        let candidate = paths.port_dir.join(relative);
        if candidate.is_file() {
            return candidate;
        }
    }
    paths.port_dir.join("out").join("bb-probe")
}

fn coop_game_command(
    state: &AppState,
    address: &str,
    name: &str,
    password: &str,
) -> Result<std::process::Command, String> {
    let mut environment = bbport_launcher::env::game_environment(&state.settings, &state.paths);
    let saves = readiness::saves_dir(&state.settings, &state.paths.data_dir);
    let _ = readiness::ensure_baked_trophy_key(&state.paths, &saves);
    let backup = coop::backup_saves(&saves).ok().flatten();
    let probe = coop_probe(&state.paths);
    coop::co_op_environment(
        &mut environment,
        address,
        name,
        password,
        &probe,
        backup.as_deref(),
    )
    .map_err(|error| error.to_string())?;
    let mut command = process::game_command(&state.settings, &state.paths);
    for (key, value) in environment {
        command.env(key, value);
    }
    Ok(command)
}

fn coop_status_text(_state: &AppState) -> String {
    let client = coop::installed("client").unwrap_or(false);
    let server = coop::installed("server").unwrap_or(false);
    let (address, network) = coop::host_address();
    let mut text = format!(
        "BBSC client: {}\nBBSC server: {}\nCo-op data: {}",
        if client { "installed" } else { "not installed" },
        if server { "installed" } else { "not installed" },
        coop::coop_dir().display()
    );
    match address {
        Some(address) => text.push_str(&format!(
            "\nHost address: {address} ({})",
            network.unwrap_or("local")
        )),
        None => text.push_str("\nHost address: none (connect Tailscale or a home network)"),
    }
    text
}

fn build_coop_page(state: &Shared) -> ScrolledWindow {
    let (scroll, content) = scrolled_page();
    section(&content, "Seamless co-op (BBSC)");
    note(
        &content,
        "Experimental: BBSC keeps a co-op session alive through deaths, lamps, areas and bosses. \
         The official BBSC packages are downloaded on demand and checked against their published \
         SHA-256; no game files or BBSC binaries are bundled.",
    );

    let status = Label::new(Some(&coop_status_text(&state.borrow())));
    status.set_xalign(0.0);
    status.set_wrap(true);
    status.set_selectable(true);
    status.add_css_class("note");
    content.append(&status);

    let setup = Button::with_label("Download / update BBSC packages");
    content.append(&setup);

    section(&content, "Host");
    let address = Label::new(Some(
        &coop::host_address()
            .0
            .unwrap_or_else(|| "No network address".into()),
    ));
    address.set_xalign(0.0);
    address.set_selectable(true);
    address.add_css_class("note");
    content.append(&address);

    let name = Entry::new();
    name.set_placeholder_text(Some("Co-op name (3-16 letters, numbers, - or _)"));
    name.set_text(state.borrow().settings.get_str("player_name").as_str());
    content.append(&row("Co-op name", &name));

    let password = Entry::new();
    password.set_placeholder_text(Some("Password (at least 4 characters)"));
    password.set_visibility(false);
    content.append(&row("Password", &password));

    let host_actions = GtkBox::new(Orientation::Horizontal, 8);
    let start = Button::with_label("Start hosting");
    let stop = Button::with_label("Stop hosting");
    host_actions.append(&start);
    host_actions.append(&stop);
    content.append(&host_actions);

    let invite = Entry::new();
    invite.set_placeholder_text(Some("Invite code (BBCOOP-…)"));
    invite.set_hexpand(true);
    let copy = Button::with_label("Copy invite");
    let invite_row = GtkBox::new(Orientation::Horizontal, 8);
    invite_row.append(&invite);
    invite_row.append(&copy);
    content.append(&row("Invite", &invite_row));

    section(&content, "Join");
    note(
        &content,
        "Paste the host's invite, create the account on their server, then launch the game. \
         Hosts can use the same buttons after starting hosting.",
    );
    let join_actions = GtkBox::new(Orientation::Horizontal, 8);
    let account = Button::with_label("Create account");
    let launch = Button::with_label("Launch co-op");
    launch.add_css_class("suggested-action");
    join_actions.append(&account);
    join_actions.append(&launch);
    content.append(&join_actions);

    let action_status = Label::new(None);
    action_status.set_xalign(0.0);
    action_status.set_wrap(true);
    action_status.add_css_class("note");
    content.append(&action_status);

    {
        let state = state.clone();
        let status = status.clone();
        setup.connect_clicked(move |_| {
            status.set_text("Downloading BBSC client…");
            let fetcher = bbport_launcher::update::CurlFetcher;
            if let Err(error) =
                coop::download("client", &fetcher, Some(&|text| status.set_text(text)))
            {
                status.set_text(&format!("Client setup failed: {error}"));
                return;
            }
            status.set_text("Downloading BBSC server…");
            if let Err(error) =
                coop::download("server", &fetcher, Some(&|text| status.set_text(text)))
            {
                status.set_text(&format!("Server setup failed: {error}"));
                return;
            }
            status.set_text(&coop_status_text(&state.borrow()));
        });
    }

    {
        let state = state.clone();
        let status = status.clone();
        let name = name.clone();
        let password = password.clone();
        let invite = invite.clone();
        let address = address.clone();
        start.connect_clicked(move |_| {
            let (Some(host), _) = coop::host_address() else {
                status.set_text("No host address: connect Tailscale or a home network first.");
                return;
            };
            let port_dir = state.borrow().paths.port_dir.clone();
            let result = state.borrow_mut().coop.start(&host, &port_dir);
            match result {
                Ok(()) => {
                    address.set_text(&host);
                    let invite_text = match coop::registration_key(&port_dir) {
                        Ok(key) => coop::make_invite(&host, &key),
                        Err(error) => {
                            status
                                .set_text(&format!("Servers started, but no invite key: {error}"));
                            return;
                        }
                    };
                    invite.set_text(&invite_text);
                    let account_result = if name.text().is_empty() || password.text().is_empty() {
                        None
                    } else {
                        Some(coop::create_account(
                            &host,
                            &coop::read_invite(&invite_text)
                                .map(|(_, key)| key)
                                .unwrap_or_default(),
                            name.text().as_str(),
                            password.text().as_str(),
                        ))
                    };
                    match account_result {
                        Some(Err(error)) => status
                            .set_text(&format!("Hosting on {host}; account setup failed: {error}")),
                        _ => status.set_text(&format!(
                            "Hosting on {host}. Share the invite code with your party."
                        )),
                    }
                }
                Err(error) => status.set_text(&format!("Could not start hosting: {error}")),
            }
        });
    }

    {
        let state = state.clone();
        let status = status.clone();
        stop.connect_clicked(move |_| {
            state.borrow_mut().coop.stop();
            status.set_text("Hosting stopped.");
        });
    }

    {
        let invite = invite.clone();
        let status = status.clone();
        copy.connect_clicked(move |_| {
            if let Some(display) = gdk::Display::default() {
                display.clipboard().set_text(invite.text().as_str());
                status.set_text("Invite copied to the clipboard.");
            }
        });
    }

    {
        let action_status = action_status.clone();
        let invite = invite.clone();
        let name = name.clone();
        let password = password.clone();
        account.connect_clicked(move |_| {
            let code = invite.text();
            let (address, key) = match coop::read_invite(&code) {
                Ok(parts) => parts,
                Err(error) => {
                    action_status.set_text(&format!("Invalid invite: {error}"));
                    return;
                }
            };
            let name = name.text().to_string();
            let password = password.text().to_string();
            match coop::create_account(&address, &key, &name, &password) {
                Ok(result) => action_status.set_text(&format!("Co-op account: {result}.")),
                Err(error) => action_status.set_text(&format!("Account setup failed: {error}")),
            }
        });
    }

    {
        let state = state.clone();
        let action_status = action_status.clone();
        let invite = invite.clone();
        let name = name.clone();
        let password = password.clone();
        launch.connect_clicked(move |_| {
            let code = invite.text();
            let (address, _) = match coop::read_invite(&code) {
                Ok(parts) => parts,
                Err(error) => {
                    action_status.set_text(&format!("Invalid invite: {error}"));
                    return;
                }
            };
            let name = name.text().to_string();
            let password = password.text().to_string();
            if name.is_empty() || password.is_empty() {
                action_status.set_text("Enter the co-op name and password first.");
                return;
            }
            let state = state.borrow();
            match coop_game_command(&state, &address, &name, &password) {
                Ok(mut command) => match command.spawn() {
                    Ok(child) => {
                        action_status.set_text(&format!("Co-op game started (pid {}).", child.id()))
                    }
                    Err(error) => {
                        action_status.set_text(&format!("Could not start co-op: {error}"))
                    }
                },
                Err(error) => action_status.set_text(&format!("Could not prepare co-op: {error}")),
            }
        });
    }

    scroll
}

fn build_game_page(state: &Shared) -> ScrolledWindow {
    let (scroll, content) = scrolled_page();
    section(&content, "Folders");
    settings_entry(&content, "Game folder", "game_dir", state);
    settings_entry(&content, "Save data folder", "user_dir", state);
    settings_entry(&content, "Mods folder", "mods_dir", state);
    settings_entry(&content, "Patches folder", "patches_dir", state);
    settings_entry(&content, "DLC folder", "dlc_dir", state);
    section(&content, "Game");
    settings_combo(
        &content,
        "In-game language",
        "language",
        GAME_LANGUAGES,
        state,
    );
    settings_entry(&content, "Player name", "player_name", state);
    settings_switch(&content, "Enable mods", "mods_enabled", state);
    section(&content, "Installed content");
    {
        let state = state.borrow();
        let mods = bbport_launcher::mods::discover(&state.paths.mods_dir());
        note(
            &content,
            &if mods.is_empty() {
                "No mods found.".to_string()
            } else {
                format!("Mods ({}): {}", mods.len(), mods.join(", "))
            },
        );
        let game_dir = state.settings.game_dir();
        let version = sfo::game_info(&game_dir)
            .map(|info| info.version)
            .unwrap_or_default();
        let patches =
            bbport_launcher::patches::external_patches(&state.paths.patches_dir(), &version, None);
        note(
            &content,
            &if patches.is_empty() {
                "No external patches found.".to_string()
            } else {
                format!(
                    "External patches ({}): {}",
                    patches.len(),
                    patches
                        .iter()
                        .map(|patch| patch.key.as_str())
                        .collect::<Vec<_>>()
                        .join(", ")
                )
            },
        );
    }
    section(&content, "Enhancements");
    for (key, _) in ENHANCEMENTS {
        ini_switch(&content, key, key, state);
    }
    section(&content, "Cheats and tweaks");
    for (key, _) in CHEATS_AND_TWEAKS {
        ini_switch(&content, key, key, state);
    }
    scroll
}

fn build_launcher_page(state: &Shared) -> ScrolledWindow {
    let (scroll, content) = scrolled_page();
    section(&content, "Launcher");
    settings_combo(&content, "UI language", "ui_language", UI_LANGUAGES, state);
    settings_switch(&content, "Check for updates", "check_updates", state);
    settings_switch(&content, "Reduce motion", "reduce_motion", state);
    settings_entry(&content, "Extra environment variables", "extra_env", state);
    note(
        &content,
        "Settings are written to settings.json; bbport.ini is updated in place with comments kept.",
    );
    scroll
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TrophySort {
    Default,
    UnlockTime,
    Name,
}

fn trophy_row(entry: &trophies::TrophyEntry, unlocked: bool) -> GtkBox {
    let row = GtkBox::new(Orientation::Horizontal, 10);
    row.add_css_class("trophy-row");
    if !unlocked {
        row.add_css_class("trophy-locked");
    }
    let text = GtkBox::new(Orientation::Vertical, 2);
    let name = Label::new(None);
    name.set_markup(&format!("<b>{}</b>", glib::markup_escape_text(&entry.name)));
    name.set_xalign(0.0);
    name.set_wrap(true);
    let description = Label::new(Some(&entry.description));
    description.set_xalign(0.0);
    description.set_wrap(true);
    description.add_css_class("note");
    text.append(&name);
    if !entry.description.is_empty() {
        text.append(&description);
    }
    let status = Label::new(Some(if unlocked { "unlocked" } else { "locked" }));
    status.add_css_class(if unlocked { "status-ready" } else { "note" });
    if let Some(icon) = entry.icon.as_deref().filter(|path| path.is_file()) {
        let image = gtk::Image::from_file(icon);
        image.set_pixel_size(36);
        image.set_valign(gtk::Align::Center);
        row.append(&image);
    } else {
        let grade = Label::new(Some(&format!("[{}]", entry.grade)));
        grade.set_width_chars(4);
        grade.add_css_class("trophy-grade");
        row.append(&grade);
    }
    row.append(&text);
    row.append(&status);
    row
}

fn render_trophies(
    report: &trophies::TrophyReport,
    sort: TrophySort,
    unlocked_box: &GtkBox,
    locked_box: &GtkBox,
) {
    while let Some(child) = unlocked_box.first_child() {
        unlocked_box.remove(&child);
    }
    while let Some(child) = locked_box.first_child() {
        locked_box.remove(&child);
    }
    let mut unlocked: Vec<&trophies::TrophyEntry> =
        report.entries.iter().filter(|entry| entry.earned).collect();
    let mut locked: Vec<&trophies::TrophyEntry> = report
        .entries
        .iter()
        .filter(|entry| !entry.earned)
        .collect();
    match sort {
        TrophySort::Default => {
            unlocked.sort_by_key(|entry| entry.id);
            locked.sort_by_key(|entry| entry.id);
        }
        TrophySort::UnlockTime => {
            unlocked.sort_by(|left, right| {
                right
                    .unlocked_at
                    .unwrap_or(0)
                    .cmp(&left.unlocked_at.unwrap_or(0))
                    .then(left.id.cmp(&right.id))
            });
            locked.sort_by_key(|entry| entry.id);
        }
        TrophySort::Name => {
            let by_name = |left: &&trophies::TrophyEntry, right: &&trophies::TrophyEntry| {
                left.name
                    .to_lowercase()
                    .cmp(&right.name.to_lowercase())
                    .then(left.id.cmp(&right.id))
            };
            unlocked.sort_by(by_name);
            locked.sort_by(by_name);
        }
    }
    if unlocked.is_empty() {
        let empty = Label::new(Some("No unlocked trophies yet."));
        empty.set_xalign(0.0);
        empty.add_css_class("note");
        unlocked_box.append(&empty);
    } else {
        for entry in unlocked {
            unlocked_box.append(&trophy_row(entry, true));
        }
    }
    if locked.is_empty() {
        let empty = Label::new(Some("No locked trophies in this set."));
        empty.set_xalign(0.0);
        empty.add_css_class("note");
        locked_box.append(&empty);
    } else {
        for entry in locked {
            locked_box.append(&trophy_row(entry, false));
        }
    }
}

fn build_trophies_page(state: &Shared) -> ScrolledWindow {
    let (scroll, content) = scrolled_page();
    if let Some(banner) = art_picture(HERO_ART, 96) {
        content.append(&banner);
    }
    section(&content, "Trophy collection");
    let state = state.borrow();
    let saves = readiness::saves_dir(&state.settings, &state.paths.data_dir);
    let game = state.settings.game_dir();
    let report = trophies::load_trophy_report(
        &saves,
        (!game.as_os_str().is_empty()).then_some(game.as_path()),
    );
    let unlocked_count = report.entries.iter().filter(|entry| entry.earned).count();
    let locked_count = report.entries.len().saturating_sub(unlocked_count);
    let summary = match report.total {
        Some(total) => format!(
            "{} — {} / {} trophies ({}%) · {} unlocked · {} locked",
            report.set_title,
            report.earned,
            total,
            report.completion.unwrap_or(0),
            unlocked_count,
            locked_count
        ),
        None => format!("{} — {} unlocked", report.set_title, report.earned),
    };
    let label = Label::new(Some(&summary));
    label.set_xalign(0.0);
    label.set_wrap(true);
    label.add_css_class("hero-title");
    content.append(&label);
    if let Some(reason) = &report.reason {
        note(&content, reason);
    }
    let platinum: Vec<&trophies::TrophyEntry> = report
        .entries
        .iter()
        .filter(|entry| entry.grade == "P")
        .collect();
    let platinum_earned = platinum.iter().any(|entry| entry.earned);
    let platinum_card = GtkBox::new(Orientation::Horizontal, 10);
    platinum_card.add_css_class("check-card");
    let platinum_icon = gtk::Image::from_icon_name("starred-symbolic");
    platinum_icon.set_pixel_size(34);
    let platinum_text = GtkBox::new(Orientation::Vertical, 2);
    let platinum_title = Label::new(None);
    platinum_title.set_markup("<b>Platinum</b>");
    platinum_title.set_xalign(0.0);
    let platinum_detail = Label::new(Some(if platinum_earned {
        "Platinum trophy earned — every base trophy is unlocked."
    } else if report.metadata_available {
        "Platinum trophy locked — unlock every base trophy to earn it."
    } else {
        "Platinum status becomes available after the trophy cache is built."
    }));
    platinum_detail.set_xalign(0.0);
    platinum_detail.set_wrap(true);
    platinum_detail.add_css_class("note");
    platinum_text.append(&platinum_title);
    platinum_text.append(&platinum_detail);
    platinum_card.append(&platinum_icon);
    platinum_card.append(&platinum_text);
    content.append(&platinum_card);
    let sort = DropDown::from_strings(&["Default order", "Unlock time (newest)", "Trophy name"]);
    content.append(&row("Sort unlocked trophies", &sort));
    let unlocked_box = GtkBox::new(Orientation::Vertical, 6);
    let locked_box = GtkBox::new(Orientation::Vertical, 6);
    section(&content, &format!("Unlocked ({unlocked_count})"));
    content.append(&unlocked_box);
    section(&content, &format!("Locked ({locked_count})"));
    content.append(&locked_box);
    let report = Rc::new(report);
    render_trophies(&report, TrophySort::Default, &unlocked_box, &locked_box);
    let report_for_sort = report.clone();
    let unlocked_for_sort = unlocked_box.clone();
    let locked_for_sort = locked_box.clone();
    sort.connect_selected_notify(move |sort| {
        let mode = match sort.selected() {
            1 => TrophySort::UnlockTime,
            2 => TrophySort::Name,
            _ => TrophySort::Default,
        };
        render_trophies(&report_for_sort, mode, &unlocked_for_sort, &locked_for_sort);
    });
    scroll
}

fn build_diagnostics_page(state: &Shared) -> ScrolledWindow {
    let (scroll, content) = scrolled_page();
    section(&content, "Paths");
    let state = state.borrow();
    for (label, value) in [
        ("Port", state.paths.port_dir.display().to_string()),
        ("Data", state.paths.data_dir.display().to_string()),
        ("Config", state.paths.config_dir.display().to_string()),
        (
            "settings.json",
            state.paths.config_file.display().to_string(),
        ),
        ("bbport.ini", state.ini_path.display().to_string()),
        (
            "Saves",
            readiness::saves_dir(&state.settings, &state.paths.data_dir)
                .display()
                .to_string(),
        ),
    ] {
        let line = Label::new(Some(&format!("{label}: {value}")));
        line.set_xalign(0.0);
        line.set_selectable(true);
        line.set_wrap(true);
        content.append(&line);
    }
    section(&content, "Version");
    let version = Label::new(Some(&format!("bbport-rs {}", bbport_launcher::VERSION)));
    version.set_xalign(0.0);
    content.append(&version);
    scroll
}

fn refresh_view(state: &AppState, title: &Label, folder: &Label, checks: &GtkBox) {
    let game_dir = state.settings.game_dir();
    let game = (!game_dir.as_os_str().is_empty()).then_some(game_dir.as_path());
    let info = game.and_then(sfo::game_info);
    let saves = readiness::saves_dir(&state.settings, &state.paths.data_dir);
    let results = readiness::check_readiness(game, &saves);

    title.set_text(&match info {
        Some(info) => format!("{} ({})", info.title, info.version),
        None => "Bloodborne PC".to_string(),
    });
    folder.set_text(&if game_dir.as_os_str().is_empty() {
        "No game folder selected".to_string()
    } else {
        format!("Game folder: {}", game_dir.display())
    });

    while let Some(child) = checks.first_child() {
        checks.remove(&child);
    }
    for check in results {
        let row = GtkBox::new(Orientation::Horizontal, 12);
        row.add_css_class("check-card");
        let badge = Label::new(Some(&check.level.as_str().to_uppercase()));
        badge.add_css_class("status-badge");
        badge.add_css_class(match check.level {
            readiness::Level::Ready => "status-ready",
            readiness::Level::Warning => "status-warning",
            readiness::Level::Error => "status-error",
        });
        let text = GtkBox::new(Orientation::Vertical, 2);
        let label = Label::new(None);
        label.set_markup(&format!(
            "<b>{}</b>",
            glib::markup_escape_text(&check.label)
        ));
        label.set_xalign(0.0);
        let detail = Label::new(Some(&check.detail));
        detail.set_xalign(0.0);
        detail.set_wrap(true);
        detail.add_css_class("dim-label");
        text.append(&label);
        text.append(&detail);
        row.append(&badge);
        row.append(&text);
        checks.append(&row);
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn discovery_finds_a_port_tree_with_run_sh() {
        let paths = super::resolve_paths();
        assert!(
            paths.port_dir.join("run.sh").is_file(),
            "resolved {} which has no run.sh",
            paths.port_dir.display()
        );
    }
}
