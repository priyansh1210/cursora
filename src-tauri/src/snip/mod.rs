//! Vision snip (spec 10): grab the screen, let the user pick a region, and hand it to a model that can see.
#[cfg(not(windows))]
mod linux;
#[cfg(windows)]
mod windows;

#[cfg(not(windows))]
use linux as os;
#[cfg(windows)]
use self::windows as os;

mod overlay;

use crate::bubble::{self, Rect};
use crate::capture::{self, Capture, Content, Method, Notice, Platform, SourceApp};
use crate::chat::{self, Image, Locality};
use crate::llm::{self, ModelRef, ModelSettings};
use crate::session::{self, Session};
use crate::settings::Settings;
use crate::AppState;
use base64::Engine as _;
use image::codecs::png::PngEncoder;
use image::imageops::FilterType;
use image::{ExtendedColorType, ImageEncoder, RgbImage};
use serde::Deserialize;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;
use std::time::{Duration, Instant};
use tauri::{AppHandle, Emitter, Manager, WebviewWindow};

pub use overlay::LABEL;

/// The thumbnail's long side: what the page shows and history keeps (VIS-2.6).
pub const THUMBNAIL: u32 = 256;

/// The whole virtual screen as the OS gave it: 4 bytes a pixel, blue first (BGRA or BGRX), rows top down.
pub struct Grab {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
    pub bgra: Vec<u8>,
}

/// The image for the model, and its thumbnail (VIS-2.2, 2.6).
pub struct Prepared {
    pub image: Image,
    pub thumbnail: Image,
    pub width: u32,
    pub height: u32,
}

/// A rectangle under 8×8 px is a cancel (VIS-2.3).
pub fn too_small(r: &Rect) -> bool {
    r.w < 8 || r.h < 8
}

/// Which model a snip goes to (VIS-1.2).
#[derive(Debug, PartialEq)]
pub enum SnipModel {
    /// The active model can see.
    Active,
    /// It can't, but this configured one can, and local-only allows it: offered as a one-time switch.
    Other(ModelRef),
    /// No model can: the bubble points to Models.
    None,
}

pub fn snip_model(active_sees: bool, m: &ModelSettings, local_only: bool) -> SnipModel {
    if active_sees {
        return SnipModel::Active;
    }
    let allowed = |p: &&llm::Provider| !(local_only && p.locality(&m.trusted_hosts) == Locality::Cloud);
    let other = m.providers.iter().filter(allowed).find_map(|p| p.models.iter().find(|x| llm::vision(p, x)).map(|x| ModelRef { provider: p.id.clone(), model: x.clone() }));
    other.map_or(SnipModel::None, SnipModel::Other)
}

/// `r` cut from the grab (clipped to it), downscaled to `max_side` on the long side, as PNG, with its thumbnail.
pub fn prepare(grab: &Grab, r: Rect, max_side: u32) -> Result<Prepared, String> {
    // The overlap of the rectangle and the grab.
    let (left, top) = (r.x.max(grab.x), r.y.max(grab.y));
    let (right, bottom) = ((r.x + r.w).min(grab.x + grab.width as i32), (r.y + r.h).min(grab.y + grab.height as i32));
    if right <= left || bottom <= top {
        return Err("The region is off the screen.".into());
    }
    let (x0, y0, w, h) = ((left - grab.x) as u32, (top - grab.y) as u32, (right - left) as u32, (bottom - top) as u32);
    let mut rgb = RgbImage::new(w, h);
    for (y, row) in rgb.rows_mut().enumerate() {
        let start = ((y0 as usize + y) * grab.width as usize + x0 as usize) * 4;
        for (px, bgra) in row.zip(grab.bgra[start..start + w as usize * 4].chunks_exact(4)) {
            px.0 = [bgra[2], bgra[1], bgra[0]];
        }
    }
    let image = fit(rgb, max_side);
    let thumbnail = fit(image.clone(), THUMBNAIL);
    Ok(Prepared { width: image.width(), height: image.height(), image: png(&image)?, thumbnail: png(&thumbnail)? })
}

/// Downscaled so the long side is at most `max`, keeping the shape; never enlarged.
pub(crate) fn fit(img: RgbImage, max: u32) -> RgbImage {
    let max = max.clamp(THUMBNAIL, 8192); // a typo in settings.json can't send a 1-px or a 100-MB image
    let long = img.width().max(img.height());
    if long <= max {
        return img;
    }
    let scale = |v: u32| ((v as u64 * max as u64 + long as u64 / 2) / long as u64).max(1) as u32;
    image::imageops::resize(&img, scale(img.width()), scale(img.height()), FilterType::Triangle)
}

pub(crate) fn png(img: &RgbImage) -> Result<Image, String> {
    let mut bytes = Vec::new();
    PngEncoder::new(&mut bytes).write_image(img.as_raw(), img.width(), img.height(), ExtendedColorType::Rgb8).map_err(|e| e.to_string())?;
    Ok(Image { mime: "image/png".into(), base64: base64::engine::general_purpose::STANDARD.encode(bytes) })
}

/// A snip in progress: the frozen screen, where it was asked from, and what to do with the region.
struct Pending {
    grab: Grab,
    source: SourceApp,
    model: Option<ModelRef>,
    question: Option<String>,
}

static PENDING: Mutex<Option<Pending>> = Mutex::new(None);

/// A snip is under way, from its start until the bubble opens or it's cancelled. Another start meanwhile is ignored:
/// a double click, or X11 repeating a held hotkey, would otherwise grab twice and leave the overlay without a grab.
static SNIPPING: AtomicBool = AtomicBool::new(false);

fn done() {
    SNIPPING.store(false, Ordering::SeqCst);
}

/// How a snip was asked for: the bubble's one-time switch to a model that can see, and `/snip <question>`.
#[derive(Default)]
pub struct Start {
    pub model: Option<ModelRef>,
    pub question: Option<String>,
}

/// What asking for a snip leads to (VIS-1.2, VIS-3.1).
enum Begin {
    Blocked,
    NeedsVision(Option<ModelRef>),
    Grabbed(Grab),
    Failed(String),
}

/// The checks before anything is grabbed: the blocklist first (VIS-3.1), then a model that can see (VIS-1.2).
fn begin(source: &SourceApp, s: &Settings, sees: bool, grab: impl FnOnce() -> Result<Grab, String>) -> Begin {
    if capture::blocked(&s.capture.blocklist, source) {
        return Begin::Blocked;
    }
    match snip_model(sees, &s.models, s.local_only) {
        SnipModel::Active => grab().map_or_else(Begin::Failed, Begin::Grabbed),
        SnipModel::Other(m) => Begin::NeedsVision(Some(m)),
        SnipModel::None => Begin::NeedsVision(None),
    }
}

/// Starts a snip (VIS-2.1). The work runs off the main thread: it waits for the bubble to go, then grabs the screen.
pub fn start(app: &AppHandle, how: Start) {
    let s = app.state::<AppState>().settings.lock().unwrap().clone();
    if s.paused {
        return log::info!("paused: snip ignored"); // APP-1.3
    }
    if SNIPPING.swap(true, Ordering::SeqCst) {
        return; // one at a time
    }
    let cursor = app.cursor_position().ok().filter(|_| session::current() != Session::Wayland).map(|p| (p.x.round() as i32, p.y.round() as i32));
    let app = app.clone();
    std::thread::spawn(move || {
        let mut source = capture::OsPlatform::new().foreground(); // before a Cursora window can take focus
        let sees = how.model.is_some() || llm::active(&app).is_ok_and(|m| m.info().vision);
        // Wayland: the grab is the screenshot the user copied, so there is nothing to hide and no overlay (VIS-4.2).
        let wayland = session::current() == Session::Wayland;
        let grab = || {
            if !wayland {
                bubble::hide(&app); // neither the bubble nor the overlay may be in the grab (VIS-2.5)
                std::thread::sleep(Duration::from_millis(150));
            }
            os::grab()
        };
        let notice = match begin(&source, &s, sees, grab) {
            Begin::Grabbed(grab) if wayland => {
                let whole = Rect { x: 0, y: 0, w: grab.width as i32, h: grab.height as i32 };
                finish(&app, Pending { grab, source, model: how.model, question: how.question }, whole, None);
                return done();
            }
            Begin::Grabbed(grab) => {
                log::info!("snip: grabbed {}×{} from {}", grab.width, grab.height, source.exe);
                let shown = (grab.x, grab.y, grab.width, grab.height);
                *PENDING.lock().unwrap() = Some(Pending { grab, source, model: how.model, question: how.question });
                if let Err(e) = overlay::show(&app, shown) {
                    PENDING.lock().unwrap().take();
                    log::warn!("snip: overlay: {e}");
                    done();
                }
                return; // the overlay ends it: snip_done, or the window closed another way
            }
            Begin::Blocked => {
                source.title.clear(); // not shown, not saved, as for Ask
                Notice::Blocked
            }
            Begin::NeedsVision(offer) => Notice::NeedsVision { offer: offer.map(|m| offer_for(&s.models, m)) },
            Begin::Failed(reason) => Notice::SnipFailed { reason },
        };
        open(&app, Capture { app: source, method: Method::None, content: Content::None, notice: Some(notice) }, cursor, None);
        done();
    });
}

/// "llava-1.6 (LM Studio)", for the one-time switch's button.
fn offer_for(m: &ModelSettings, model: ModelRef) -> capture::VisionOffer {
    let provider = m.providers.iter().find(|p| p.id == model.provider).map_or(model.provider.as_str(), |p| p.name.as_str());
    capture::VisionOffer { label: format!("{} ({provider})", model.model), model }
}

/// A new conversation in the bubble; `model` is the one-time switch, which leaves the default alone.
fn open(app: &AppHandle, capture: Capture, cursor: Option<(i32, i32)>, model: Option<ModelRef>) {
    let previous = capture.app.window;
    let mut conv = chat::start(app, capture);
    if let Some(m) = model {
        conv = chat::own_model(app, bubble::LABEL, m).unwrap_or(conv);
    }
    bubble::open(app, cursor, previous, conv);
}

/// The overlay's rectangle: CSS pixels from its top-left corner.
#[derive(Deserialize)]
pub struct CssRect {
    x: f64,
    y: f64,
    w: f64,
    h: f64,
}

/// In screen pixels. The overlay sits at the grab's origin and is exactly as large, so one scale maps it,
/// even across monitors with different scales: Windows lays a window out in physical pixels (VIS-2.4).
fn to_screen(r: &CssRect, scale: f64, origin: (i32, i32)) -> Rect {
    let px = |v: f64| (v * scale).round() as i32;
    Rect { x: origin.0 + px(r.x), y: origin.1 + px(r.y), w: px(r.x + r.w) - px(r.x), h: px(r.y + r.h) - px(r.y) }
}

/// The overlay picked a region, or cancelled (`None`): Esc, a right-click, or under 8×8 px (VIS-2.3).
#[tauri::command(async)]
pub fn snip_done(window: WebviewWindow, app: AppHandle, rect: Option<CssRect>) {
    let scale = window.scale_factor().unwrap_or(1.0);
    let pending = PENDING.lock().unwrap().take(); // taken first, so the window's Destroyed event leaves the end to us
    let _ = window.destroy();
    if let Some(p) = pending {
        match rect.map(|r| to_screen(&r, scale, (p.grab.x, p.grab.y))).filter(|r| !too_small(r)) {
            Some(r) => finish(&app, p, r, Some((r.x + r.w, r.y + r.h))), // the bubble opens at the rectangle's corner, by BUB-1's rules (VIS-2.2)
            None => log::info!("snip: cancelled"),
        }
    }
    done();
}

/// The region becomes the image of a new conversation; `/snip <question>` asks it at once.
fn finish(app: &AppHandle, p: Pending, r: Rect, corner: Option<(i32, i32)>) {
    let started = Instant::now();
    let max_side = app.state::<AppState>().settings.lock().unwrap().snip.max_side;
    match prepare(&p.grab, r, max_side) {
        Ok(img) => {
            log::info!("snip: {}×{} PNG in {} ms", img.width, img.height, started.elapsed().as_millis());
            let content = Content::Snip(capture::Snip { thumbnail: img.thumbnail, width: img.width, height: img.height, full: Some(img.image) });
            open(&app, Capture { app: p.source, method: Method::None, content, notice: None }, corner, p.model);
            if let Some(q) = p.question.filter(|q| !q.trim().is_empty()) {
                let _ = app.emit_to(bubble::LABEL, "snip-ask", q); // `/snip <question>`: sent as if typed, so confirming still applies
            }
        }
        Err(reason) => open(&app, Capture { app: p.source, method: Method::None, content: Content::None, notice: Some(Notice::SnipFailed { reason }) }, corner, None),
    }
}

/// From the bubble: "Snip an area instead", `/snip`, and the one-time switch (VIS-1.2).
#[tauri::command]
pub fn snip_start(app: AppHandle, model: Option<ModelRef>, question: Option<String>) {
    start(&app, Start { model, question });
}

/// The frozen screen, for the overlay's page only, while a snip is in progress.
pub fn screen_bmp() -> Option<Vec<u8>> {
    PENDING.lock().unwrap().as_ref().map(|p| bmp(&p.grab))
}

/// A 32-bit top-down BMP straight from the grab's rows: no encoding, nothing on disk.
fn bmp(g: &Grab) -> Vec<u8> {
    const HEADERS: u32 = 14 + 40;
    let size = g.bgra.len() as u32;
    let mut out = Vec::with_capacity(HEADERS as usize + g.bgra.len());
    out.extend(b"BM");
    out.extend((HEADERS + size).to_le_bytes());
    out.extend([0; 4]);
    out.extend(HEADERS.to_le_bytes());
    out.extend(40u32.to_le_bytes());
    out.extend((g.width as i32).to_le_bytes());
    out.extend((-(g.height as i32)).to_le_bytes()); // negative: top-down rows
    out.extend(1u16.to_le_bytes());
    out.extend(32u16.to_le_bytes());
    out.extend(0u32.to_le_bytes()); // BI_RGB
    out.extend(size.to_le_bytes());
    out.extend([0; 16]);
    out.extend(&g.bgra);
    // The 4th byte is padding on X11 and usually 0 on Windows; a browser could take it for transparency.
    out[HEADERS as usize..].iter_mut().skip(3).step_by(4).for_each(|a| *a = 255);
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::GenericImageView;
    use serde_json::json;

    /// A screen whose left edge is at x = -100, as with a second monitor left of the primary one.
    fn grab(width: u32, height: u32) -> Grab {
        Grab { x: -100, y: 0, width, height, bgra: vec![0; (width * height * 4) as usize] }
    }

    fn decoded(i: &Image) -> image::DynamicImage {
        image::load_from_memory(&base64::engine::general_purpose::STANDARD.decode(&i.base64).unwrap()).unwrap()
    }

    #[test]
    fn vis_2_2_the_region_is_cut_downscaled_on_its_long_side_and_encoded_as_png() {
        let mut g = grab(2100, 500);
        g.bgra[(20 * 2100 + 10) * 4] = 255; // a blue pixel at screen (-90, 20)
        let whole = prepare(&g, Rect { x: -100, y: 0, w: 2100, h: 500 }, 2048).unwrap();
        assert_eq!((whole.width, whole.height, whole.image.mime.as_str()), (2048, 488, "image/png"));
        assert_eq!(decoded(&whole.image).dimensions(), (2048, 488));
        assert_eq!(decoded(&whole.thumbnail).dimensions(), (256, 61));
        let small = prepare(&g, Rect { x: -90, y: 20, w: 30, h: 10 }, 2048).unwrap();
        assert_eq!((small.width, small.height), (30, 10), "never enlarged");
        assert_eq!(decoded(&small.image).to_rgb8().get_pixel(0, 0).0, [0, 0, 255], "blue stays blue");
        let edge = prepare(&g, Rect { x: 1900, y: 400, w: 500, h: 500 }, 2048).unwrap();
        assert_eq!((edge.width, edge.height), (100, 100), "clipped to the screen");
        let left = prepare(&g, Rect { x: -150, y: -10, w: 100, h: 50 }, 2048).unwrap();
        assert_eq!((left.width, left.height), (50, 40), "clipped on the left and top too, not shifted");
        assert!(prepare(&g, Rect { x: 5000, y: 0, w: 50, h: 50 }, 2048).is_err());
        assert_eq!(prepare(&g, Rect { x: -100, y: 0, w: 2100, h: 500 }, 1).unwrap().width, 256, "max side kept sane");
    }

    fn app(exe: &str) -> SourceApp {
        SourceApp { exe: exe.into(), name: exe.into(), ..Default::default() }
    }

    #[test]
    fn vis_3_1_a_blocked_app_in_front_gets_the_notice_and_nothing_is_grabbed() {
        let s = Settings::default(); // password managers are blocked by default (PRI-2.3)
        let never = || -> Result<Grab, String> { panic!("grabbed in a blocked app") };
        assert!(matches!(begin(&app("KeePassXC.exe"), &s, true, never), Begin::Blocked));
        assert!(matches!(begin(&app("notepad.exe"), &s, true, || Ok(grab(10, 10))), Begin::Grabbed(_)));
        assert!(matches!(begin(&app("notepad.exe"), &s, true, || Err("no screen".into())), Begin::Failed(e) if e == "no screen"));
    }

    #[test]
    fn vis_1_2_without_a_model_that_can_see_nothing_is_grabbed() {
        let never = || -> Result<Grab, String> { panic!("grabbed without vision") };
        assert!(matches!(begin(&app("notepad.exe"), &Settings::default(), false, never), Begin::NeedsVision(None)));
        let mut s = Settings::default();
        s.models = serde_json::from_value(json!({ "providers": [{ "id": "p-lm", "name": "LM Studio", "kind": "openai-compatible", "baseUrl": "http://localhost:1234/v1", "models": ["llava-1.6"] }] })).unwrap();
        let Begin::NeedsVision(Some(m)) = begin(&app("notepad.exe"), &s, false, never) else { panic!("no offer") };
        assert_eq!(offer_for(&s.models, m).label, "llava-1.6 (LM Studio)");
    }

    #[test]
    fn vis_2_4_the_overlay_rectangle_maps_to_screen_pixels_with_one_scale() {
        let r = CssRect { x: 10.0, y: 20.0, w: 100.0, h: 50.0 };
        assert_eq!(to_screen(&r, 1.5, (-1920, 0)), Rect { x: -1905, y: 30, w: 150, h: 75 });
        assert_eq!(to_screen(&r, 1.0, (0, 0)), Rect { x: 10, y: 20, w: 100, h: 50 });
        let r = CssRect { x: 0.4, y: 0.4, w: 5.0, h: 5.0 }; // edges round the same way, so the width can't drift
        assert_eq!(to_screen(&r, 1.25, (0, 0)), Rect { x: 1, y: 1, w: 6, h: 6 });
    }

    #[test]
    fn vis_2_1_the_overlay_gets_the_frozen_screen_as_an_opaque_top_down_bmp() {
        let mut g = grab(3, 2);
        g.bgra[0..4].copy_from_slice(&[1, 2, 3, 0]);
        let b = bmp(&g);
        assert_eq!(&b[0..2], b"BM");
        assert_eq!(b.len(), 54 + 3 * 2 * 4);
        assert_eq!(i32::from_le_bytes(b[22..26].try_into().unwrap()), -2, "top-down");
        assert_eq!(&b[54..58], [1, 2, 3, 255], "blue first, opaque");
    }

    #[test]
    fn vis_2_6_the_snip_code_never_writes_the_screen_or_the_image_anywhere() {
        let fs = ["std::", "fs"].concat();
        for (file, code) in [("mod.rs", include_str!("mod.rs")), ("overlay.rs", include_str!("overlay.rs")), ("windows.rs", include_str!("windows.rs")), ("linux.rs", include_str!("linux.rs"))] {
            assert!(!code.contains(&fs) && !code.contains(&["File", "::"].concat()), "{file} touches files");
        }
    }

    #[test]
    fn vis_2_3_a_rectangle_under_8_by_8_is_a_cancel() {
        let r = |w, h| Rect { x: 0, y: 0, w, h };
        assert!(too_small(&r(7, 100)) && too_small(&r(100, 7)) && too_small(&r(0, 0)));
        assert!(!too_small(&r(8, 8)));
    }

    #[test]
    fn vis_1_2_another_model_that_can_see_is_offered_if_local_only_allows_it() {
        let models: ModelSettings = serde_json::from_value(json!({ "providers": [
            { "id": "p-cloud", "name": "OpenAI", "kind": "openai-compatible", "baseUrl": "https://api.openai.com/v1", "models": ["gpt-4o"] },
            { "id": "p-lm", "name": "LM Studio", "kind": "openai-compatible", "baseUrl": "http://localhost:1234/v1", "models": ["qwen2.5-7b", "llava-1.6"] },
        ]}))
        .unwrap();
        let other = |provider: &str, model: &str| SnipModel::Other(ModelRef { provider: provider.into(), model: model.into() });
        assert_eq!(snip_model(true, &models, false), SnipModel::Active);
        assert_eq!(snip_model(false, &models, false), other("p-cloud", "gpt-4o"));
        assert_eq!(snip_model(false, &models, true), other("p-lm", "llava-1.6"), "local-only skips the cloud one");
        let cloud_only = ModelSettings { providers: models.providers[..1].to_vec(), ..models.clone() };
        assert_eq!(snip_model(false, &cloud_only, true), SnipModel::None);
        assert_eq!(snip_model(false, &ModelSettings::default(), false), SnipModel::None);
    }
}
