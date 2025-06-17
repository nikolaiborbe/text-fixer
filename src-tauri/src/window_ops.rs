use anyhow::{Result};
use x_win::WindowInfo;

/// Cross-platform helper function
pub fn activate(window: &WindowInfo) -> Result<()> {
  #[cfg(target_os = "windows")]
  {
    use windows::Win32::UI::WindowsAndMessaging::{SetForegroundWindow, ShowWindow, SW_RESTORE};
    let hwnd = windows::Win32::Foundation::HWND(window.handle as isize);
    // If the window is minimised, restore it first.
    unsafe { ShowWindow(hwnd, SW_RESTORE) };
    let ok = unsafe { SetForegroundWindow(hwnd).as_bool() };
    if !ok {
      bail!("SetForegroundWindow failed (maybe running as non-interactive session?)");
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
