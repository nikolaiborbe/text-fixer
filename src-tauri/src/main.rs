#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
// Hides the console window on Windows release builds.

use tauri::{
  generate_context,
  menu::MenuBuilder,
  tray::{MouseButton, TrayIconBuilder, TrayIconEvent},
  webview::{Url, WebviewWindowBuilder},
  AppHandle, Manager, WebviewUrl, WebviewWindow, WindowEvent,
};
use std::sync::{Arc, Mutex};
use tauri_plugin_clipboard_manager::ClipboardExt;
use tauri_plugin_global_shortcut::{Code, GlobalShortcutExt, Modifiers, Shortcut, ShortcutState};

fn main() {
  tauri::Builder::default()
    .plugin(tauri_plugin_clipboard_manager::init())
    .manage(PastePlugin::default())
    .invoke_handler(tauri::generate_handler![
      submit_input,
      hide_window,
      mark_previous_window,
      paste_into_previous_app,
    ])
    // ── 1. One‑time setup phase ───────────────────────────────────────────────
    .setup(|app| {
      // Global shortcut we want to register (⌘‑Space)
      let shortcut = Shortcut::new(Some(Modifiers::ALT), Code::Space);

      // Register the global‑shortcut plugin **once** with its handler
      app.handle().plugin(
        tauri_plugin_global_shortcut::Builder::new()
          .with_handler(move |app_handle, s, event| {
            if s == &shortcut && matches!(event.state(), ShortcutState::Pressed) {
              if let Some(window) = app_handle.get_webview_window("main") {
                // Toggle visibility: hide if already visible, else show & focus
                match window.is_visible() {
                  Ok(true) => {
                    let _ = window.hide();
                  }
                  _ => {
                    let _ = window.show();
                    let _ = window.set_focus();
                  }
                }
              } else {
                // Otherwise, create a new main window
                show_or_create_main_window(app_handle);
              }
            }
          })
          .build(),
      )?;

      // Tell the OS we actually want that shortcut
      app.global_shortcut().register(shortcut)?;

      // ── Tray icon & menu ─────────────────────────────────────────────────────
      let tray_menu = MenuBuilder::new(app)
        .text("show", "Show")
        .separator()
        .text("quit", "Quit")
        .build()?;

      let icon = tauri::include_image!("icons/tray/icon.png");

      TrayIconBuilder::with_id("main-tray")
        .icon(icon)
        .menu(&tray_menu)
        .show_menu_on_left_click(false)
        .build(app)?;

      Ok(())
    })
    // ── 2. React to tray clicks, menu items, and window close events ──────────
    .on_tray_icon_event(|app, event| {
      if matches!(
        event,
        TrayIconEvent::Click {
          button: MouseButton::Left,
          ..
        }
      ) {
        show_or_create_main_window(app);
      }
    })
    .on_menu_event(|app, event| match event.id().as_ref() {
      "show" => show_or_create_main_window(app),
      "quit" => std::process::exit(0),
      _ => {}
    })
    .on_window_event(|window, event| {
      if let WindowEvent::CloseRequested { api, .. } = event {
        let _ = window.hide();
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

#[tauri::command]
fn submit_input(text: String) {
  println!("{text}");
}

#[tauri::command]
fn hide_window(app: AppHandle) {
  if let Some(window) = app.get_webview_window("main") {
    let _ = window.hide();
  }
}

#[derive(Default)]
struct PastePlugin {
  last_window: Arc<Mutex<Option<PlatformHandle>>>,
}

#[tauri::command]
fn mark_previous_window(state: tauri::State<PastePlugin>) {
  *state.last_window.lock().unwrap() = Some(platform_get_foreground_handle());
}

#[tauri::command]
fn paste_into_previous_app(
  text: String,
  app: tauri::AppHandle,
  state: tauri::State<PastePlugin>,
) -> tauri::Result<()> {
  // 1. to clipboard
  println!("{text}");
  let _ = app.clipboard().write_text(&text);

  // 2. reactivate old window
  if let Some(h) = *state.last_window.lock().unwrap() {
    platform_activate_window(h)?;
    platform_send_paste_shortcut(h)?;
  }
  Ok(())
}

// ── Platform stubs (replace with real implementations) ────────────────
#[allow(dead_code)]
type PlatformHandle = ();

#[allow(dead_code)]
fn platform_get_foreground_handle() -> PlatformHandle {
  ()
}

#[allow(dead_code)]
fn platform_activate_window(_: PlatformHandle) -> tauri::Result<()> {
  Ok(())
}

#[allow(dead_code)]
fn platform_send_paste_shortcut(_: PlatformHandle) -> tauri::Result<()> {
  Ok(())
}
