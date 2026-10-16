//! Audio Program mixer.

#![cfg_attr(windows, windows_subsystem = "windows")]

use audio_program::{
    attach_parent_console, draw, install_panic_hook, install_style, report_startup_failure, Session,
};

fn main() {
    let headless = std::env::args().any(|arg| arg == "--headless");
    if headless {
        attach_parent_console();
        run_headless(Session::start());
        return;
    }
    install_panic_hook();
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
    let options = eframe::NativeOptions {
        viewport: eframe::egui::ViewportBuilder::default()
            .with_inner_size([980.0, 720.0])
            .with_min_inner_size([640.0, 480.0])
            .with_title("Audio Program"),
        ..Default::default()
    };
    let result = eframe::run_native(
        "Audio Program",
        options,
        Box::new(|cc| {
            install_style(&cc.egui_ctx);
            Ok(Box::new(Mixer { session }))
        }),
    );
    if let Err(err) = result {
        report_startup_failure(&err.to_string());
    }
}

struct Mixer {
    session: Session,
}

impl eframe::App for Mixer {
    fn ui(&mut self, ui: &mut eframe::egui::Ui, _frame: &mut eframe::Frame) {
        ui.ctx()
            .request_repaint_after(std::time::Duration::from_millis(33));
        draw(ui, &mut self.session);
    }
}

fn wait_until_stopped() {
    // SIGTERM closes the PipeWire socket. Nodes were created with linger off,
    // so the daemon removes them even when this sleep is interrupted.
    loop {
        std::thread::sleep(std::time::Duration::from_secs(3600));
    }
}
