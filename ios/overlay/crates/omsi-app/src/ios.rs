//! openOMSI on iOS: the game and its launcher in one window of one process, exactly as on
//! Android (see `android.rs`, which this module mirrors: a phone runs the same `Shell`,
//! only the surroundings differ).
//!
//! * the app shell (`ios/OpenOMSI/main.swift`) is the process's `main`; it computes the
//!   sandbox folders and calls [`openomsi_ios_start`] on the main thread;
//! * winit's event loop calls `UIApplicationMain` itself on iOS (its backend), so the
//!   Swift side never does;
//! * `$HOME` is the app's home (settings.cfg, launcher.json, the profiles); the content
//!   folder (`$OMSI_CONTENT`, where the OMSI 2 installation and the mods are imported to
//!   through the Files app - see docs/IOS.md) is `Documents/openOMSI`;
//! * the tilt sensor and the buzz are not wired up yet (CoreMotion/UIKit bindings would be
//!   needed); a Bluetooth game controller works as on a computer (gilrs), and so do the
//!   on-screen wheel and pedals of `touch.rs`.

use super::*;
use std::sync::atomic::{AtomicBool, Ordering};

/// A no-buffer, line-by-line log of the startup steps, written to the content folder
/// (`Documents/openOMSI/boot.log`) where the Files app can reach it even when nothing else
/// works: if the app shows only a black screen, this says exactly where it stopped.
/// `game.log` lives in the data folder; `boot.log` sits next to the crash log instead, so
/// a sandbox or path problem in the data folder does not hide it.
pub(crate) fn boot_log(text: &str) {
    let p = BOOT.get_or_init(|| {
        let content = PathBuf::from(std::env::var_os("OMSI_CONTENT").unwrap_or_else(|| std::ffi::OsString::from(".")));
        let _ = std::fs::create_dir_all(&content);
        content.join("boot.log")
    });
    use std::io::Write;
    let secs = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs_f64()).unwrap_or(0.0);
    if let Ok(mut f) = std::fs::OpenOptions::new().create(true).append(true).open(p) {
        let _ = writeln!(f, "[{secs:.3}] {text}");
        let _ = f.flush();
    }
}

static BOOT: std::sync::OnceLock<PathBuf> = std::sync::OnceLock::new();

/// The content folder, as the Swift shell hands it over (its default: the app's
/// `Documents/openOMSI`).
///
/// Called by `ios/OpenOMSI/AppMain.swift` before `UIApplicationMain`, on the main thread,
/// from the process's `main`. `home` and `content` are NUL-terminated UTF-8 paths (the
/// Swift side makes them with `withCString`). Nothing of this returns: the winit event
/// loop (which calls `UIApplicationMain` itself) runs until the process ends.
#[no_mangle]
pub extern "C" fn openomsi_ios_start(home_ptr: *const std::ffi::c_char, content_ptr: *const std::ffi::c_char) {
    let cstr = |p: *const std::ffi::c_char| -> String {
        if p.is_null() {
            return String::new();
        }
        // SAFETY: the Swift shell passes valid NUL-terminated strings
        unsafe { std::ffi::CStr::from_ptr(p) }.to_string_lossy().into_owned()
    };
    let home = cstr(home_ptr);
    let content = cstr(content_ptr);
    // (the boot log is written before HOME is set, so the path it falls back to is fine)
    if home.is_empty() {
        log::error!("openomsi_ios_start: the app shell gave no home folder");
        return;
    }
    std::env::set_var("HOME", &home);
    let content = if content.is_empty() {
        PathBuf::from(&home).join("Documents/openOMSI")
    } else {
        PathBuf::from(content)
    };
    let _ = std::fs::create_dir_all(&content);
    std::env::set_var("OMSI_CONTENT", &content);
    boot_log(&format!("openomsi_ios_start: home={home:?} content={content:?}"));
    // an error is also written where a person finds it without a computer:
    // openOMSI/crash.log in the content folder (or Documents when that is not writable)
    let crash = content.join("crash.log");
    std::panic::set_hook(Box::new(move |info| {
        let text = format!("the game stopped on an error (build {BUILD}): {info}\n{}", std::backtrace::Backtrace::force_capture());
        log::error!("{text}");
        let _ = std::fs::write(&crash, &text);
    }));
    init_log();
    boot_log("logger ready (game.log in the data folder)");
    log::info!("openOMSI {VERSION} for iOS, build {BUILD}");
    log::info!("home {:?}, content {}", std::env::var_os("HOME"), content.display());
    let _ = std::fs::write(content.join("README.txt"), README);
    // `openOMSI/env.txt`: the OMSI_* switches a computer takes from its environment, one
    // `NAME=value` a line (an iPhone has no environment to set; for looking into problems)
    if let Ok(t) = std::fs::read_to_string(content.join("env.txt")) {
        for line in t.lines() {
            if let Some((k, v)) = line.trim().split_once('=') {
                let k = k.trim();
                if k.starts_with("OMSI_") && !k.contains(char::is_whitespace) {
                    log::info!("env.txt: {k}");
                    std::env::set_var(k, v.trim());
                }
            }
        }
    }
    omsi_cfg::migrate_legacy_data_dir();
    boot_log("data migrated");
    let seed = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_nanos() as u64).unwrap_or(1) % 1_000_000_000;
    omsi_script::set_session_seed(seed);

    let event_loop = match EventLoop::new() {
        Ok(e) => e,
        Err(e) => {
            log::error!("no event loop: {e}");
            boot_log(&format!("no event loop: {e}"));
            return;
        }
    };
    boot_log("event loop created");
    let mut shell = Shell { launcher: None, game: None, instance: None };
    if let Err(e) = event_loop.run_app(&mut shell) {
        log::error!("{e}");
        boot_log(&format!("run_app returned an error: {e}"));
    }
    lan_mods::clean_up();
    // (the app ends with the process)
    std::process::exit(0);
}

/// The log to `game.log` in the app's data folder and to the device console (visible in
/// Xcode) - an iPhone has no terminal, and the launcher and the game share one process, so
/// without the file a crash left the launcher's report empty (see the Android build, #229).
/// The previous run's log is kept as `game-prev.log`: when that run closed in the middle of
/// a drive (the system killed the app, a graphics driver took it down), the launcher shows
/// its end ([`previous_run_crash`]).
fn init_log() {
    let dir = omsi_launcher_lib::data_dir();
    let (now, prev) = (dir.join("game.log"), dir.join("game-prev.log"));
    let _ = std::fs::rename(&now, &prev);
    let file = std::fs::File::create(&now).ok();
    let logger = FileLogger { file: std::sync::Mutex::new(file) };
    if log::set_boxed_logger(Box::new(logger)).is_ok() {
        log::set_max_level(log::LevelFilter::Info);
    }
}

/// A `log` logger that writes the desktop's `env_logger` layout (the launcher's `crash_of`
/// reads it) to the session's `game.log` and prints to the console.
struct FileLogger {
    file: std::sync::Mutex<Option<std::fs::File>>,
}

impl log::Log for FileLogger {
    fn enabled(&self, m: &log::Metadata) -> bool {
        m.level() <= log::Level::Info
    }

    fn log(&self, r: &log::Record) {
        if !self.enabled(r.metadata()) {
            return;
        }
        println!("{}", r.args());
        if r.level() <= log::Level::Warn {
            boot_log(&format!("[{} {}] {}", r.level(), r.target(), r.args()));
        }
        use std::io::Write;
        let secs = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs_f64()).unwrap_or(0.0);
        if let Some(f) = self.file.lock().unwrap_or_else(|e| e.into_inner()).as_mut() {
            let _ = writeln!(f, "[{secs:.3} {:<5} {}] {}", r.level(), r.target(), r.args());
        }
    }

    fn flush(&self) {
        if let Some(f) = self.file.lock().unwrap_or_else(|e| e.into_inner()).as_mut() {
            use std::io::Write;
            let _ = f.flush();
        }
    }
}

/// When the app's previous run closed while a drive was going (a game started from the
/// launcher, and neither "game ends" nor "session ended" after it): what it said last and
/// the end of its log, for the launcher's crash dialog. The same as the Android build's.
fn previous_run_crash() -> Option<(String, String)> {
    let p = omsi_launcher_lib::data_dir().join("game-prev.log");
    let text = std::fs::read_to_string(&p).ok()?;
    let lines: Vec<&str> = text.lines().collect();
    let start = lines.iter().rposition(|l| l.contains("starting the game:"))?;
    if lines[start..].iter().any(|l| l.contains("game ends") || l.contains("session ended")) {
        return None;
    }
    // sent to the background and not brought back: the system ended the app there (or the
    // player swiped it away) - no crash, whatever the game was doing
    let back = lines[start..].iter().rposition(|l| l.contains("app in the background"));
    let front = lines[start..].iter().rposition(|l| l.contains("app in front"));
    if back.is_some() && back > front {
        return None;
    }
    if let Some(c) = launcher::crash_of(&p) {
        return Some(c);
    }
    let last = lines.last().map(|l| l.split_once("] ").map(|x| x.1).unwrap_or(l)).unwrap_or("");
    let tail = lines[lines.len().saturating_sub(150)..].join("\n");
    Some((format!("the game closed without a word while it was running (the last it said: {last})"), tail))
}

const README: &str = "openOMSI\n\
\n\
Put a complete copy of OMSI 2 (the folder with Omsi.exe, maps and Vehicles in it) here as\n\
\"OMSI 2\", e.g. openOMSI/OMSI 2, and choose it in the launcher under Setup.\n\
How to get the folder onto the phone: in the Files app, put the OMSI 2 folder into\n\
\"On My iPhone/iPad → openOMSI\" (AirDrop, iCloud Drive, a USB cable with Finder, ...).\n\
Mods: copy them into openOMSI/Mods (they are installed when the launcher opens), or install\n\
a folder or a .zip, .7z or .rar from the launcher's Mods page. Screenshots are written to openOMSI/Screenshots.\n";

/// Whether the first drive of this run was already started (see `Shell::switch`).
static SAFER_TRIED: AtomicBool = AtomicBool::new(false);

/// The launcher, or the game in the launcher's window (the same shell as the Android
/// build; kept here so the two phones do not have to agree on one shared module yet).
struct Shell {
    launcher: Option<Box<launcher::Launcher>>,
    game: Option<Box<App>>,
    instance: Option<()>,
}

impl Shell {
    fn launcher(&mut self) -> &mut launcher::Launcher {
        if self.launcher.is_none() {
            // the original installation and the content roots, as a bare start finds them
            let args = Args::parse_from(["openomsi"]);
            if let Err(e) = prepare(args, true) {
                log::error!("{e:#}");
            }
            // (an installation chosen under Setup is known from here on)
            // (iOS has no gallery scanning to hide the textures from, as Android's .nomedia
            // does - the content is in the app's own Documents, where Photos never looks)
            launcher_statics();
            self.instance = Some(());
            self.launcher = Some(Box::new(launcher::Launcher::new(graphics_instance())));
        }
        self.launcher.as_mut().unwrap()
    }

    /// After every event: a game the launcher asked for starts, a game that ended gives
    /// the window back.
    fn switch(&mut self, event_loop: &ActiveEventLoop) {
        if self.game.is_some() {
            if !crate::platform::take_leave() {
                return;
            }
            let mut game = self.game.take().unwrap();
            game.exiting(event_loop);
            let window = game.window.take();
            drop(game);
            lan_mods::clean_up();
            log::info!("session ended: back to the launcher");
            let l = self.launcher();
            if let Some(w) = window {
                l.adopt_window(w);
            }
            l.resumed(event_loop);
            return;
        }
        let Some(line) = omsi_launcher_lib::take_in_process_launch() else { return };
        // the first drive after a run that closed in the middle of one starts with safer
        // graphics (on Android also on OpenGL after a Vulkan failure: an iPhone has no
        // other backend than Metal, so the Vulkan branch below never fires there)
        if !SAFER_TRIED.swap(true, Ordering::Relaxed) && previous_run_crash().is_some() {
            let prev = std::fs::read_to_string(omsi_launcher_lib::data_dir().join("game-prev.log")).unwrap_or_default();
            let vulkan = prev.lines().any(|l| l.contains("renderer: ") && l.contains("(Vulkan)")) || prev.lines().any(|l| l.contains("graphics: ") && l.to_ascii_uppercase().contains("VULKAN"));
            std::env::set_var("OMSI_SAFE_GPU", "1");
            let last = prev.lines().rev().find(|l| !l.trim().is_empty()).unwrap_or("");
            let compiling = last.contains("renderer: compiling") || last.contains("cloud noise made") || last.contains("opening graphics device") || last.contains("compiling renderer pipelines");
            if vulkan && compiling {
                if let Ok(mut v) = omsi_launcher_lib::get_settings() {
                    v["graphics_api"] = serde_json::json!("gl");
                    match omsi_launcher_lib::save_settings(&v) {
                        Ok(()) => log::warn!("the graphics driver went down compiling the shaders on Vulkan: OpenGL from now on (Settings → Graphics API)"),
                        Err(e) => log::warn!("settings not saved: {e:#}"),
                    }
                }
            }
            if vulkan && std::env::var_os("OMSI_BACKEND").is_none() {
                std::env::set_var("OMSI_BACKEND", "gl");
            }
            log::warn!("the last run closed in the middle of a drive: this one starts with safer graphics{}", if vulkan { " on OpenGL" } else { "" });
        }
        log::info!("starting the game: {}", line.join(" "));
        let argv: Vec<String> = std::iter::once("openomsi".to_string()).chain(line).collect();
        let args = match Args::try_parse_from(&argv) {
            Ok(a) => a,
            Err(e) => {
                log::error!("the launcher's command line: {e}");
                return;
            }
        };
        let game = prepare(args, false).and_then(|p| match p {
            Some((args, server)) => make_app(args, server),
            None => Ok(None),
        });
        let mut app = match game {
            Ok(Some(app)) => app,
            Ok(None) => return,
            Err(e) => {
                log::error!("the game could not start: {e:#}");
                return;
            }
        };
        let window = self.launcher().release_window();
        app.create_window(event_loop, window);
        self.game = Some(Box::new(app));
    }
}

impl ApplicationHandler for Shell {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        log::info!("app in front");
        boot_log("app in front (resumed): creating the launcher");
        match self.game.as_mut() {
            Some(g) => g.resumed(event_loop),
            None => self.launcher().resumed(event_loop),
        }
        self.switch(event_loop);
        boot_log("launcher resumed, window and surface made");
    }

    fn suspended(&mut self, event_loop: &ActiveEventLoop) {
        // (the system may end an app in the background without a word: see
        // `previous_run_crash`)
        log::info!("app in the background");
        match self.game.as_mut() {
            Some(g) => g.suspended(event_loop),
            None => self.launcher().suspended(event_loop),
        }
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, id: WindowId, event: WindowEvent) {
        match self.game.as_mut() {
            Some(g) => g.window_event(event_loop, id, event),
            None => self.launcher().window_event(event_loop, id, event),
        }
        self.switch(event_loop);
    }

    fn device_event(&mut self, event_loop: &ActiveEventLoop, id: winit::event::DeviceId, event: DeviceEvent) {
        if let Some(g) = self.game.as_mut() {
            g.device_event(event_loop, id, event);
        }
    }

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        match self.game.as_mut() {
            Some(g) => {
                event_loop.set_control_flow(winit::event_loop::ControlFlow::Poll);
                g.about_to_wait(event_loop)
            }
            None => self.launcher().about_to_wait(event_loop),
        }
        self.switch(event_loop);
    }

    fn memory_warning(&mut self, _event_loop: &ActiveEventLoop) {
        log::warn!("the system is short of memory");
        crate::memory::release_free_memory();
    }

    fn exiting(&mut self, event_loop: &ActiveEventLoop) {
        if let Some(g) = self.game.as_mut() {
            g.exiting(event_loop);
        }
    }
}
