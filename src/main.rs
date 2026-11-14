#![allow(float_literal_f32_fallback)]
#![allow(dead_code)]
mod audio;
#[cfg(target_os = "macos")]
mod native_overlay;
#[cfg(not(target_os = "macos"))]
#[path = "native_overlay_stub.rs"]
mod native_overlay;
mod keys;
mod paths;
mod platform;
mod dictation;
mod engine;
mod features;
mod hub;
mod meeting;
mod model;
mod notes;
mod pages;
mod theme;
mod ui;
mod voice;
mod welcome;

use dictation::{OverlayAction, Phase};
use eframe::egui;
use model::*;
use std::path::PathBuf;
use std::time::{Duration, Instant};
use theme::Palette;

// Values recovered from the original app; see MECHANICS.md.
pub const RECORDING_SAMPLE_RATE: u32 = 16000;
pub const DICTATION_DEBOUNCE_MS: f32 = 500.0;
pub const MAX_DICTATION_SECS: f32 = 1200.0;
pub const WARN_DICTATION_SECS: f32 = 1140.0;
pub const SCRATCHPAD_PTT_DELAY_MS: f32 = 350.0;
pub const DOUBLE_TAP_WINDOW_MS: u128 = 500;
/// Other audio is silenced 0.3 s after the start cue, so the cue itself is
/// still heard.
pub const MUTE_AFTER_START_SOUND_MS: f32 = 300.0;
/// After a stop, wait 0.5 s for the output device to settle before restoring
/// the volume, so the stop cue is not clipped.
pub const AUDIO_RESTORE_DELAY_MS: f32 = 500.0;
/// The overlay stays up for this long after the key is released.
pub const STOP_LINGER_SECS: f32 = 2.0;
pub const SNIPPET_TRIGGER_MAX: usize = 60;
pub const SNIPPET_EXPANSION_MAX: usize = 4000;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Screen {
    Welcome,
    Hub,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Page {
    Home,
    Dictionary,
    Snippets,
    Style,
    Transforms,
    Scratchpad,
    Notetaker,
    Insights,
    MyVoice,
    Settings,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum SettingsTab {
    General,
    System,
    Popup,
    Data,
    Account,
    About,
}

#[derive(Clone)]
pub enum Modal {
    AddWord,
    EditWord(usize),
    DeleteWord(usize),
    AddSnippet,
    EditSnippet(usize),
    DeleteSnippet(usize),
    DeleteHistory,
}

pub struct App {
    pub screen: Screen,
    pub page: Page,
    pub settings_tab: SettingsTab,
    pub state: State,
    pub search: String,
    pub style_tab: usize,
    pub modal: Option<Modal>,
    pub form1: String,
    pub form2: String,
    pub toast: Option<(String, Instant)>,
    pub dict: Phase,
    pub last_pasted: Option<String>,
    pub frames: u32,
    pub shot: Option<PathBuf>,
    pub shot_done: bool,
    pub start_time: Instant,
    pub last_ptt_press: Option<Instant>,
    /// When the key was released for the clip now being transcribed; the
    /// overlay's 2.0 s linger is measured from here.
    pub stop_at: Option<Instant>,
    pub mute_at: Option<Instant>,
    /// Native restores the volume 0.5 s after the codec settles, not at the
    /// instant the stop cue plays. Pending restores are cancelled by a restart.
    pub unmute_at: Option<Instant>,
    pub muted_by_us: bool,
    pub rec: Option<audio::Recorder>,
    pub mic_error: Option<String>,
    pub engine: engine::Discovery,
    pub engine_checking: bool,
    pub events: Option<std::sync::mpsc::Receiver<engine::Event>>,
    /// The model id captured when the clip was sent, so history is accurate
    /// even if the person changes the setting while it transcribes.
    pub active_model: String,
    /// Whether the detected-model menu is open in Settings.
    pub model_menu: bool,
    pub from_model: bool,
    /// Last clip sent to the engine, kept so a failed transcription can be retried.
    pub last_audio: Option<(Vec<u8>, f32)>,
    /// App that was frontmost when a global (Fn) dictation started. `None` when
    /// the dictation began inside the hub, which keeps the Scratchpad sink.
    pub global_target: Option<String>,
    /// Clipboard contents to put back after a paste lands.
    pub clipboard_restore: Option<(platform::ClipboardRestore, Instant)>,
    /// Set by the menu bar so the next close request really exits.
    pub quitting: bool,
    /// Whether the Input Monitoring hint has already been shown.
    pub hotkey_denied_note: bool,
    /// Stop/Cancel clicks coming from the floating overlay window.
    pub overlay_actions: std::sync::Arc<std::sync::Mutex<Vec<dictation::OverlayAction>>>,
    /// Accessibility is only prompted for once per session.
    pub asked_accessibility: bool,
    /// Window is hidden in the menu bar; pause idle animations then.
    pub hidden_to_tray: bool,
    /// Live voice bands shared with the meters while recording.
    pub meter: Option<std::sync::Arc<std::sync::Mutex<[f32; 7]>>>,
    /// Render-rate smoothed copy for the Home hero.
    pub smooth_bands: [f32; 7],
    /// Render-rate smoothed copy for the floating pill.
    pub overlay_smooth: std::sync::Arc<std::sync::Mutex<[f32; 7]>>,
    /// First-run permission guide, also reachable from the menu bar.
    pub show_permissions: bool,
    /// Dev flag: keep the permission sheet open for layout checks.
    pub permissions_forced: bool,
    /// One-shot placement of the floating popup for this dictation.
    popup_placed: bool,
    /// Last known popup origin in screen points.
    pub popup_origin: std::sync::Arc<std::sync::Mutex<Option<(f32, f32)>>>,
    /// A global dictation could not paste; show the permission guide.
    pub pending_permissions_prompt: bool,
    /// Notetaker's saved notes (notes.json).
    pub notes: notes::NoteStore,
    /// Transforms and Notetaker runtime state.
    pub ext: features::Extras,
}

fn arg_value<'a>(args: &'a [String], flag: &str) -> Option<&'a str> {
    args.iter()
        .position(|a| a == flag)
        .and_then(|i| args.get(i + 1))
        .map(|s| s.as_str())
}

#[cfg_attr(target_os = "macos", link(name = "System", kind = "dylib"))]
extern "C" {
    fn flock(fd: i32, operation: i32) -> i32;
}

enum InstanceLock {
    Held(std::fs::File),
    Duplicate,
    /// No writable Application Support (sandboxed dev run): carry on unlocked.
    Unavailable,
}

/// Hold an exclusive `flock` for the life of the process. The file descriptor
/// is returned so the lock outlives this call.
fn single_instance_lock() -> InstanceLock {
    use std::os::unix::io::AsRawFd;
    const LOCK_EX: i32 = 2;
    const LOCK_NB: i32 = 4;
    // errno for "would block": 35 on macOS, 11 on Linux.
    const EWOULDBLOCK: i32 = if cfg!(target_os = "macos") { 35 } else { 11 };
    let Some(dir) = paths::data_dir() else {
        return InstanceLock::Unavailable;
    };
    if std::fs::create_dir_all(&dir).is_err() {
        return InstanceLock::Unavailable;
    }
    let file = match std::fs::OpenOptions::new()
        .create(true)
        .truncate(false)
        .write(true)
        .open(dir.join("instance.lock"))
    {
        Ok(f) => f,
        Err(_) => return InstanceLock::Unavailable,
    };
    if unsafe { flock(file.as_raw_fd(), LOCK_EX | LOCK_NB) == 0 } {
        InstanceLock::Held(file)
    } else if std::io::Error::last_os_error().raw_os_error() == Some(EWOULDBLOCK) {
        InstanceLock::Duplicate
    } else {
        InstanceLock::Unavailable
    }
}

/// The app icon for the window (Linux; macOS takes it from the bundle).
fn window_icon() -> Option<egui::IconData> {
    let bytes = include_bytes!("../dist/icon-source.png");
    let img = image::load_from_memory_with_format(bytes, image::ImageFormat::Png).ok()?;
    let rgba = img.to_rgba8();
    let (width, height) = rgba.dimensions();
    Some(egui::IconData { rgba: rgba.into_raw(), width, height })
}

fn main() -> eframe::Result<()> {
    let args: Vec<String> = std::env::args().collect();
    // CLI overrides, so any model/endpoint can be exercised without the UI.
    if let Some(url) = arg_value(&args, "--endpoint") {
        std::env::set_var("FLOWLINE_ENGINE_URL", url);
    }
    if let Some(model) = arg_value(&args, "--model") {
        std::env::set_var("FLOWLINE_MODEL", model);
    }
    if args
        .iter()
        .any(|a| a == "--engine-probe" || a == "--lmf3-probe")
    {
        let wanted = engine::requested_model(engine::DEFAULT_MODEL);
        let found = engine::discover(engine::DEFAULT_BASE, &wanted);
        println!("engine endpoint: {}", found.base);
        println!("selected model: {wanted}");
        println!("models: {:?}", found.models);
        println!("reachable: {} listed: {}", found.reachable, found.listed);
        println!(
            "speech model on server: {}",
            if engine::has_speech_model(&found.models) || found.models.is_empty() {
                "yes/unknown"
            } else {
                "no"
            }
        );
        if !found.models.is_empty() && !engine::has_speech_model(&found.models) {
            if engine::has_tts_model(&found.models) {
                println!(
                    "warning: this is a text-to-speech server (Kokoro/tts-1); \
                     dictation needs a speech-to-text model such as Whisper"
                );
            } else {
                println!(
                    "warning: no speech-to-text model here (Ollama cannot transcribe audio); \
                     run a Whisper server such as speaches and use its /v1 endpoint"
                );
            }
        }
        match &found.problem {
            Some(p) => println!("problem: {p}"),
            None => println!("ready"),
        }
        return Ok(());
    }
    if let Some(i) = args.iter().position(|a| a == "--transcribe") {
        let Some(path) = args.get(i + 1) else {
            eprintln!("usage: flowline --transcribe <file.wav> [--model <id>] [--endpoint <url>]");
            return Ok(());
        };
        let bytes = match std::fs::read(path) {
            Ok(b) => b,
            Err(e) => {
                eprintln!("cannot read {path}: {e}");
                std::process::exit(2);
            }
        };
        let base = engine::requested_base(engine::DEFAULT_BASE);
        let model = engine::requested_model(engine::DEFAULT_MODEL);
        // `--quiet` applies quiet-speech mode: boost the clip, relax the VAD.
        let quiet = args.iter().any(|a| a == "--quiet");
        let bytes = if quiet {
            let (boosted, gain) = audio::boost_quiet(&bytes);
            eprintln!("quiet mode: gain x{gain:.1}");
            boosted
        } else {
            bytes
        };
        let result = if args.iter().any(|a| a == "--local-whisper") {
            engine::transcribe_local_opts(&bytes, &model, "en", quiet)
        } else {
            engine::transcribe(&base, &model, &bytes, "en")
        };
        match result {
            Ok(text) => println!("{text}"),
            Err(e) => {
                eprintln!("{model}: {e}");
                std::process::exit(2);
            }
        }
        return Ok(());
    }
    // `--transform <text>` / `--summarize <text>` exercise the text model that
    // Transforms and Notetaker use, without the UI.
    for (flag, system) in [
        ("--transform", model::POLISH_PROMPT),
        ("--summarize", notes::SUMMARY_PROMPT),
    ] {
        if let Some(i) = args.iter().position(|a| a == flag) {
            let Some(text) = args.get(i + 1) else {
                eprintln!(
                    "usage: flowline {flag} <text> --text-endpoint <url> --text-model <id>"
                );
                std::process::exit(2);
            };
            let base = arg_value(&args, "--text-endpoint")
                .unwrap_or(model::DEFAULT_TEXT_BASE)
                .to_string();
            let model_id = arg_value(&args, "--text-model").unwrap_or("").to_string();
            match engine::chat(&base, &model_id, system, text, Duration::from_secs(180)) {
                Ok(out) => println!("{out}"),
                Err(e) => {
                    eprintln!("{base} {model_id}: {e}");
                    std::process::exit(2);
                }
            }
            return Ok(());
        }
    }
    // `--command <instruction> <text>`: what command mode sends once your
    // spoken instruction has been transcribed.
    if let Some(i) = args.iter().position(|a| a == "--command") {
        let (Some(instruction), Some(text)) = (args.get(i + 1), args.get(i + 2)) else {
            eprintln!("usage: flowline --command <instruction> <text> --text-endpoint <url> --text-model <id>");
            std::process::exit(2);
        };
        let base = arg_value(&args, "--text-endpoint")
            .unwrap_or(model::DEFAULT_TEXT_BASE)
            .to_string();
        let model_id = arg_value(&args, "--text-model").unwrap_or("").to_string();
        let user = features::command_input(instruction, text);
        match engine::chat(&base, &model_id, features::COMMAND_PROMPT, &user, Duration::from_secs(180)) {
            Ok(out) => println!("{out}"),
            Err(e) => {
                eprintln!("{base} {model_id}: {e}");
                std::process::exit(2);
            }
        }
        return Ok(());
    }
    if args.iter().any(|a| a == "--record-test") {
        match audio::Recorder::start(None) {
            Ok(r) => {
                std::thread::sleep(Duration::from_millis(1200));
                match r.finish() {
                    Some(wav) => println!(
                        "recorded {} bytes ({:.2} s)",
                        wav.len(),
                        audio::wav_seconds(&wav)
                    ),
                    None => println!("recorded too little audio"),
                }
            }
            Err(e) => {
                eprintln!("microphone: {e}");
                std::process::exit(2);
            }
        }
        return Ok(());
    }
    if let Some(i) = args.iter().position(|a| a == "--voice-test") {
        let secs = args
            .get(i + 1)
            .and_then(|v| v.parse::<f32>().ok())
            .unwrap_or(1.5);
        match audio::Recorder::start(None) {
            Ok(r) => {
                std::thread::sleep(Duration::from_secs_f32(secs.max(0.4)));
                match r.finish() {
                    Some(wav) => {
                        println!("captured {:.2} s of 16 kHz audio", audio::wav_seconds(&wav));
                        let base = engine::requested_base(engine::DEFAULT_BASE);
                        let model = engine::requested_model(engine::DEFAULT_MODEL);
                        let local = args.iter().any(|a| a == "--local-whisper");
                        let result = if local {
                            engine::transcribe_local(&wav, &model, "en")
                        } else {
                            engine::transcribe(&base, &model, &wav, "en")
                        };
                        match result {
                            Ok(text) => println!("{model}: {text}"),
                            Err(e) => {
                                eprintln!("{model}: {e}");
                                std::process::exit(2);
                            }
                        }
                    }
                    None => {
                        eprintln!("microphone captured too little audio");
                        std::process::exit(2);
                    }
                }
            }
            Err(e) => {
                eprintln!("microphone: {e}");
                std::process::exit(2);
            }
        }
        return Ok(());
    }
    if args.iter().any(|a| a == "--diagnose") {
        println!("flowline {}", env!("CARGO_PKG_VERSION"));
        println!("accessibility (paste): {}", platform::accessibility_granted());
        println!(
            "input monitoring (hotkey): {}",
            platform::input_monitoring_granted()
        );
        println!("microphone: {}", platform::mic_authorized());
        println!(
            "frontmost app: {}",
            platform::frontmost_app().unwrap_or_else(|| "?".into())
        );
        println!(
            "clipboard: {}",
            if platform::clipboard_available() { "ok" } else { "unavailable" }
        );
        let (ready, detail) = engine::local_ready("");
        println!(
            "on-device whisper: {} ({})",
            if ready { "ready" } else { "missing" },
            detail
        );
        return Ok(());
    }
    // One instance per login session: the login item is allowed to start while
    // the person already has Flowline open, and two hubs would fight over
    // state.json (last writer wins).
    let _instance_lock = match single_instance_lock() {
        InstanceLock::Held(lock) => Some(lock),
        InstanceLock::Unavailable => None,
        InstanceLock::Duplicate => {
            // With no tray item to click, launching again is how the window is
            // reopened on Linux: ask the running copy to show itself.
            if platform::signal_running_instance() {
                eprintln!("Flowline is already running; showing its window.");
            } else {
                eprintln!("Flowline is already running.");
            }
            return Ok(());
        }
    };
    let mut shot = None;
    if let Some(i) = args.iter().position(|a| a == "--screenshot") {
        if let Some(p) = args.get(i + 1) {
            shot = Some(PathBuf::from(p));
        }
    }
    let start_hub = args.iter().any(|a| a == "--hub");
    let dark = args.iter().any(|a| a == "--dark");
    let modal_arg = args
        .iter()
        .position(|a| a == "--modal")
        .and_then(|i| args.get(i + 1))
        .cloned();
    let settings_tab_arg = args
        .iter()
        .position(|a| a == "--settings-tab")
        .and_then(|i| args.get(i + 1))
        .cloned()
        .unwrap_or_else(|| "general".into());
    let start_page = args
        .iter()
        .position(|a| a == "--page")
        .and_then(|i| args.get(i + 1))
        .map(|s| s.as_str())
        .unwrap_or("home");
    let background = args.iter().any(|a| a == "--background");
    let mut viewport = egui::ViewportBuilder::default()
        .with_inner_size([1350.0, 850.0])
        .with_min_inner_size([1120.0, 720.0])
        .with_title("Flowline")
        .with_visible(!background);
    if cfg!(target_os = "macos") {
        // Custom window chrome: Flowline draws its own title area and close button.
        viewport = viewport.with_decorations(false).with_transparent(true);
    } else {
        // Linux desktops draw the title bar, borders and resize handles, and
        // `app_id` ties the window to flowline.desktop (icon, dock grouping).
        viewport = viewport.with_app_id("flowline");
        if let Some(icon) = window_icon() {
            viewport = viewport.with_icon(icon);
        }
    }
    let options = eframe::NativeOptions {
        viewport,
        // A vsynced swap can block forever on a window the compositor has
        // hidden; Flowline must keep handling its shortcuts then.
        vsync: cfg!(target_os = "macos"),
        ..Default::default()
    };
    eframe::run_native(
        "Flowline",
        options,
        Box::new(move |cc| {
            theme::install_fonts(&cc.egui_ctx);
            Ok(Box::new(App::new(
                cc,
                shot.clone(),
                start_hub,
                start_page,
                settings_tab_arg.clone(),
                dark,
                modal_arg.clone(),
            )))
        }),
    )
}

impl App {
    fn new(
        cc: &eframe::CreationContext<'_>,
        shot: Option<PathBuf>,
        start_hub: bool,
        start_page: &str,
        settings_tab: String,
        dark: bool,
        modal_arg: Option<String>,
    ) -> Self {
        let mut state = State::load();
        // Only an empty endpoint falls back to the default. A remote endpoint
        // is a deliberate choice (e.g. a Tailscale Ollama host) and is kept.
        if state.settings.engine_endpoint.trim().is_empty() {
            state.settings.engine_endpoint = engine::DEFAULT_BASE.into();
        }
        if state.settings.model.trim().is_empty() {
            state.settings.model = engine::DEFAULT_MODEL.into();
        }
        if dark {
            state.settings.dark = true;
        }
        let pal = Palette {
            dark: state.settings.dark,
        };
        theme::apply_style(&cc.egui_ctx, pal);
        let state_model = engine::requested_model(&state.settings.model);
        let mut app = Self {
            screen: if start_hub {
                Screen::Hub
            } else {
                Screen::Welcome
            },
            page: match start_page {
                "dictionary" => Page::Dictionary,
                "snippets" => Page::Snippets,
                "style" => Page::Style,
                "transforms" => Page::Transforms,
                "notetaker" => Page::Notetaker,
                "myvoice" => Page::MyVoice,
                "scratchpad" => Page::Scratchpad,
                "insights" => Page::Insights,
                "settings" => Page::Settings,
                _ => Page::Home,
            },
            settings_tab: match settings_tab.as_str() {
                "system" => SettingsTab::System,
                "data" => SettingsTab::Data,
                "account" => SettingsTab::Account,
                "about" => SettingsTab::About,
                _ => SettingsTab::General,
            },
            state,
            search: String::new(),
            // Inspection only: FLOWLINE_STYLE_TAB=5 opens the Apps tab.
            style_tab: std::env::var("FLOWLINE_STYLE_TAB")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(0),
            modal: match modal_arg.as_deref() {
                Some("add-word") => Some(Modal::AddWord),
                Some("add-snippet") => Some(Modal::AddSnippet),
                Some("delete-word") => Some(Modal::DeleteWord(0)),
                _ => None,
            },
            form1: String::new(),
            form2: String::new(),
            toast: None,
            dict: Phase::Idle,
            last_pasted: None,
            frames: 0,
            shot,
            shot_done: false,
            start_time: Instant::now(),
            last_ptt_press: None,
            stop_at: None,
            mute_at: None,
            unmute_at: None,
            muted_by_us: false,
            rec: None,
            mic_error: None,
            engine: engine::Discovery {
                base: engine::DEFAULT_BASE.into(),
                models: Vec::new(),
                problem: Some("checking…".into()),
                reachable: false,
                listed: false,
            },
            engine_checking: true,
            events: None,
            model_menu: false,
            active_model: state_model.clone(),
            from_model: false,
            last_audio: None,
            global_target: None,
            clipboard_restore: None,
            quitting: false,
            hotkey_denied_note: false,
            overlay_actions: std::sync::Arc::new(std::sync::Mutex::new(Vec::new())),
            asked_accessibility: false,
            hidden_to_tray: std::env::args().any(|a| a == "--background"),
            meter: None,
            smooth_bands: [0.0; 7],
            overlay_smooth: std::sync::Arc::new(std::sync::Mutex::new([0.0; 7])),
            // `--no-permissions` skips the first-run sheet (screenshots, CI).
            show_permissions: (!platform::accessibility_granted()
                && !std::env::args().any(|a| a == "--no-permissions"))
                || std::env::args().any(|a| a == "--show-permissions"),
            permissions_forced: std::env::args().any(|a| a == "--show-permissions"),
            popup_placed: false,
            popup_origin: std::sync::Arc::new(std::sync::Mutex::new(None)),
            pending_permissions_prompt: false,
            notes: notes::NoteStore::load(),
            ext: features::Extras::default(),
        };
        native_overlay::install();
        platform::set_hotkey_mode(&app.state.settings.ptt);
        platform::set_dock_visible(app.state.settings.show_in_dock);
        // A login item only makes sense for the bundled app: installing one
        // from a cargo target would point LaunchAgents at a temp build.
        let bundled = std::env::current_exe()
            .map(|p| p.to_string_lossy().contains(".app/Contents/MacOS/"))
            .unwrap_or(false);
        if bundled
            && app.state.settings.launch_at_login
            && !platform::launch_agent_installed()
        {
            let _ = platform::set_launch_at_login(true);
        }
        platform::install_status_item(cc.egui_ctx.clone());
        platform::ensure_hotkey_monitor(cc.egui_ctx.clone());
        platform::start_hid_keyboard_service();
        meeting::start(platform::wake);
        app.probe_engine();
        if std::env::args().any(|a| a == "--model-menu") {
            app.model_menu = true;
        }
        app
    }

    pub fn toast(&mut self, text: String) {
        self.toast = Some((text, Instant::now()));
    }

    pub fn save(&self) {
        self.state.save();
    }

    /// Ask the local model server what it is serving, off the UI thread.
    /// Smoothed bands for drawing at display rate.
    pub fn display_bands(&self) -> [f32; 7] {
        self.smooth_bands
    }

    /// Soft speech barely moves the meter, so quiet-speech mode draws it larger.
    pub fn meter_gain(&self) -> f32 {
        if self.state.settings.quiet_mode { 4.0 } else { 1.0 }
    }

    /// The seven live voice bands, or silence when nothing is recording.
    pub fn bands(&self) -> [f32; 7] {
        self.meter
            .as_ref()
            .and_then(|m| m.lock().ok().map(|b| *b))
            .unwrap_or([0f32; 7])
    }

    fn engine_display_name(&self) -> String {
        if self.uses_local_engine() {
            engine::local_model_file(&self.state.settings.local_model)
                .ok()
                .and_then(|p| p.file_stem().map(|n| n.to_string_lossy().into_owned()))
                .unwrap_or_else(|| "Whisper".into())
        } else {
            engine::requested_model(&self.state.settings.model)
        }
    }

    fn overlay_position(&self, ctx: &egui::Context) -> (f64, f64) {
        let screen = ctx
            .input(|i| i.viewport().monitor_size)
            .unwrap_or(egui::vec2(1440.0, 900.0));
        let w = native_overlay::WIDTH;
        let h = native_overlay::HEIGHT;
        // NSWindow frame origin is bottom-left.
        match self.state.settings.popup_position.as_str() {
            "bottom-right" => (screen.x as f64 - w - 24.0, 36.0),
            "top-center" => ((screen.x as f64 - w) / 2.0, screen.y as f64 - h - 40.0),
            "custom"
                if self.state.settings.popup_x != 0.0 || self.state.settings.popup_y != 0.0 =>
            {
                (
                    self.state.settings.popup_x as f64,
                    self.state.settings.popup_y as f64,
                )
            }
            _ => ((screen.x as f64 - w) / 2.0, 36.0),
        }
    }

    fn overlay_model(&self) -> native_overlay::Model {
        let mut model = native_overlay::Model {
            label: self.engine_display_name(),
            bands: self.smooth_bands,
            ..Default::default()
        };
        match &self.dict {
            Phase::Idle => model.kind = 0,
            Phase::Listening { started, .. } => {
                model.kind = 1;
                model.elapsed = started.elapsed().as_secs_f32();
            }
            Phase::Transcribing { secs, .. } => {
                model.kind = 2;
                model.elapsed = *secs;
            }
            Phase::Done {
                text,
                pasted,
                copied,
                head,
                ..
            } => {
                model.kind = if *pasted { 3 } else { 4 };
                model.head = head.clone().unwrap_or_default();
                model.text = text.clone();
                model.copied = *copied;
            }
        }
        model
    }

    pub fn uses_local_engine(&self) -> bool {
        self.state.settings.engine != "server"
    }

    pub fn probe_engine(&mut self) {
        self.engine_checking = true;
        if self.uses_local_engine() {
            let model = self.state.settings.local_model.clone();
            let (ready, detail) = engine::local_ready(&model);
            self.engine = engine::Discovery {
                base: "local".into(),
                models: engine::local_models(),
                problem: if ready { None } else { Some(detail.clone()) },
                reachable: ready,
                listed: true,
            };
            self.engine_checking = false;
            return;
        }
        // Never show a previous server's models while this probe runs.
        self.engine.models.clear();
        self.engine.base = self.state.settings.engine_endpoint.clone();
        let (tx, rx) = std::sync::mpsc::channel();
        self.events = Some(rx);
        let base = self.state.settings.engine_endpoint.clone();
        let model = self.state.settings.model.clone();
        std::thread::spawn(move || {
            let found = engine::discover(&base, &model);
            let _ = tx.send(engine::Event::Probe(found));
        });
    }

    /// Send one captured clip to the selected model and wait in the background.
    pub fn start_transcribing(&mut self, wav: Vec<u8>, secs: f32) {
        let language = engine::language_code(&self.state.settings.language).to_string();
        let local = self.uses_local_engine();
        let base = engine::requested_base(&self.state.settings.engine_endpoint);
        let model = engine::requested_model(&self.state.settings.model);
        let local_model = self.state.settings.local_model.clone();
        let quiet = self.state.settings.quiet_mode;
        let label = if local {
            engine::local_model_file(&local_model)
                .ok()
                .and_then(|p| p.file_stem().map(|s| s.to_string_lossy().into_owned()))
                .unwrap_or_else(|| "Whisper".into())
        } else {
            model.clone()
        };
        let (tx, rx) = std::sync::mpsc::channel();
        self.events = Some(rx);
        self.from_model = false;
        self.active_model = label.clone();
        self.last_audio = Some((wav.clone(), secs));
        self.dict = Phase::Transcribing {
            started: Instant::now(),
            raw: String::new(),
            secs,
        };
        std::thread::spawn(move || {
            let result = if local {
                engine::transcribe_local_opts(&wav, &local_model, &language, quiet)
            } else {
                engine::transcribe(&base, &model, &wav, &language)
            };
            let _ = tx.send(engine::Event::Transcript {
                result,
                secs,
                model: label,
            });
        });
    }

    /// Transcribe a WAV the person picked (or dropped on the window).
    pub fn transcribe_path(&mut self, path: &std::path::Path) {
        match std::fs::read(path) {
            Ok(bytes) if audio::is_wav(&bytes) => {
                let secs = audio::wav_seconds(&bytes);
                self.page = Page::Scratchpad;
                self.start_transcribing(bytes, secs);
            }
            Ok(_) => self.toast("That file is not a WAV recording".into()),
            Err(e) => self.toast(format!("Could not read that file: {e}")),
        }
    }

    /// Which Style tab applies to the dictation being finished: the app it is
    /// going into (when "Match tone to the app" is on), else the old default.
    fn tone_tab(&self) -> usize {
        let legacy = if self.page == Page::Scratchpad { 4 } else { 2 };
        if !self.state.tone_by_app {
            return legacy;
        }
        let Some(app) = self.global_target.as_deref() else {
            return legacy;
        };
        let overridden = self
            .state
            .app_tones
            .iter()
            .any(|r| r.app.eq_ignore_ascii_case(app.trim()));
        let title = if !overridden && is_browser(app) {
            platform::front_window_title()
        } else {
            None
        };
        self.state.tab_for_app(app, title.as_deref())
    }

    pub fn style_for_tab(&self, tab: usize) -> Style {
        match tab {
            1 => self.state.work_style,
            2 => self.state.email_style,
            3 => self.state.personal_style,
            _ => self.state.other_style,
        }
    }

    pub fn set_style_for_tab(&mut self, tab: usize, s: Style) {
        match tab {
            1 => self.state.work_style = s,
            2 => self.state.email_style = s,
            3 => self.state.personal_style = s,
            _ => self.state.other_style = s,
        }
    }

    pub fn start_dictation(&mut self) {
        self.start_dictation_with(false);
    }

    /// A separate always-on-top, non-activating panel for the dictation pill,
    /// mirroring the original's NSPanel. It keeps working when the hub is
    /// hidden in the menu bar and never steals focus from the app being dictated
    /// into.
    fn show_overlay(&mut self, ctx: &egui::Context) {
        if matches!(self.dict, Phase::Idle) {
            return;
        }
        let phase = self.dict.clone();
        let model = if self.uses_local_engine() {
            engine::local_model_file(&self.state.settings.local_model)
                .ok()
                .and_then(|p| p.file_stem().map(|s| s.to_string_lossy().into_owned()))
                .unwrap_or_else(|| "Whisper".into())
        } else {
            engine::requested_model(&self.state.settings.model)
        };
        let screen = ctx
            .input(|i| i.viewport().monitor_size)
            .unwrap_or(egui::vec2(1440.0, 900.0));
        let size = egui::vec2(470.0, 176.0);
        let pos = match self.state.settings.popup_position.as_str() {
            "bottom-right" => egui::pos2(screen.x - size.x - 24.0, screen.y - size.y - 36.0),
            "top-center" => egui::pos2((screen.x - size.x) * 0.5, 40.0),
            "custom" if self.state.settings.popup_x != 0.0 || self.state.settings.popup_y != 0.0 => {
                egui::pos2(self.state.settings.popup_x, self.state.settings.popup_y)
            }
            _ => egui::pos2((screen.x - size.x) * 0.5, screen.y - size.y - 36.0),
        };
        let viewport_id = egui::ViewportId::from_hash_of("flowline-dictation-overlay");
        let builder = egui::ViewportBuilder::default()
            .with_title("Flowline Dictation")
            .with_inner_size(size)
            .with_position(pos)
            .with_decorations(false)
            .with_transparent(true)
            .with_always_on_top()
            .with_resizable(false)
            .with_active(false);
        let actions = self.overlay_actions.clone();
        let pal = Palette { dark: true };
        // Capture the clock, not a timestamp: the child viewport keeps
        // repainting even when the hub is hidden and stops receiving updates.
        let clock = self.start_time;
        let meter = self.meter.clone();
        let origin = self.popup_origin.clone();
        let draggable = self.state.settings.popup_position == "custom";
        let smooth = self.overlay_smooth.clone();
        let meter_gain = self.meter_gain();
        if !self.popup_placed {
            ctx.send_viewport_cmd_to(viewport_id, egui::ViewportCommand::OuterPosition(pos));
            self.popup_placed = true;
        }
        ctx.show_viewport_deferred(
            viewport_id,
            builder,
            move |ctx, _class| {
                egui::CentralPanel::default()
                    .frame(egui::Frame::NONE.fill(egui::Color32::TRANSPARENT))
                    .show(ctx, |ui| {
                        let time = clock.elapsed().as_secs_f32();
                        let target = meter
                            .as_ref()
                            .and_then(|m| m.lock().ok().map(|b| *b))
                            .unwrap_or([0f32; 7])
                            .map(|v| (v * meter_gain).min(1.0));
                        let bands = smooth
                            .lock()
                            .map(|mut s| {
                                for i in 0..7 {
                                    s[i] += (target[i] - s[i]) * 0.32;
                                }
                                *s
                            })
                            .unwrap_or(target);
                        if let Some(rect) = ui.input(|i| i.viewport().outer_rect) {
                            if let Ok(mut o) = origin.lock() {
                                *o = Some((rect.min.x, rect.min.y));
                            }
                        }
                        let action = dictation::overlay(
                            ui, pal, &phase, time, &bands, &model, draggable,
                        );
                        let mut action = action;
                        if ui.input(|i| i.key_pressed(egui::Key::Escape)) {
                            action = dictation::OverlayAction::Cancel;
                        }
                        if !matches!(action, dictation::OverlayAction::None) {
                            if let Ok(mut q) = actions.lock() {
                                q.push(action);
                            }
                        }
                    });
                if matches!(
                    phase,
                    Phase::Listening { .. } | Phase::Transcribing { .. }
                ) {
                    ctx.request_repaint_after(Duration::from_millis(16));
                }
            },
        );
    }

    fn show_window(&mut self, ctx: &egui::Context) {
        self.hidden_to_tray = false;
        ctx.send_viewport_cmd(egui::ViewportCommand::Visible(true));
        ctx.send_viewport_cmd(egui::ViewportCommand::Minimized(false));
        ctx.send_viewport_cmd(egui::ViewportCommand::Focus);
        platform::activate_app();
    }

    /// A dictation started from the global shortcut: remember which app was
    /// focused so the transcript can be pasted back into it.
    fn start_global_dictation(&mut self) {
        if !matches!(self.dict, Phase::Idle | Phase::Done { .. }) {
            return;
        }
        let front = platform::frontmost_app();
        let us = matches!(front.as_deref(), Some("Flowline") | Some("flowline"));
        self.global_target = if us { None } else { front };
        if std::env::var_os("FLOWLINE_DEBUG").is_some() {
            eprintln!("dictation: target={:?}", self.global_target);
        }
        if !platform::accessibility_granted() && !self.asked_accessibility {
            self.asked_accessibility = true;
            platform::request_accessibility();
        }
        self.start_dictation_with(false);
    }

    pub fn start_dictation_with(&mut self, hands_free: bool) {
        if matches!(self.dict, Phase::Idle | Phase::Done { .. }) {
            self.stop_at = None;
            // A restart cancels a pending post-stop restore, and the start cue
            // must be audible, so restore first and re-mute 300 ms later.
            self.restore_media_now();
            self.dict = Phase::Listening {
                started: Instant::now(),
                hands_free,
            };
            self.rec = match audio::Recorder::start(Some(&self.state.settings.microphone)) {
                Ok(r) => {
                    self.mic_error = None;
                    self.meter = Some(r.meter());
                    Some(r)
                }
                Err(e) => {
                    let short: String = e.chars().take(70).collect();
                    self.toast(format!("Microphone: {short}"));
                    self.mic_error = Some(e);
                    self.meter = None;
                    None
                }
            };
            self.popup_placed = false;
            self.smooth_bands = [0.0; 7];
            if let Ok(mut s) = self.overlay_smooth.lock() {
                *s = [0.0; 7];
            }
            self.cue(true);
            // Play the start cue first, then silence other audio after
            // MUTE_AFTER_START_SOUND_MS.
            if self.state.settings.mute_media {
                self.mute_at = Some(
                    Instant::now() + Duration::from_secs_f32(MUTE_AFTER_START_SOUND_MS / 1000.0),
                );
            }
        }
    }

    #[cfg(target_os = "linux")]
    fn output_is_muted() -> Option<bool> {
        let out = std::process::Command::new("pactl")
            .args(["get-sink-mute", "@DEFAULT_SINK@"])
            .output()
            .ok()?;
        if !out.status.success() {
            return None;
        }
        let text = String::from_utf8_lossy(&out.stdout).to_lowercase();
        if text.contains("yes") {
            Some(true)
        } else if text.contains("no") {
            Some(false)
        } else {
            None
        }
    }

    #[cfg(target_os = "linux")]
    fn set_output_muted(muted: bool) {
        let _ = std::process::Command::new("pactl")
            .args(["set-sink-mute", "@DEFAULT_SINK@", if muted { "1" } else { "0" }])
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .spawn();
    }

    #[cfg(target_os = "macos")]
    fn output_is_muted() -> Option<bool> {
        let out = std::process::Command::new("osascript")
            .args(["-e", "output muted of (get volume settings)"])
            .output()
            .ok()?;
        if !out.status.success() {
            return None;
        }
        match String::from_utf8_lossy(&out.stdout).trim() {
            "true" => Some(true),
            "false" => Some(false),
            _ => None,
        }
    }

    #[cfg(target_os = "macos")]
    fn set_output_muted(muted: bool) {
        let _ = std::process::Command::new("osascript")
            .args(["-e", &format!("set volume output muted {muted}")])
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .spawn();
    }

    /// Silence other audio, but never unmute a Mac the person muted themselves.
    fn begin_media_silence(&mut self) {
        if self.muted_by_us {
            return;
        }
        if Self::output_is_muted() == Some(false) {
            Self::set_output_muted(true);
            self.muted_by_us = true;
        }
    }

    /// Wait for the output device to settle before unmuting. A single process
    /// cannot read the device's stability flag, so a fixed 500 ms interval is
    /// used.
    fn end_media_silence(&mut self) {
        self.mute_at = None;
        if self.muted_by_us {
            self.unmute_at =
                Some(Instant::now() + Duration::from_secs_f32(AUDIO_RESTORE_DELAY_MS / 1000.0));
        }
    }

    fn restore_media_now(&mut self) {
        self.mute_at = None;
        self.unmute_at = None;
        if self.muted_by_us {
            Self::set_output_muted(false);
            self.muted_by_us = false;
        }
    }

    fn min_hold_secs(&self) -> f32 {
        if self.page == Page::Scratchpad {
            SCRATCHPAD_PTT_DELAY_MS / 1000.0
        } else {
            DICTATION_DEBOUNCE_MS / 1000.0
        }
    }

    pub fn stop_dictation(&mut self) {
        if let Phase::Listening { started, .. } = &self.dict {
            let elapsed = started.elapsed().as_secs_f32();
            if elapsed < self.min_hold_secs() {
                self.dict = Phase::Idle;
                self.global_target = None;
                self.end_media_silence();
                self.toast("Too short — hold a little longer".into());
                return;
            }
            let secs = elapsed.min(MAX_DICTATION_SECS);
            // Anchor for the stop + 2.0 s overlay linger.
            self.stop_at = Some(Instant::now());
            let wav = self.rec.take().and_then(|r| r.finish());
            self.meter = None;
            if self.global_target.is_some() && self.state.settings.ptt == "Fn" {
                platform::dismiss_search_overlay();
            }
            self.cue(false);
            self.end_media_silence();
            let quiet = self.state.settings.quiet_mode;
            let floor = if quiet { audio::QUIET_MIN_PEAK } else { audio::NORMAL_MIN_PEAK };
            match wav {
                Some(bytes) if bytes.len() > 44 && audio::wav_peak(&bytes) >= floor => {
                    // Quiet-speech mode: bring soft speech up to a normal level
                    // so the engine can hear it. The boosted clip is what is
                    // kept for "Retry audio".
                    let bytes = if quiet { audio::boost_quiet(&bytes).0 } else { bytes };
                    self.start_transcribing(bytes, secs)
                }
                _ => {
                    self.dict = Phase::Idle;
                    self.global_target = None;
                    self.toast("No speech detected — hold the shortcut and speak".into());
                }
            }
        }
    }

    pub fn cancel_dictation(&mut self) {
        self.rec = None;
        self.meter = None;
        self.stop_at = None;
        self.global_target = None;
        if matches!(self.dict, Phase::Listening { .. }) {
            self.dict = Phase::Idle;
            self.end_media_silence();
            self.toast("Dictation cancelled".into());
        }
        if matches!(self.dict, Phase::Done { .. }) {
            self.dict = Phase::Idle;
        }
    }

    fn cue(&self, start: bool) {
        if !self.state.settings.sounds {
            return;
        }
        #[cfg(target_os = "macos")]
        {
            let file = if start {
                "/System/Library/Sounds/Tink.aiff"
            } else {
                "/System/Library/Sounds/Pop.aiff"
            };
            let _ = std::process::Command::new("afplay").arg(file).spawn();
        }
        #[cfg(target_os = "linux")]
        {
            // The freedesktop sound theme, played through PipeWire/PulseAudio
            // when both the player and the sound are there.
            let file = if start {
                "/usr/share/sounds/freedesktop/stereo/message.oga"
            } else {
                "/usr/share/sounds/freedesktop/stereo/complete.oga"
            };
            if std::path::Path::new(file).is_file() {
                let _ = std::process::Command::new("paplay").arg(file).spawn();
            }
        }
    }

    fn finish_transcription(&mut self) {
        let (raw, secs) = match &self.dict {
            Phase::Transcribing { raw, secs, .. } => (raw.clone(), *secs),
            _ => return,
        };
        // A spoken command: the transcript is an instruction for the selection,
        // not text to paste.
        if self.ext.cmd_active {
            let _ = self.finish_command(&raw);
            return;
        }
        let engine = if self.from_model {
            self.active_model.clone()
        } else {
            "sample".to_string()
        };
        let with_snippets = apply_snippets(&raw, &self.state.snippets);
        let with_dict = apply_dictionary(&with_snippets, &self.state.dictionary);
        // Work out the tone once per dictation: the finish step can run for
        // many frames while an auto transform is pending.
        let tab = match self.ext.tone_tab {
            Some(t) => t,
            None => {
                let t = self.tone_tab();
                self.ext.tone_tab = Some(t);
                t
            }
        };
        let style = self.style_for_tab(tab);
        let cleaned = cleanup_text(&with_dict, self.state.cleanup, style);
        // Transforms: an opted-in auto transform rewrites the text before it is
        // pasted. The pill stays on "transcribing" until the model answers.
        let cleaned = match self.auto_transform_gate(cleaned) {
            features::Gate::Proceed(text) => {
                self.ext.tone_tab = None;
                text
            }
            features::Gate::Wait => return,
        };
        // A global dictation belongs in the app the person was using, not in
        // our Scratchpad. Only real transcripts are pasted — a labelled sample
        // from a missing model must never land in someone's editor.
        let mut pasted_into: Option<String> = None;
        let mut kept_reason: Option<String> = None;
        let mut pasted = false;
        let mut show_copy = false;
        if self.global_target.take().is_some() {
            let front = platform::frontmost_app();
            let our_own = matches!(front.as_deref(), Some("Flowline") | Some("flowline"));
            let trusted = platform::accessibility_granted();
            // Some is a real answer, None means "this app hides focus": paste
            // anyway, as chosen in Settings.
            let focus = platform::focused_text_input();
            if std::env::var_os("FLOWLINE_DEBUG").is_some() {
                eprintln!(
                    "finish: front={front:?} trusted={trusted} focus={focus:?} cleaned={}",
                    cleaned.len()
                );
            }
            if !cleaned.is_empty() {
                if !trusted {
                    if !self.asked_accessibility {
                        self.asked_accessibility = true;
                        platform::request_accessibility();
                    }
                    platform::copy_text(&cleaned);
                    show_copy = true;
                    kept_reason =
                        Some("Accessibility is off — transcript copied to the clipboard".into());
                    self.pending_permissions_prompt = true;
                } else if our_own || focus == Some(false) {
                    platform::copy_text(&cleaned);
                    show_copy = true;
                    kept_reason = Some("No text field focused — copied to the clipboard".into());
                } else {
                    self.clipboard_restore = Some((
                        platform::paste_transcript(&cleaned),
                        Instant::now() + Duration::from_millis(900),
                    ));
                    pasted = true;
                    pasted_into = front.clone();
                }
            }
        }
        if pasted {
            self.start_learning(cleaned.clone());
        }
        if !cleaned.is_empty() {
            if !self.state.note.is_empty() && !self.state.note.ends_with('\n') {
                self.state.note.push('\n');
            }
            self.state.note.push_str(&cleaned);
            self.state.note.push('\n');
            let words = cleaned.split_whitespace().count();
            let at = chrono::Local::now().format("%-I:%M %p").to_string();
            self.state.history.push(Dictation {
                text: cleaned.clone(),
                raw,
                app: pasted_into.clone().unwrap_or_else(|| "Scratchpad".into()),
                at,
                ts: chrono::Local::now().timestamp(),
                seconds: secs,
                words,
                engine,
            });
            self.last_pasted = Some(cleaned.clone());
            let label = self.active_model.clone();
            match (&pasted_into, &kept_reason) {
                (Some(target), _) => self.toast(format!(
                    "Pasted into {target} · {}",
                    cleaned.chars().take(40).collect::<String>()
                )),
                (None, Some(reason)) => self.toast(reason.clone()),
                (None, None) => self.toast(format!(
                    "{label} · {}",
                    cleaned.chars().take(40).collect::<String>()
                )),
            }
            self.save();
        } else if let Some(reason) = kept_reason {
            self.toast(reason);
        }
        self.from_model = false;
        self.stop_at = None;
        if self.state.settings.popup_position == "custom" {
            if let Some((x, y)) = native_overlay::origin() {
                self.state.settings.popup_x = x as f32;
                self.state.settings.popup_y = y as f32;
                self.state.save();
            }
        }
        if cleaned.is_empty() {
            self.dict = Phase::Idle;
            return;
        }
        let timeout = self.state.settings.popup_timeout_ms;
        let deadline = if pasted && timeout > 0 {
            Some(Instant::now() + Duration::from_millis(timeout))
        } else {
            None
        };
        self.dict = Phase::Done {
            deadline,
            text: cleaned,
            pasted,
            copied: show_copy,
            head: None,
        };
    }
}

impl Drop for App {
    fn drop(&mut self) {
        if self.muted_by_us {
            Self::set_output_muted(false);
        }
    }
}

impl eframe::App for App {
    fn clear_color(&self, _visuals: &egui::Visuals) -> [f32; 4] {
        // Lets the dictation overlay window be transparent around the pill.
        [0.0, 0.0, 0.0, 0.0]
    }

    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let pal = Palette {
            dark: self.state.settings.dark,
        };
        theme::apply_style(ctx, pal);
        let time = self.start_time.elapsed().as_secs_f32();

        // Menu bar and the global hold-to-talk shortcut.
        if platform::take_quit_request() {
            self.quitting = true;
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
        }
        if platform::take_show_request() {
            self.show_window(ctx);
        }
        if platform::take_permissions_request() {
            self.show_permissions = true;
            self.show_window(ctx);
        }
        let focused = ctx.input(|i| i.viewport().focused) == Some(true);
        if self.frames % 30 == 0 || (self.show_permissions && focused) {
            if !platform::hotkey_running() && platform::accessibility_granted() {
                platform::ensure_hotkey_monitor(ctx.clone());
            }
            platform::start_hid_keyboard_service();
            if platform::accessibility_granted() && !platform::hotkey_active() {
                platform::upgrade_hotkey_tap(ctx.clone());
            }
            if self.show_permissions
                && platform::accessibility_granted()
                && platform::hotkey_active()
                && !self.permissions_forced
            {
                self.show_permissions = false;
                self.toast("Accessibility enabled — dictate anywhere with Fn".into());
            }
        }
        if cfg!(not(target_os = "macos"))
            && ctx.input(|i| i.modifiers.command && i.key_pressed(egui::Key::Q))
        {
            self.quitting = true;
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
        }
        if ctx.input(|i| i.viewport().close_requested()) && !self.quitting {
            // Closing the hub leaves Flowline running in the menu bar so Fn
            // keeps working. Quit is explicit, from the menu.
            ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
            ctx.send_viewport_cmd(egui::ViewportCommand::Visible(false));
            if cfg!(not(target_os = "macos")) {
                // Wayland cannot hide a window, so also minimize it.
                ctx.send_viewport_cmd(egui::ViewportCommand::Minimized(true));
            }
            self.hidden_to_tray = true;
        }
        for event in platform::take_events() {
            match event {
                platform::HotkeyEvent::Start => self.start_global_dictation(),
                platform::HotkeyEvent::Stop => self.stop_dictation(),
                platform::HotkeyEvent::Cancel => self.cancel_dictation(),
                platform::HotkeyEvent::CommandStart => self.start_command(),
                platform::HotkeyEvent::CommandStop => self.stop_dictation(),
                platform::HotkeyEvent::Transform(n) => self.selection_transform(n as usize),
                platform::HotkeyEvent::Denied => {
                    if !self.hotkey_denied_note {
                        self.hotkey_denied_note = true;
                        platform::request_accessibility();
                        self.toast(platform::denied_hint().into());
                    }
                }
            }
        }
        if std::env::var_os("FLOWLINE_DEBUG").is_some()
            && matches!(self.dict, Phase::Listening { .. })
            && self.frames % 20 == 0
        {
            eprintln!(
                "bands {:?} peak {:.3}",
                self.bands().map(|v| (v * 100.0).round() / 100.0),
                self.rec.as_ref().map(|r| r.level()).unwrap_or(0.0)
            );
        }
        if matches!(self.dict, Phase::Listening { .. }) {
            let gain = self.meter_gain();
            let target = self.bands().map(|v| (v * gain).min(1.0));
            for i in 0..7 {
                let s = self.smooth_bands[i];
                self.smooth_bands[i] = s + (target[i] - s) * 0.30;
            }
        } else {
            for v in &mut self.smooth_bands {
                *v *= 0.65;
                if *v < 0.002 {
                    *v = 0.0;
                }
            }
        }
        if self.dict.is_active() && ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
            self.cancel_dictation();
        }
        if self.pending_permissions_prompt {
            self.pending_permissions_prompt = false;
            self.show_permissions = true;
            self.show_window(ctx);
        }
        if let Some((_, at)) = &self.clipboard_restore {
            if Instant::now() >= *at {
                if let Some((restore, _)) = self.clipboard_restore.take() {
                    platform::restore_clipboard(restore);
                }
            }
        }

        self.tick_features(ctx);

        // Answers from the local model thread.
        let mut notes = Vec::new();
        if let Some(rx) = &self.events {
            while let Ok(e) = rx.try_recv() {
                notes.push(e);
            }
        }
        for event in notes {
            match event {
                engine::Event::Probe(found) => {
                    // The typed endpoint is authoritative; probe results never
                    // rewrite it or point at a different server.
                    self.engine = found;
                    self.engine_checking = false;
                }
                engine::Event::Transcript {
                    result,
                    secs,
                    model,
                } => {
                    let waiting =
                        matches!(&self.dict, Phase::Transcribing { raw, .. } if raw.is_empty());
                    if !waiting {
                        continue;
                    }
                    match result {
                        Ok(text) if !text.trim().is_empty() => {
                            self.from_model = true;
                            self.active_model = model;
                            self.last_audio = None;
                            self.dict = Phase::Transcribing {
                                started: Instant::now(),
                                raw: text,
                                secs,
                            };
                        }
                        Ok(_) => {
                            self.dict = Phase::Idle;
                            self.global_target = None;
                            self.toast(format!("{model} heard no speech"));
                        }
                        Err(e) => {
                            let short: String = e.chars().take(90).collect();
                            self.dict = Phase::Idle;
                            self.global_target = None;
                            self.toast(format!("{model} failed: {short} — audio kept for retry"));
                        }
                    }
                }
            }
        }

        // A WAV dropped on the window is transcribed by the selected model.
        let dropped: Vec<std::path::PathBuf> = ctx.input(|i| {
            i.raw
                .dropped_files
                .iter()
                .filter_map(|f| f.path.clone())
                .collect()
        });
        if let Some(path) = dropped.first() {
            self.transcribe_path(path);
        }

        // Silence other audio once the start cue has had its 300 ms.
        if let Some(at) = self.mute_at {
            if Instant::now() >= at {
                self.mute_at = None;
                self.begin_media_silence();
            }
        }
        // Restore the volume after the 500 ms codec-settle interval.
        if let Some(at) = self.unmute_at {
            if Instant::now() >= at {
                self.restore_media_now();
            }
        }

        // global shortcuts
        if ctx.input(|i| i.modifiers.command && i.key_pressed(egui::Key::Q)) {
            self.quitting = true;
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
        }
        let typing = ctx.wants_keyboard_input();
        let widget_focus = ctx.memory(|m| m.focused().is_some());
        if self.screen == Screen::Hub && !typing && self.modal.is_none() {
            let ctrl_space = ctx.input(|i| i.modifiers.ctrl && i.key_pressed(egui::Key::Space));
            let plain_space = ctx.input(|i| !i.modifiers.any() && i.key_pressed(egui::Key::Space));
            if ctrl_space || (plain_space && !widget_focus) {
                let is_double = self
                    .last_ptt_press
                    .map(|t| t.elapsed().as_millis() < DOUBLE_TAP_WINDOW_MS)
                    .unwrap_or(false);
                match &self.dict {
                    Phase::Idle | Phase::Done { .. } => {
                        let hands_free = is_double && self.state.settings.hands_free;
                        self.start_dictation_with(hands_free);
                    }
                    Phase::Listening {
                        hands_free: true, ..
                    } => {
                        self.stop_dictation();
                    }
                    _ => {}
                }
                self.last_ptt_press = Some(Instant::now());
            }
            if ctx.input(|i| i.key_released(egui::Key::Space)) {
                if let Phase::Listening { hands_free, .. } = &self.dict {
                    if !hands_free {
                        self.stop_dictation();
                    }
                }
            }
            if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
                self.cancel_dictation();
            }
            if let Phase::Listening { started, .. } = &self.dict {
                if started.elapsed().as_secs_f32() >= MAX_DICTATION_SECS {
                    self.stop_dictation();
                }
            }
        }

        let mut ready = false;
        let mut stalled: Option<f32> = None;
        if let Phase::Transcribing { started, raw, secs } = &self.dict {
            if raw.is_empty() {
                if started.elapsed() > Duration::from_secs(45) {
                    stalled = Some(*secs);
                }
            } else if started.elapsed() > Duration::from_millis(1150) {
                ready = true;
            }
        }
        if ready {
            self.finish_transcription();
        }
        if let Some(_secs) = stalled {
            let model = self.active_model.clone();
            self.dict = Phase::Idle;
            self.global_target = None;
            self.toast(format!("{model} timed out — audio kept for retry"));
        }
        // The original keeps the pill for 2.0 s after the stop timestamp.
        if let Phase::Done {
            deadline: Some(deadline),
            ..
        } = &self.dict
        {
            if Instant::now() >= *deadline {
                self.dict = Phase::Idle;
            }
        }

        egui::CentralPanel::default()
            .frame(egui::Frame::NONE.fill(pal.bg()))
            .show(ctx, |ui| {
                if self.show_permissions {
                    hub::permissions_sheet(ui, self, pal);
                } else {
                match self.screen {
                    Screen::Welcome => {
                        if welcome::show(ui, pal, time) {
                            self.screen = Screen::Hub;
                            self.page = Page::Home;
                        }
                        // drag the window by the blank top strip
                        let strip = egui::Rect::from_min_size(
                            ui.max_rect().min,
                            egui::vec2(ui.max_rect().width(), 44.0),
                        );
                        let resp =
                            ui.interact(strip, ui.id().with("drag-strip"), egui::Sense::drag());
                        if resp.drag_started() {
                            ctx.send_viewport_cmd(egui::ViewportCommand::StartDrag);
                        }
                    }
                    Screen::Hub => {
                        hub::show(ui, self, pal, time);
                        let w = ui.max_rect().width();
                        let strip =
                            egui::Rect::from_min_size(ui.max_rect().min, egui::vec2(w, 44.0));
                        let resp =
                            ui.interact(strip, ui.id().with("hub-drag"), egui::Sense::drag());
                        if resp.drag_started() {
                            ctx.send_viewport_cmd(egui::ViewportCommand::StartDrag);
                        }
                        let close = egui::Rect::from_center_size(
                            egui::pos2(ui.max_rect().right() - 26.0, 22.0),
                            egui::vec2(24.0, 24.0),
                        );
                        if close.contains(
                            ui.input(|i| i.pointer.hover_pos())
                                .unwrap_or(egui::pos2(-100.0, -100.0)),
                        ) {
                            ui.painter()
                                .circle_filled(close.center(), 11.0, pal.hover());
                        }
                        ui.painter().text(
                            close.center(),
                            egui::Align2::CENTER_CENTER,
                            "×",
                            egui::FontId::proportional(16.0),
                            pal.text_muted(),
                        );
                        if ui
                            .interact(close, ui.id().with("hub-close"), egui::Sense::click())
                            .clicked()
                        {
                            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                        }
                        // The pill lives in its own always-on-top viewport (see
                        // show_overlay) so it also appears while the hub is
                        // hidden in the menu bar.
                    }
                }
                }

                if let Some((text, at)) = &self.toast {
                    let t = at.elapsed().as_secs_f32();
                    if t < 2.4 {
                        let alpha = if t < 1.9 { 1.0 } else { (2.4 - t) / 0.5 };
                        dictation::toast(ui, pal, text, alpha.max(0.0));
                    } else {
                        self.toast = None;
                    }
                }
            });

        if self.dict.is_active() {
            if !self.popup_placed {
                let (x, y) = self.overlay_position(ctx);
                native_overlay::place(x, y);
                self.popup_placed = true;
            }
            native_overlay::show(self.overlay_model());
        } else {
            native_overlay::hide();
            self.popup_placed = false;
        }
        // Without the macOS AppKit panel, the pill is an egui overlay window.
        if cfg!(not(target_os = "macos")) {
            if self.dict.is_active() {
                self.show_overlay(ctx);
            }
            let clicks: Vec<dictation::OverlayAction> = self
                .overlay_actions
                .lock()
                .map(|mut q| std::mem::take(&mut *q))
                .unwrap_or_default();
            for action in clicks {
                match action {
                    OverlayAction::Stop => self.stop_dictation(),
                    OverlayAction::Cancel => self.cancel_dictation(),
                    OverlayAction::Copy => {
                        if let Phase::Done { text, copied, .. } = &mut self.dict {
                            platform::copy_text(text);
                            *copied = true;
                        }
                    }
                    OverlayAction::Dismiss => self.dict = Phase::Idle,
                    OverlayAction::None => {}
                }
            }
        }
        let native_actions = native_overlay::take_actions();
        if std::env::var_os("FLOWLINE_DEBUG").is_some() && !native_actions.is_empty() {
            eprintln!("app got {} native actions", native_actions.len());
        }
        for action in native_actions {
            match action {
                native_overlay::Action::Stop => self.stop_dictation(),
                native_overlay::Action::Copy => {
                    if let Phase::Done { text, copied, .. } = &mut self.dict {
                        platform::copy_text(text);
                        *copied = true;
                    }
                }
                native_overlay::Action::Dismiss => self.dict = Phase::Idle,
            }
        }

        self.frames += 1;
        if let Some(path) = self.shot.clone() {
            if self.frames >= 120 && !self.shot_done {
                ctx.send_viewport_cmd(egui::ViewportCommand::Screenshot(Default::default()));
                self.shot_done = true;
            }
            if self.shot_done {
                let shot = ctx.input(|i| {
                    i.events.iter().find_map(|e| match e {
                        egui::Event::Screenshot { image, .. } => Some(image.clone()),
                        _ => None,
                    })
                });
                if let Some(image) = shot {
                    let [w, h] = image.size;
                    let mut buf = Vec::with_capacity(w * h * 4);
                    for px in image.pixels.iter() {
                        buf.extend_from_slice(&px.to_array());
                    }
                    if let Some(img) = image::RgbaImage::from_raw(w as u32, h as u32, buf) {
                        if let Some(parent) = path.parent() {
                            let _ = std::fs::create_dir_all(parent);
                        }
                        let _ = img.save(&path);
                    }
                    // Without this the hide-to-menu-bar handler cancels the close.
                    self.quitting = true;
                    ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                }
            }
            ctx.request_repaint();
        } else if matches!(
            self.dict,
            Phase::Listening { .. } | Phase::Transcribing { .. } | Phase::Done { .. }
        ) {
            ctx.request_repaint_after(Duration::from_millis(16));
        } else if self.show_permissions {
            // Keep permission statuses live while the guide is open.
            ctx.request_repaint_after(Duration::from_millis(250));
        } else if !self.hidden_to_tray
            && (matches!(self.screen, Screen::Welcome)
                || (self.screen == Screen::Hub && self.page == Page::Home))
        {
            // The welcome mic pill and the Home hero waveform animate on their
            // own; without this they only repaint when the pointer moves.
            ctx.request_repaint_after(Duration::from_millis(40));
        }
    }
}
