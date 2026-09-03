//! The Slint GUI half of E-OS Notes.
//!
//! Two windowing platforms, picked at compile time and never both:
//!   * Redox — the shared E-OS Slint-on-Orbital backend from `eos-ui`;
//!   * host (Linux/macOS/Windows) — winit, behind the `host-backend` feature.
//!
//! `--no-default-features` still builds the CLI/selftest half alone.

use crate::db;
use slint::{ModelRc, SharedString, VecModel};
use std::cell::RefCell;
use std::rc::Rc;

slint::include_modules!();

/// Format a unix timestamp as "YYYY-MM-DD HH:MM" (UTC; the point is a stable,
/// sortable label — not local-time correctness).
fn format_ts(ts: i64) -> String {
    if ts <= 0 {
        return String::new();
    }
    let days = ts.div_euclid(86_400);
    let secs = ts.rem_euclid(86_400);
    // Howard Hinnant's civil_from_days.
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };
    format!(
        "{:04}-{:02}-{:02} {:02}:{:02}",
        y,
        m,
        d,
        secs / 3600,
        (secs % 3600) / 60
    )
}

struct App {
    db: db::Db,
    filter: String,
}

fn refresh_list(app: &App, win: &MainWindow) {
    let items: Vec<NoteItem> = match app.db.list(&app.filter) {
        Ok(notes) => notes
            .iter()
            .map(|n| NoteItem {
                id: n.id as i32,
                title: SharedString::from(n.title.as_str()),
                subtitle: SharedString::from(format_ts(n.updated_at)),
            })
            .collect(),
        Err(err) => {
            win.set_status(SharedString::from(format!("Błąd bazy: {err}")));
            Vec::new()
        }
    };
    let count = items.len();
    win.set_notes(ModelRc::new(VecModel::from(items)));
    win.set_status(SharedString::from(format!("{count} notatek")));
}

fn open_note(app: &App, win: &MainWindow, id: i64) {
    match app.db.get(id) {
        Ok(note) => {
            win.set_current_id(note.id as i32);
            win.set_current_title(SharedString::from(note.title.as_str()));
            win.set_current_body(SharedString::from(note.body.as_str()));
            win.set_editor_enabled(true);
        }
        Err(err) => win.set_status(SharedString::from(format!("Błąd odczytu: {err}"))),
    }
}

fn clear_editor(win: &MainWindow) {
    win.set_current_id(-1);
    win.set_current_title(SharedString::default());
    win.set_current_body(SharedString::default());
    win.set_editor_enabled(false);
}

/// Install the host window backend (winit), when this build has one.
///
/// Three mutually exclusive definitions rather than `cfg!` branches inside one
/// body: the "no backend" arm has to be a plain `Err`, not a diverging block,
/// or `clippy -D warnings` trips over the `return` in tail position.
#[cfg(all(feature = "host-backend", not(target_os = "redox")))]
fn install_host_backend() -> Result<(), String> {
    let backend = i_slint_backend_winit::Backend::new()
        .map_err(|e| format!("winit backend unavailable: {e}"))?;
    slint::platform::set_platform(Box::new(backend))
        .map_err(|_| "a Slint platform was already installed".to_string())
}

/// Fail-closed default on a host: nothing was linked in to draw on, so say so.
///
/// The alternative is what the pre-`host-backend` binary actually did, measured
/// 2026-09-03 on rustc 1.98.0: it compiled, packaged, ran `--selftest` fine and
/// then panicked at the window constructor with "No default Slint platform was
/// selected, and no Slint platform was initialized". A binary that only fails
/// once a user double-clicks it is the worst shape this can take.
#[cfg(all(not(feature = "host-backend"), not(target_os = "redox")))]
fn install_host_backend() -> Result<(), String> {
    Err(
        "built without the `host-backend` feature: no windowing backend is linked in \
         (rebuild with `--features host-backend` for a host window)"
            .to_string(),
    )
}

/// On Redox the platform is Orbital, installed by `eos_ui::init` below.
#[cfg(target_os = "redox")]
fn install_host_backend() -> Result<(), String> {
    Ok(())
}

pub fn run() {
    // Order matters: winit first on a host, then the shared E-OS
    // Slint-on-Orbital backend + font bootstrap, which is a no-op off Redox.
    if let Err(err) = install_host_backend() {
        eprintln!("eos-notes: {err}");
        // 2, not 1: no defect was found, the check could not run at all —
        // this binary has no backend to draw on (CLAUDE.md §13.1).
        std::process::exit(2);
    }
    eos_ui::init("E-OS Notes");

    let database =
        db::Db::open(&db::default_path()).expect("eos-notes: cannot open the notes database");
    let app = Rc::new(RefCell::new(App {
        db: database,
        filter: String::new(),
    }));

    let win = MainWindow::new().expect("eos-notes: cannot create the window");
    refresh_list(&app.borrow(), &win);

    {
        let app = app.clone();
        let weak = win.as_weak();
        win.on_new_note(move || {
            let win = weak.unwrap();
            let app = app.borrow();
            match app.db.create() {
                Ok(id) => {
                    refresh_list(&app, &win);
                    open_note(&app, &win, id);
                }
                Err(err) => win.set_status(SharedString::from(format!("Błąd tworzenia: {err}"))),
            }
        });
    }

    {
        let app = app.clone();
        let weak = win.as_weak();
        win.on_open_note(move |id| {
            let win = weak.unwrap();
            open_note(&app.borrow(), &win, id as i64);
        });
    }

    {
        let app = app.clone();
        let weak = win.as_weak();
        win.on_delete_note(move || {
            let win = weak.unwrap();
            let app = app.borrow();
            let id = win.get_current_id();
            if id < 0 {
                return;
            }
            match app.db.delete(id as i64) {
                Ok(()) => {
                    clear_editor(&win);
                    refresh_list(&app, &win);
                }
                Err(err) => win.set_status(SharedString::from(format!("Błąd usuwania: {err}"))),
            }
        });
    }

    {
        let app = app.clone();
        let weak = win.as_weak();
        win.on_content_edited(move || {
            let win = weak.unwrap();
            let app = app.borrow();
            let id = win.get_current_id();
            if id < 0 {
                return;
            }
            let title = win.get_current_title();
            let body = win.get_current_body();
            match app.db.save(id as i64, title.as_str(), body.as_str()) {
                // The sidebar is refreshed on open/new/delete/search, not on
                // every keystroke — saving stays silent to keep the list from
                // re-sorting under the cursor.
                Ok(()) => win.set_status(SharedString::from("Zapisano")),
                Err(err) => win.set_status(SharedString::from(format!("Błąd zapisu: {err}"))),
            }
        });
    }

    {
        let app = app.clone();
        let weak = win.as_weak();
        win.on_search_changed(move |text| {
            let win = weak.unwrap();
            app.borrow_mut().filter = text.to_string();
            refresh_list(&app.borrow(), &win);
        });
    }

    win.run().expect("eos-notes: event loop failed");
}
