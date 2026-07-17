//! The Slint GUI half of E-OS Notes (Redox-target concern; hosts may build
//! with `--no-default-features` for the CLI/selftest half only).

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

/// Register the image's TTF fonts with Slint's fontique collection. Fontique
/// has no system-font discovery on Redox, so an empty collection would panic
/// the renderer on the first text draw; the E-OS image ships DejaVu under
/// /usr/share/fonts (the `dejavu` package).
#[cfg(target_os = "redox")]
fn register_system_fonts() {
    use slint::fontique_010::fontique;

    fn collect_ttfs(dir: &std::path::Path, out: &mut Vec<std::path::PathBuf>) {
        if let Ok(entries) = std::fs::read_dir(dir) {
            let mut entries: Vec<_> = entries.flatten().map(|e| e.path()).collect();
            entries.sort();
            for path in entries {
                if path.is_dir() {
                    collect_ttfs(&path, out);
                } else if path
                    .extension()
                    .is_some_and(|e| e.eq_ignore_ascii_case("ttf"))
                {
                    out.push(path);
                }
            }
        }
    }

    let mut paths = Vec::new();
    collect_ttfs(std::path::Path::new("/usr/share/fonts"), &mut paths);

    let mut collection = slint::fontique_010::shared_collection();
    let mut sans = Vec::new();
    let mut mono = Vec::new();
    for path in &paths {
        let Ok(bytes) = std::fs::read(path) else {
            continue;
        };
        let blob = fontique::Blob::new(std::sync::Arc::new(bytes));
        let fonts = collection.register_fonts(blob, None);
        let is_mono = path
            .file_name()
            .and_then(|n| n.to_str())
            .is_some_and(|n| n.contains("Mono"));
        for (family, _) in &fonts {
            if is_mono {
                mono.push(*family);
            } else {
                sans.push(*family);
            }
        }
    }
    if sans.is_empty() {
        sans = mono.clone();
    }
    if mono.is_empty() {
        mono = sans.clone();
    }
    collection.set_generic_families(fontique::GenericFamily::SansSerif, sans.iter().copied());
    collection.set_generic_families(fontique::GenericFamily::SystemUi, sans.iter().copied());
    collection.set_generic_families(fontique::GenericFamily::Monospace, mono.iter().copied());
    collection.append_fallbacks(
        fontique::FallbackKey::new(fontique::Script::from_str_unchecked("Latn"), None),
        sans.iter().copied(),
    );
    if sans.is_empty() {
        eprintln!("eos-notes: no TTF fonts found under /usr/share/fonts — text will not render");
    }
}

pub fn run() {
    #[cfg(target_os = "redox")]
    slint::platform::set_platform(Box::new(crate::orbital_platform::OrbitalPlatform::new()))
        .expect("eos-notes: cannot set the orbital platform");
    #[cfg(target_os = "redox")]
    register_system_fonts();

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
