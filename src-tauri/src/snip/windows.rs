//! The grab on Windows: the whole virtual screen (every monitor), in physical pixels, since the app is per-monitor DPI aware.
use super::Grab;
use windows::Win32::Graphics::Gdi::{
    BitBlt, CreateCompatibleBitmap, CreateCompatibleDC, DeleteDC, DeleteObject, GetDC, GetDIBits, ReleaseDC, SelectObject, BITMAPINFO, BITMAPINFOHEADER,
    BI_RGB, CAPTUREBLT, DIB_RGB_COLORS, SRCCOPY,
};
use windows::Win32::UI::WindowsAndMessaging::{GetSystemMetrics, SM_CXVIRTUALSCREEN, SM_CYVIRTUALSCREEN, SM_XVIRTUALSCREEN, SM_YVIRTUALSCREEN};

pub fn grab() -> Result<Grab, String> {
    let metric = |m| unsafe { GetSystemMetrics(m) };
    let (x, y) = (metric(SM_XVIRTUALSCREEN), metric(SM_YVIRTUALSCREEN));
    let (w, h) = (metric(SM_CXVIRTUALSCREEN), metric(SM_CYVIRTUALSCREEN));
    if w <= 0 || h <= 0 {
        return Err("Windows reported no screen.".into());
    }
    let mut bgra = vec![0u8; w as usize * h as usize * 4];
    // CAPTUREBLT includes layered windows (tooltips, menus with shadows); the copy is BGRA rows, top down.
    let copied = unsafe {
        let screen = GetDC(None);
        let mem = CreateCompatibleDC(Some(screen));
        let bitmap = CreateCompatibleBitmap(screen, w, h);
        let old = SelectObject(mem, bitmap.into());
        let blt = BitBlt(mem, 0, 0, w, h, Some(screen), x, y, SRCCOPY | CAPTUREBLT);
        let header = BITMAPINFOHEADER { biSize: size_of::<BITMAPINFOHEADER>() as u32, biWidth: w, biHeight: -h, biPlanes: 1, biBitCount: 32, biCompression: BI_RGB.0, ..Default::default() };
        let mut info = BITMAPINFO { bmiHeader: header, ..Default::default() };
        let lines = GetDIBits(mem, bitmap, 0, h as u32, Some(bgra.as_mut_ptr().cast()), &mut info, DIB_RGB_COLORS);
        SelectObject(mem, old);
        let _ = DeleteObject(bitmap.into());
        let _ = DeleteDC(mem);
        ReleaseDC(None, screen);
        blt.is_ok() && lines == h
    };
    if !copied {
        return Err("Windows didn't let Cursora copy the screen.".into());
    }
    Ok(Grab { x, y, width: w as u32, height: h as u32, bgra })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn vis_2_4_the_whole_virtual_screen_is_grabbed_on_windows() {
        let g = grab().unwrap();
        let metric = |m| unsafe { GetSystemMetrics(m) };
        assert_eq!((g.x, g.y, g.width as i32, g.height as i32), (metric(SM_XVIRTUALSCREEN), metric(SM_YVIRTUALSCREEN), metric(SM_CXVIRTUALSCREEN), metric(SM_CYVIRTUALSCREEN)));
        assert_eq!(g.bgra.len(), (g.width * g.height * 4) as usize);
    }
}
