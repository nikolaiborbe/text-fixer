use anyhow::{bail, Result};
use std::ffi::c_void;
use x_win::WindowInfo;

use windows::Win32::{
  Foundation::HWND,
  System::Threading::{AttachThreadInput, GetCurrentThreadId},
  UI::WindowsAndMessaging::{
    SetForegroundWindow, GetForegroundWindow, GetWindowThreadProcessId, ShowWindow, SW_RESTORE,
  },
};

/// Cross-platform helper function
pub fn activate(window: &WindowInfo) -> Result<()> {
  #[cfg(target_os = "windows")]
  {
    let hwnd = HWND(window.id as usize as *mut c_void);

    unsafe {
      println!("Activating window: {}", window.title);
      let _ = ShowWindow(hwnd, SW_RESTORE);

      // 2) Get the thread IDs
      let fg_hwnd = GetForegroundWindow();
      let mut _fg_pid: u32 = 0;
      let fg_tid = GetWindowThreadProcessId(fg_hwnd, Some(&mut _fg_pid));
      let my_tid = GetCurrentThreadId();

      // 3) Attach input so we're “allowed” to set focus
      let _ = AttachThreadInput(my_tid, fg_tid, true);
      let ok = SetForegroundWindow(hwnd).as_bool();
      // 4) Detach again
      let _ = AttachThreadInput(my_tid, fg_tid, false);

      if !ok {
        println!("Failed to set foreground window: {}", window.title);
        bail!("Still couldn’t set foreground (focus-stealing prevented)");
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
