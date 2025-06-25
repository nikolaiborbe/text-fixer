use anyhow::{bail, Result};
use std::ffi::c_void;
use x_win::WindowInfo;

use windows::Win32::{
  Foundation::{HWND, LPARAM, WPARAM, LRESULT},
  System::Threading::{AttachThreadInput, GetCurrentThreadId},
  UI::WindowsAndMessaging::{
    GetForegroundWindow, GetWindowThreadProcessId, SendMessageW, SetForegroundWindow, ShowWindow,
    ICON_SMALL, SW_RESTORE, WM_GETICON, HICON
  },
};

pub fn get_window_icon(window: &WindowInfo) -> Option<HICON> {
  let hwnd = HWND(window.id as usize as *mut c_void);
  let icon: LRESULT = unsafe {
    SendMessageW(
      hwnd,
      WM_GETICON,
      Some(WPARAM(ICON_SMALL as usize)),
      Some(LPARAM(0)),
    )
  };
  if icon.0 != 0{
    return Some(HICON(icon.0 as *mut c_void))
  } else {
    None
  }
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

#[cfg(target_os = "windows")]
fn hicon_to_png(hicon: HICON) -> anyhow::Result<Vec<u8>> {
    use windows::Win32::Graphics::Gdi::{GetIconInfo, ICONINFO, BITMAP, GetObjectW, GetDC, CreateCompatibleDC, SelectObject, GetDIBits, DeleteDC, DeleteObject, BITMAPINFO, BITMAPINFOHEADER, BI_RGB, DIB_RGB_COLORS};
    use windows::Win32::Foundation::{HBITMAP, BOOL};
    use std::ptr::null_mut;
    use std::mem::{size_of, zeroed};
    use image::{ImageBuffer, Rgba};

    unsafe {
        // 1. Get icon info
        let mut icon_info: ICONINFO = zeroed();
        if !GetIconInfo(hicon, &mut icon_info).as_bool() {
            anyhow::bail!("GetIconInfo failed");
        }
        let hbm_color = icon_info.hbmColor;
        let hbm_mask = icon_info.hbmMask;

        // 2. Get bitmap info
        let mut bmp: BITMAP = zeroed();
        if GetObjectW(hbm_color, size_of::<BITMAP>() as i32, &mut bmp as *mut _ as *mut c_void) == 0 {
            DeleteObject(hbm_color.0 as _);
            DeleteObject(hbm_mask.0 as _);
            anyhow::bail!("GetObjectW failed");
        }
        let (width, height) = (bmp.bmWidth as u32, bmp.bmHeight as u32);
        let dc = GetDC(HWND(0));
        let mem_dc = CreateCompatibleDC(dc);
        let old_obj = SelectObject(mem_dc, hbm_color);

        // 3. Prepare BITMAPINFO
        let mut bi = BITMAPINFO {
            bmiHeader: BITMAPINFOHEADER {
                biSize: size_of::<BITMAPINFOHEADER>() as u32,
                biWidth: width as i32,
                biHeight: -(height as i32), // top-down
                biPlanes: 1,
                biBitCount: 32,
                biCompression: BI_RGB as u32,
                biSizeImage: 0,
                biXPelsPerMeter: 0,
                biYPelsPerMeter: 0,
                biClrUsed: 0,
                biClrImportant: 0,
            },
            bmiColors: [zeroed(); 1],
        };
        let mut pixels = vec![0u8; (width * height * 4) as usize];
        let res = GetDIBits(
            mem_dc,
            hbm_color,
            0,
            height as u32,
            Some(pixels.as_mut_ptr() as *mut c_void),
            &mut bi,
            DIB_RGB_COLORS,
        );
        // Cleanup
        SelectObject(mem_dc, old_obj);
        DeleteDC(mem_dc);
        DeleteObject(hbm_color.0 as _);
        DeleteObject(hbm_mask.0 as _);
        if res == 0 {
            anyhow::bail!("GetDIBits failed");
        }
        // 4. Convert BGRA to RGBA
        for chunk in pixels.chunks_exact_mut(4) {
            let b = chunk[0];
            let g = chunk[1];
            let r = chunk[2];
            let a = chunk[3];
            chunk[0] = r;
            chunk[1] = g;
            chunk[2] = b;
            chunk[3] = a;
        }
        // 5. Encode as PNG
        let img_buf: ImageBuffer<Rgba<u8>, _> = ImageBuffer::from_raw(width, height, pixels)
            .ok_or_else(|| anyhow::anyhow!("Failed to create image buffer"))?;
        let mut png_bytes = Vec::new();
        image::codecs::png::PngEncoder::new(&mut png_bytes)
            .encode(
                &img_buf,
                width,
                height,
                image::ColorType::Rgba8,
            )?;
        Ok(png_bytes)
    }
}
