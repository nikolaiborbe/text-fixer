#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
// Hides the console window on Windows release builds.

use tauri::{
  generate_context,
  menu::MenuBuilder,
  tray::{TrayIconBuilder, TrayIconEvent},
  webview::{WebviewWindowBuilder},
  Manager, WindowEvent, WebviewUrl
};

fn main() {
  tauri::Builder::default()
    // ---------- 1. Create the tray (icon + menu) ----------
    .setup(|app| {
      // Build a small context‑menu for the tray icon
      let tray_menu = MenuBuilder::new(app)
        .text("show", "Show")
        .separator()
        .text("quit", "Quit")
        .build()?;

      // Load an .ico / .png file at compile‑time
      // (path is relative to the Cargo.toml directory)
      let icon = tauri::include_image!("icons/tray/icon.png");

      // Register the tray icon
      TrayIconBuilder::with_id("main-tray")
        .icon(icon)
        .menu(&tray_menu)
        // show menu only on right‑click
        .show_menu_on_left_click(false)
        .build(app)?;

      Ok(())
    })
    // ---------- 2. Handle tray icon clicks ----------
    .on_tray_icon_event(|app, event| {
      if let TrayIconEvent::Click { .. } = event {
        show_or_create_main_window(app);
      }
    })
    // ---------- 3. Handle tray‑menu actions ----------
    .on_menu_event(|app, event| match event.id().as_ref() {
      "show" => show_or_create_main_window(app),
      "quit" => std::process::exit(0),
      _ => {}
    })
    // ---------- 4. Keep the app alive (hide instead of close) ----------
    .on_window_event(|window, event| {
      if let WindowEvent::CloseRequested { api, .. } = event {
        window.hide().ok();
        api.prevent_close();
      }
    })
    .run(generate_context!())
    .expect("error while running tauri app");
}

fn show_or_create_main_window<R: tauri::Runtime>(app: &tauri::AppHandle<R>) {
  if let Some(w) = app.get_webview_window("main") {
    let _ = w.show();
    let _ = w.set_focus();
  } else {
    let _ = WebviewWindowBuilder::new(
      app,
      "main",
      WebviewUrl::App("index.html".into()), // Svelte entry‑point
    )
    .title("LiveRewrite")
    .min_inner_size(400.0, 300.0)
    .build();
  }
}
