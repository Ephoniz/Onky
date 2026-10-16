//! Audio Program mixer.

#![cfg_attr(windows, windows_subsystem = "windows")]

use audio_program::{
    attach_parent_console, draw, install_panic_hook, install_style, report_startup_failure, Session,
};
#[cfg(windows)]
use audio_program::{set_updates_enabled, updates_enabled_at, updates_setting_path};

fn main() {
    let headless = std::env::args().any(|arg| arg == "--headless");
    if headless {
        attach_parent_console();
        run_headless(Session::start());
        return;
    }
    install_panic_hook();
    #[cfg(windows)]
    audio_program::apply_pending_update();
    launch_window(Session::start());
}

fn run_headless(session: Session) {
    if let Some(error) = &session.error {
        eprintln!("audio-program: {error}");
        std::process::exit(1);
    }
    println!("Audio Program virtual cables:");
    for cable in &session.cables {
        let kind = if cable.output { "output" } else { "input" };
        println!("  {kind} {} ({})", cable.node_name, cable.description);
    }
    println!("routing virtual input N to virtual output N");
    println!("leave this process running; cables are removed when it exits");
    wait_until_stopped();
}

fn launch_window(session: Session) {
    // The Windows software OpenGL driver is 1.1, so a glow window exits at once.
    // wgpu paints with Direct3D instead.
    let options = eframe::NativeOptions {
        viewport: eframe::egui::ViewportBuilder::default()
            .with_inner_size([980.0, 720.0])
            .with_min_inner_size([640.0, 480.0])
            .with_title("Audio Program"),
        #[cfg(windows)]
        renderer: eframe::Renderer::Wgpu,
        ..Default::default()
    };
    let result = eframe::run_native(
        "Audio Program",
        options,
        Box::new(|cc| {
            install_style(&cc.egui_ctx);
            Ok(Box::new(Mixer {
                session,
                #[cfg(windows)]
                check_updates: updates_setting_path()
                    .as_deref()
                    .is_none_or(updates_enabled_at),
            }))
        }),
    );
    if let Err(err) = result {
        report_startup_failure(&err.to_string());
    }
}

struct Mixer {
    session: Session,
    #[cfg(windows)]
    check_updates: bool,
}

impl eframe::App for Mixer {
    fn ui(&mut self, ui: &mut eframe::egui::Ui, _frame: &mut eframe::Frame) {
        ui.ctx()
            .request_repaint_after(std::time::Duration::from_millis(33));
        #[cfg(windows)]
        let updates_before = self.check_updates;
        draw(
            ui,
            &mut self.session,
            #[cfg(windows)]
            Some(&mut self.check_updates),
            #[cfg(not(windows))]
            None,
        );
        #[cfg(windows)]
        if self.check_updates != updates_before {
            let _ = set_updates_enabled(self.check_updates);
        }
    }
}

fn wait_until_stopped() {
    // SIGTERM closes the PipeWire socket. Nodes were created with linger off,
    // so the daemon removes them even when this sleep is interrupted.
    loop {
        std::thread::sleep(std::time::Duration::from_secs(3600));
    }
}
