use std::{path::PathBuf, sync::Mutex};
use tauri::{Emitter, Manager};

/// Script files waiting to be opened by the page (double-click, Open With, or a second launch).
#[derive(Default)]
struct Pending(Mutex<Vec<PathBuf>>);

fn is_script(p: &PathBuf) -> bool {
    matches!(
        p.extension().and_then(|e| e.to_str()).map(|e| e.to_ascii_lowercase()).as_deref(),
        Some("midad") | Some("fountain") | Some("txt") | Some("json")
    ) && p.is_file()
}

fn queue(app: &tauri::AppHandle, paths: Vec<PathBuf>) {
    let paths: Vec<PathBuf> = paths.into_iter().filter(is_script).collect();
    if !paths.is_empty() {
        app.state::<Pending>().0.lock().unwrap().extend(paths);
        let _ = app.emit("midad-open", ());
    }
    if let Some(w) = app.get_webview_window("main") {
        let _ = w.unminimize();
        let _ = w.set_focus();
    }
}

#[derive(serde::Serialize)]
struct Opened {
    name: String,
    content: String,
}

/// Hand the waiting files to the page, read as text.
#[tauri::command]
fn opened_files(state: tauri::State<Pending>) -> Vec<Opened> {
    let list: Vec<PathBuf> = state.0.lock().unwrap().drain(..).collect();
    list.into_iter()
        .filter_map(|p| {
            let content = std::fs::read_to_string(&p).ok()?;
            let name = p.file_name()?.to_string_lossy().into_owned();
            Some(Opened { name, content })
        })
        .collect()
}

/// Save an exported file (print-ready HTML, Fountain text, Midad file, CSV, backup) to the
/// user's Downloads folder without overwriting an existing file.
#[tauri::command]
fn save_file(app: tauri::AppHandle, name: String, data: String) -> Result<String, String> {
    let dir = app
        .path()
        .download_dir()
        .or_else(|_| app.path().document_dir())
        .or_else(|_| app.path().home_dir())
        .map_err(|e| e.to_string())?;
    let clean: String = name
        .chars()
        .map(|c| if "\\/:*?\"<>|".contains(c) || c.is_control() { ' ' } else { c })
        .collect();
    let clean = clean.trim();
    let clean = if clean.is_empty() { "midad.txt" } else { clean };

    let (stem, ext) = match clean.rfind('.') {
        Some(i) if i > 0 => (&clean[..i], &clean[i..]),
        _ => (clean, ""),
    };
    let mut path: PathBuf = dir.join(clean);
    let mut n = 2;
    while path.exists() {
        path = dir.join(format!("{stem} ({n}){ext}"));
        n += 1;
    }
    std::fs::write(&path, data.as_bytes()).map_err(|e| e.to_string())?;
    Ok(path.to_string_lossy().into_owned())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    #[allow(unused_mut)]
    let mut builder = tauri::Builder::default();

    // A second launch (e.g. double-clicking a script while Midad is open) hands its files to the open window.
    #[cfg(desktop)]
    {
        builder = builder.plugin(tauri_plugin_single_instance::init(|app, args, _cwd| {
            queue(app, args.into_iter().skip(1).map(PathBuf::from).collect());
        }));
    }

    builder
        .manage(Pending::default())
        .setup(|app| {
            // Windows and Linux pass the opened file as a launch argument.
            let files: Vec<PathBuf> = std::env::args().skip(1).map(PathBuf::from).filter(is_script).collect();
            app.state::<Pending>().0.lock().unwrap().extend(files);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![save_file, opened_files])
        .build(tauri::generate_context!())
        .expect("error while running Midad")
        .run(|_app, _event| {
            // macOS delivers opened files as an event.
            #[cfg(any(target_os = "macos", target_os = "ios"))]
            if let tauri::RunEvent::Opened { urls } = _event {
                let paths = urls.into_iter().filter_map(|u| u.to_file_path().ok()).collect();
                queue(_app, paths);
            }
        });
}
