use crate::chat::Image;
use crate::settings::CaptureSettings;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::{fs, time::UNIX_EPOCH};

const MAX_ITEMS: usize = 20;
const MAX_CHILDREN: usize = 50;
/// Images a model that can see gets with selected files (VIS-2.7).
const MAX_IMAGES: usize = 4;
const IMAGE_MAX_BYTES: u64 = 20_000_000;
const IMAGE_TYPES: [&str; 5] = ["png", "jpg", "jpeg", "webp", "gif"];

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Default)]
pub struct FileContext {
    pub items: Vec<FileItem>,
    /// Selected items beyond the first 20 (SEL-5.5).
    pub omitted: usize,
    /// The attached images, in item order (VIS-2.7). Never serialized: history keeps the file list only.
    #[serde(skip)]
    pub images: Vec<Image>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct FileItem {
    pub path: PathBuf,
    pub name: String,
    pub kind: Kind,
    pub size: u64,
    /// Unix seconds.
    pub modified: u64,
    pub children: Vec<String>,
    pub content: Option<String>,
    /// Why `content` is missing, for the model and the bubble.
    pub skipped: Option<String>,
    /// Attached as an image for a model that can see (VIS-2.7).
    #[serde(default)]
    pub image: bool,
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Clone, Copy)]
#[serde(rename_all = "lowercase")]
pub enum Kind {
    File,
    Folder,
    Link,
}

/// Metadata for each selected item, plus contents of small text files (SEL-5.2, 5.3, 5.5, 5.6).
pub fn file_context(paths: &[PathBuf], cfg: &CaptureSettings) -> FileContext {
    let mut budget = cfg.files_total_bytes;
    let items = paths.iter().take(MAX_ITEMS).map(|p| item(p, cfg.file_max_bytes, &mut budget)).collect();
    FileContext { items, omitted: paths.len().saturating_sub(MAX_ITEMS), images: Vec::new() }
}

/// For a model that can see: up to 4 image files of up to 20 MB, downscaled to `max_side` (VIS-2.7).
/// One that won't decode, a 5th, or a larger one stays metadata only.
pub fn attach_images(ctx: &mut FileContext, max_side: u32) {
    for it in &mut ctx.items {
        let is_image = it.path.extension().is_some_and(|e| IMAGE_TYPES.iter().any(|t| e.eq_ignore_ascii_case(t)));
        if ctx.images.len() == MAX_IMAGES || it.kind != Kind::File || !is_image || it.size > IMAGE_MAX_BYTES {
            continue;
        }
        // The reader's default limits refuse an image that would need over 512 MB to decode.
        let decoded = image::ImageReader::open(&it.path).ok().and_then(|r| r.with_guessed_format().ok()?.decode().ok());
        match decoded.map(|img| crate::snip::png(&crate::snip::fit(img.to_rgb8(), max_side))) {
            Some(Ok(png)) => {
                ctx.images.push(png);
                it.image = true;
                it.skipped = None;
            }
            _ => it.skipped = Some("an image that couldn't be read".into()),
        }
    }
}

fn item(path: &Path, max_bytes: u64, budget: &mut u64) -> FileItem {
    let name = path.file_name().map_or_else(|| path.display().to_string(), |n| n.to_string_lossy().into_owned());
    let mut it = FileItem { path: path.into(), name, kind: Kind::File, size: 0, modified: 0, children: Vec::new(), content: None, skipped: None, image: false };
    // symlink_metadata never follows links or junctions out of the selection (SEL-5.6).
    let meta = match fs::symlink_metadata(path) {
        Ok(m) => m,
        Err(e) => {
            it.skipped = Some(format!("unreadable: {e}"));
            return it;
        }
    };
    it.size = meta.len();
    it.modified = meta.modified().ok().and_then(|t| t.duration_since(UNIX_EPOCH).ok()).map_or(0, |d| d.as_secs());
    if meta.file_type().is_symlink() {
        it.kind = Kind::Link;
        it.skipped = Some("link, not followed".into());
    } else if meta.is_dir() {
        it.kind = Kind::Folder;
        it.children = fs::read_dir(path)
            .map(|rd| rd.filter_map(Result::ok).take(MAX_CHILDREN).map(|e| e.file_name().to_string_lossy().into_owned()).collect())
            .unwrap_or_default();
    } else if it.size > max_bytes {
        it.skipped = Some(format!("larger than {} KB", max_bytes / 1000));
    } else if it.size > *budget {
        it.skipped = Some("total size limit for selected files reached".into());
    } else {
        match read_text(path) {
            Some(text) => {
                *budget -= it.size;
                it.content = Some(text);
            }
            None => it.skipped = Some("not a text file".into()),
        }
    }
    it
}

/// Text means valid UTF-16 (with BOM) or UTF-8 without NUL bytes; anything else is treated as binary.
fn read_text(path: &Path) -> Option<String> {
    let bytes = fs::read(path).ok()?;
    if let Some(rest) = bytes.strip_prefix(&[0xFF, 0xFE]) {
        let units: Vec<u16> = rest.chunks_exact(2).map(|c| u16::from_le_bytes([c[0], c[1]])).collect();
        return String::from_utf16(&units).ok();
    }
    let text = String::from_utf8(bytes).ok()?;
    let text = text.strip_prefix('\u{FEFF}').map(str::to_owned).unwrap_or(text);
    (!text.contains('\0')).then_some(text)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_dir(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("cursora-files-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn sel_5_2_items_carry_metadata_and_folders_list_up_to_50_children() {
        let dir = temp_dir("meta");
        fs::write(dir.join("a.txt"), "hello").unwrap();
        let sub = dir.join("sub");
        fs::create_dir(&sub).unwrap();
        for i in 0..60 {
            fs::write(sub.join(format!("f{i}.txt")), "").unwrap();
        }
        let ctx = file_context(&[dir.join("a.txt"), sub.clone()], &CaptureSettings::default());
        let (file, folder) = (&ctx.items[0], &ctx.items[1]);
        assert_eq!((file.name.as_str(), file.kind, file.size), ("a.txt", Kind::File, 5));
        assert!(file.modified > 0);
        assert_eq!((folder.kind, folder.children.len()), (Kind::Folder, 50));
    }

    #[test]
    fn sel_5_3_small_text_is_inlined_while_binary_large_and_over_budget_are_skipped() {
        let dir = temp_dir("content");
        fs::write(dir.join("note.md"), "# hi").unwrap();
        fs::write(dir.join("utf16.txt"), [0xFF, 0xFE, b'o', 0, b'k', 0]).unwrap();
        fs::write(dir.join("bin.dat"), [0u8, 159, 146, 150]).unwrap();
        fs::write(dir.join("big.txt"), "x".repeat(200)).unwrap();
        fs::write(dir.join("second.txt"), "y".repeat(81)).unwrap();
        let cfg = CaptureSettings { file_max_bytes: 100, files_total_bytes: 90, ..CaptureSettings::default() };
        let names = ["note.md", "utf16.txt", "bin.dat", "big.txt", "second.txt"];
        let ctx = file_context(&names.map(|n| dir.join(n)), &cfg);
        assert_eq!(ctx.items[0].content.as_deref(), Some("# hi"));
        assert_eq!(ctx.items[1].content.as_deref(), Some("ok"));
        assert_eq!(ctx.items[2].skipped.as_deref(), Some("not a text file"));
        assert!(ctx.items[3].skipped.as_deref().unwrap().starts_with("larger than"));
        // 4 + 6 bytes used leaves 80; 81 more would pass the 90 total
        assert!(ctx.items[4].skipped.as_deref().unwrap().starts_with("total size limit"));
    }

    #[test]
    fn vis_2_7_up_to_4_images_are_attached_downscaled_and_a_broken_one_is_metadata() {
        let dir = temp_dir("images");
        for i in 0..5 {
            image::RgbImage::new(3000, 100).save(dir.join(format!("{i}.png"))).unwrap();
        }
        fs::write(dir.join("broken.jpg"), b"not a jpeg").unwrap();
        fs::write(dir.join("notes.txt"), "text").unwrap();
        let names = ["broken.jpg", "notes.txt", "0.png", "1.png", "2.png", "3.png", "4.png"];
        let mut ctx = file_context(&names.map(|n| dir.join(n)), &CaptureSettings::default());
        assert!(ctx.images.is_empty() && ctx.items.iter().all(|i| !i.image), "a model that can't see gets metadata only");
        attach_images(&mut ctx, 2048);
        let attached: Vec<&str> = ctx.items.iter().filter(|i| i.image).map(|i| i.name.as_str()).collect();
        assert_eq!(attached, ["0.png", "1.png", "2.png", "3.png"]);
        assert_eq!(ctx.images.len(), 4);
        let first = image::load_from_memory(&base64::Engine::decode(&base64::engine::general_purpose::STANDARD, &ctx.images[0].base64).unwrap()).unwrap();
        assert_eq!((first.width(), first.height()), (2048, 68));
        assert_eq!(ctx.items[0].skipped.as_deref(), Some("an image that couldn't be read"));
        assert_eq!(ctx.items[1].content.as_deref(), Some("text"));
        assert!(!serde_json::to_string(&ctx).unwrap().contains(&ctx.images[0].base64), "never serialized");
    }

    #[test]
    fn sel_5_5_only_the_first_20_items_are_included() {
        let dir = temp_dir("many");
        let paths: Vec<PathBuf> = (0..25).map(|i| dir.join(format!("{i}.txt"))).collect();
        let ctx = file_context(&paths, &CaptureSettings::default());
        assert_eq!((ctx.items.len(), ctx.omitted), (20, 5));
    }

    #[test]
    fn sel_5_6_links_are_reported_but_not_followed() {
        let dir = temp_dir("link");
        let target = dir.join("target");
        fs::create_dir(&target).unwrap();
        fs::write(target.join("inside.txt"), "x").unwrap();
        let link = dir.join("link");
        // Junctions need no admin rights on Windows, unlike symlinks.
        #[cfg(windows)]
        let made = std::process::Command::new("cmd").args(["/C", "mklink", "/J"]).arg(&link).arg(&target).output().unwrap().status.success();
        #[cfg(unix)]
        let made = std::os::unix::fs::symlink(&target, &link).is_ok();
        assert!(made, "could not create a link for the test");
        let it = &file_context(&[link], &CaptureSettings::default()).items[0];
        assert_eq!(it.kind, Kind::Link);
        assert!(it.children.is_empty());
    }
}
