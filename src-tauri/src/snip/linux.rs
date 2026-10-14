//! The grab on X11: the root window, which spans every monitor. On Wayland, XWayland can't see other apps' windows,
//! so the snip is a screenshot the user copied with the desktop's own tool (VIS-4.2).
use super::Grab;
use crate::session::{self, Session};
use crate::x11::X11;
use std::time::Duration;
use x11rb::connection::Connection;
use x11rb::protocol::xproto::{ConnectionExt as _, ImageFormat, ImageOrder};

pub fn grab() -> Result<Grab, String> {
    if session::current() == Session::Wayland {
        return copied();
    }
    root()
}

/// What the bubble says on Wayland without a copied screenshot (VIS-4.2).
const HOW: &str = "Copy part of the screen with your desktop's screenshot tool first (on GNOME, the Print key), then press the Snip shortcut again. Cursora reads it with wl-paste, from the wl-clipboard package.";

/// The PNG on the clipboard, as a grab of its own size; the region is then the whole of it.
fn copied() -> Result<Grab, String> {
    // shortcut: a copied screenshot over 64 MB is refused; raise it if 8K screens show up.
    let png = crate::capture::wl_paste(&["--type", "image/png"], 64 << 20, Duration::from_secs(2)).ok_or(HOW)?;
    let rgba = image::load_from_memory_with_format(&png, image::ImageFormat::Png).map_err(|_| HOW.to_string())?.to_rgba8();
    let (width, height) = rgba.dimensions();
    let mut bgra = rgba.into_raw();
    bgra.chunks_exact_mut(4).for_each(|p| p.swap(0, 2));
    Ok(Grab { x: 0, y: 0, width, height, bgra })
}

fn root() -> Result<Grab, String> {
    let x = X11::connect().map_err(|e| format!("Couldn't reach the X server: {e}"))?;
    let geometry = x.conn.get_geometry(x.root).map_err(|e| e.to_string())?.reply().map_err(|e| e.to_string())?;
    // Blue first, 4 bytes a pixel: what depth 24 and 32 give on a little-endian server, i.e. every desktop today.
    let setup = x.conn.setup();
    let bpp = setup.pixmap_formats.iter().find(|f| f.depth == geometry.depth).map(|f| f.bits_per_pixel);
    if bpp != Some(32) || setup.image_byte_order != ImageOrder::LSB_FIRST {
        return Err(format!("This X screen's colour format (depth {}) isn't supported.", geometry.depth));
    }
    let (w, h) = (geometry.width, geometry.height);
    let image = x.conn.get_image(ImageFormat::Z_PIXMAP, x.root, 0, 0, w, h, !0).map_err(|e| e.to_string())?.reply().map_err(|e| e.to_string())?;
    if image.data.len() != w as usize * h as usize * 4 {
        return Err("The X server sent a screen of the wrong size.".into());
    }
    Ok(Grab { x: 0, y: 0, width: w.into(), height: h.into(), bgra: image.data })
}

#[cfg(test)]
mod tests {
    use super::*;
    use x11rb::protocol::xproto::{CreateWindowAux, WindowClass};

    #[test]
    fn vis_4_1_a_window_is_grabbed_in_its_own_colour_on_a_real_x_server() {
        assert!(std::env::var_os("CURSORA_XVFB").is_some(), "the X11 tests need their own X server (scripts/wsl-test.sh, or CURSORA_XVFB=1 xvfb-run -a cargo test)");
        let x = X11::connect().unwrap();
        let window = x.conn.generate_id().unwrap();
        let red = 0xff0000; // TrueColor at depth 24
        let aux = CreateWindowAux::new().background_pixel(red).override_redirect(1);
        x.conn.create_window(0, window, x.root, 40, 30, 20, 10, 0, WindowClass::INPUT_OUTPUT, 0, &aux).unwrap();
        x.conn.map_window(window).unwrap();
        x.conn.get_input_focus().unwrap().reply().unwrap(); // a round trip: the window is mapped and painted
        let g = root().unwrap(); // the X11 grab itself: WSL also has a Wayland display, which grab() would pick
        let at = |px: u32, py: u32| &g.bgra[((py * g.width + px) * 4) as usize..][..3];
        assert_eq!(at(45, 35), [0, 0, 255], "blue, green, red");
        assert_eq!(at(39, 29), [0, 0, 0], "the root around it is black");
    }
}
