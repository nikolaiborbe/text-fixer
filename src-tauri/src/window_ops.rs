use anyhow::{bail, Result};
use std::ffi::c_void;
use x_win::WindowInfo;
use image::{ImageBuffer, Rgba, RgbaImage};
use std::io::Cursor;

use windows::Win32::{
  Foundation::{HWND, LPARAM, WPARAM, LRESULT},
  System::Threading::{AttachThreadInput, GetCurrentThreadId},
  UI::WindowsAndMessaging::{
    GetForegroundWindow, GetWindowThreadProcessId, SendMessageW, SetForegroundWindow, ShowWindow,
    ICON_SMALL, SW_RESTORE, WM_GETICON, HICON, ICON_BIG
  },
  Graphics::Gdi::{
    GetDC, GetDIBits, CreateCompatibleDC, SelectObject, DeleteObject, DeleteDC,
    BITMAPINFO, BITMAPINFOHEADER, DIB_RGB_COLORS, BI_RGB, HGDIOBJ, HBITMAP, HDC
  },
};

#[cfg(target_os = "windows")]
pub fn get_window_icon(window: &WindowInfo) -> Option<HICON> {
  // Validate that window.id is not null/zero
  if window.id == 0 {
    return None;
  }
  
  let hwnd = HWND(window.id as usize as *mut c_void);
  let icon: LRESULT = unsafe {
    SendMessageW(
      hwnd,
      WM_GETICON,
      Some(WPARAM(ICON_SMALL as usize)),
      Some(LPARAM(0)),
    )
  };
  
  // Check if we got a valid icon handle
  if icon.0 != 0 {
    return Some(HICON(icon.0 as *mut c_void));
  }
  
  // Fallback to big icon if small icon failed
  let icon: LRESULT = unsafe {
    SendMessageW(
      hwnd,
      WM_GETICON,
      Some(WPARAM(ICON_BIG as usize)),
      Some(LPARAM(0)),
    )
  };
  
  if icon.0 != 0 {
    return Some(HICON(icon.0 as *mut c_void));
  } else {
    None
  }
}

#[cfg(not(target_os = "windows"))]
pub fn get_window_icon(_window: &WindowInfo) -> Option<HICON> {
  // Not implemented for other platforms yet
  None
}

#[cfg(target_os = "windows")]
fn hicon_to_png(hicon: HICON) -> Result<Vec<u8>> {
  unsafe {
    // Get icon info to extract the bitmap
    let mut icon_info = std::mem::zeroed();
    if !windows::Win32::UI::WindowsAndMessaging::GetIconInfo(hicon, &mut icon_info).is_ok() {
      bail!("Failed to get icon info");
    }
    
    let hbmp = icon_info.hbmColor;
    if hbmp.is_invalid() {
      bail!("Invalid bitmap handle");
    }
    
    // Use fixed size for icon (32x32 is standard)
    let width = 32;
    let height = 32;
    
    // Create compatible DC and select bitmap
    let hdc = GetDC(None);
    let mem_dc = CreateCompatibleDC(Some(hdc));
    let old_obj = SelectObject(mem_dc, HGDIOBJ(hbmp.0));
    
    // Prepare bitmap info for GetDIBits
    let mut bmi = BITMAPINFO {
      bmiHeader: BITMAPINFOHEADER {
        biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
        biWidth: width as i32,
        biHeight: -(height as i32), // Negative for top-down
        biPlanes: 1,
        biBitCount: 32,
        biCompression: BI_RGB.0,
        ..Default::default()
      },
      ..Default::default()
    };
    
    // Allocate buffer for pixel data
    let buffer_size = (width * height * 4) as usize;
    let mut buffer = vec![0u8; buffer_size];
    
    // Get pixel data
    if GetDIBits(
      mem_dc,
      hbmp,
      0,
      height,
      Some(buffer.as_mut_ptr() as *mut c_void),
      &mut bmi,
      DIB_RGB_COLORS,
    ) == 0 {
      bail!("Failed to get bitmap bits");
    }
    
    // Clean up GDI objects
    SelectObject(mem_dc, old_obj);
    DeleteDC(mem_dc);
    DeleteObject(HGDIOBJ(hbmp.0));
    
    // Convert BGRA to RGBA and create image
    let mut image = RgbaImage::new(width, height);
    for y in 0..height {
      for x in 0..width {
        let idx = ((y * width + x) * 4) as usize;
        let b = buffer[idx];
        let g = buffer[idx + 1];
        let r = buffer[idx + 2];
        let a = buffer[idx + 3];
        image.put_pixel(x, y, Rgba([r, g, b, a]));
      }
    }
    
    // Convert to PNG
    let mut png_data = Vec::new();
    image.write_to(&mut Cursor::new(&mut png_data), image::ImageFormat::Png)?;
    
    Ok(png_data)
  }
}

#[cfg(not(target_os = "windows"))]
fn hicon_to_png(_hicon: HICON) -> Result<Vec<u8>> {
  bail!("Icon conversion not supported on this platform");
}

#[tauri::command]
pub fn get_prev_window_icon_png(state: tauri::State<'_, crate::PastePlugin>) -> Result<Vec<u8>, String> {
  // Get the last window from state
  let last_window = {
    let guard = state.last_window.lock().unwrap();
    guard.clone()
  };
  
  if let Some(win) = last_window {
    if let Some(hicon) = get_window_icon(&win) {
      return hicon_to_png(hicon).map_err(|e| e.to_string());
    }
  }
  
  Err("No icon found for window".to_string())
}

/// Cross-platform helper function
pub fn activate(window: &WindowInfo) -> Result<()> {
  #[cfg(target_os = "windows")]
  {
    let hwnd = HWND(window.id as usize as *mut c_void);

    unsafe {
      println!("Activating window: {}", window.title);
      let _ = ShowWindow(hwnd, SW_RESTORE);


      let fg_hwnd = GetForegroundWindow();
      let mut _fg_pid: u32 = 0;
      let fg_tid = GetWindowThreadProcessId(fg_hwnd, Some(&mut _fg_pid));
      let my_tid = GetCurrentThreadId();

      let _ = AttachThreadInput(my_tid, fg_tid, true);
      let ok = SetForegroundWindow(hwnd).as_bool();

      let _ = AttachThreadInput(my_tid, fg_tid, false);

      if !ok {
        println!("Failed to set foreground window: {}", window.title);
        bail!("Still couldn't set foreground (focus-stealing prevented)");
      }
    }
  }

  #[cfg(target_os = "macos")]
  {
    use libc::pid_t;
    use objc2_app_kit::{NSApplicationActivationOptions, NSRunningApplication};

    let pid = window.info.process_id as pid_t;
    unsafe {
      if let Some(app) = NSRunningApplication::runningApplicationWithProcessIdentifier(pid) {
        app.activateWithOptions(NSApplicationActivationOptions::empty());
      }
    }
  }

  #[cfg(all(unix, not(target_os = "macos")))] // X11 path
  {
    use x11rb::protocol::xproto::{self, ConnectionExt as _};
    use x11rb::xcb_ffi::XCBConnection;
    let conn = XCBConnection::connect(None)?.0;
    let root = conn.setup().roots[0].root;
    let win = window.handle as u32;

    // _NET_ACTIVE_WINDOW message (EWMH)
    conn.send_event(
      false,
      root,
      xproto::EventMask::SUBSTRUCTURE_REDIRECT | xproto::EventMask::SUBSTRUCTURE_NOTIFY,
      xproto::ClientMessageEvent::new(
        32,
        win,
        conn.atom("_NET_ACTIVE_WINDOW")?,
        x11rb::CURRENT_TIME,
        0,
        0,
        0,
        0,
      )
      .serialize(),
    )?;
    conn.flush()?;
  }

  Ok(())
}


