//! E-OS Notes — Crimson-themed notes app.
//!
//! GUI: Slint (software renderer over the winit orbital backend), behind the
//! default `gui` cargo feature — a Redox-target concern.
//! Storage: SQLite in WAL mode (rusqlite, bundled).
//! `eos-notes --selftest` runs the headless storage proof (used by CI/boot
//! probes) and prints EOS-NOTES-SELFTEST-OK.

mod db;
#[cfg(feature = "gui")]
mod gui;
#[cfg(all(feature = "gui", target_os = "redox"))]
mod orbital_platform;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.iter().any(|a| a == "--selftest") {
        let path = std::path::PathBuf::from("/tmp/eos-notes-selftest.db");
        match db::selftest(&path) {
            Ok(()) => {
                // Printed on both streams so it lands in the boot serial no
                // matter how the probe is wired.
                println!("EOS-NOTES-SELFTEST-OK");
                eprintln!("EOS-NOTES-SELFTEST-OK");
            }
            Err(err) => {
                println!("EOS-NOTES-SELFTEST-FAIL: {err}");
                eprintln!("EOS-NOTES-SELFTEST-FAIL: {err}");
                std::process::exit(1);
            }
        }
        return;
    }

    #[cfg(feature = "gui")]
    gui::run();

    #[cfg(not(feature = "gui"))]
    {
        eprintln!("eos-notes: built without the `gui` feature (selftest-only binary)");
        std::process::exit(2);
    }
}
