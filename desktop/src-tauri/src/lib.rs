use std::path::PathBuf;
use tauri::Manager;

/// Save an exported file (print-ready HTML, Fountain text, CSV, backup) to the
/// user's Downloads folder without overwriting an existing file.
#[tauri::command]
fn save_file(app: tauri::AppHandle, name: String, data: String) -> Result<String, String> {
    let dir = app
        .path()
        .download_dir()
        .or_else(|_| app.path().document_dir())
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
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![save_file])
        .run(tauri::generate_context!())
        .expect("error while running Midad");
}
