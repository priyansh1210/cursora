//! Text recognition for snips (spec 20): the system's own, so a model that can't see still gets a snip's text. The image
//! stays in memory: Windows reads it from a buffer, Tesseract from its standard input (OCR-1.5).
use image::RgbImage;

/// OCR-1.1: whether this computer can read the text in a snip. Only "yes" is remembered, so Tesseract installed while
/// Cursora runs counts at the next snip (OCR-1.6); while it's missing, the check fails at once.
pub fn available() -> bool {
    static AVAILABLE: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);
    use std::sync::atomic::Ordering::Relaxed;
    AVAILABLE.load(Relaxed) || {
        let now = os::available();
        AVAILABLE.store(now, Relaxed);
        now
    }
}

/// The region's text, tidied; empty when there was none (OCR-1.4).
pub fn read(img: &RgbImage) -> Result<String, String> {
    os::read(img).map(|t| tidy(&t))
}

/// Each line trimmed, and at most one empty line between paragraphs.
pub fn tidy(text: &str) -> String {
    let mut out: Vec<&str> = vec![];
    let mut gap = false;
    for line in text.lines().map(str::trim) {
        if line.is_empty() {
            gap = !out.is_empty();
        } else {
            if std::mem::take(&mut gap) {
                out.push("");
            }
            out.push(line);
        }
    }
    out.join("\n")
}

#[cfg(windows)]
mod os {
    use image::RgbImage;
    use windows::Graphics::Imaging::{BitmapPixelFormat, SoftwareBitmap};
    use windows::Media::Ocr::OcrEngine;
    use windows::Storage::Streams::DataWriter;

    /// The engine for the OCR languages installed with Windows. WinRT needs the multithreaded apartment, which the snip's
    /// threads don't set up, so one is kept for the process.
    fn engine() -> windows::core::Result<OcrEngine> {
        static MTA: std::sync::Once = std::sync::Once::new();
        MTA.call_once(|| drop(unsafe { windows::Win32::System::Com::CoIncrementMTAUsage() }));
        OcrEngine::TryCreateFromUserProfileLanguages()
    }

    pub fn available() -> bool {
        engine().is_ok()
    }

    pub fn read(img: &RgbImage) -> Result<String, String> {
        let err = |e: windows::core::Error| format!("Couldn't read the text in the snip: {}", e.message());
        let engine = engine().map_err(err)?;
        let img = crate::snip::fit(img.clone(), OcrEngine::MaxImageDimension().map_err(err)?);
        let bgra: Vec<u8> = img.pixels().flat_map(|p| [p[2], p[1], p[0], 255]).collect();
        let writer = DataWriter::new().map_err(err)?;
        writer.WriteBytes(&bgra).map_err(err)?;
        let bitmap = SoftwareBitmap::CreateCopyFromBuffer(&writer.DetachBuffer().map_err(err)?, BitmapPixelFormat::Bgra8, img.width() as i32, img.height() as i32).map_err(err)?;
        let result = engine.RecognizeAsync(&bitmap).map_err(err)?.join().map_err(err)?;
        // Line by line, so lists and code keep their shape.
        Ok(result.Lines().map_err(err)?.into_iter().filter_map(|l| l.Text().ok()).map(|t| t.to_string()).collect::<Vec<_>>().join("\n"))
    }
}

#[cfg(not(windows))]
mod os {
    use image::{codecs::png::PngEncoder, ExtendedColorType, ImageEncoder, RgbImage};
    use std::io::Write;
    use std::process::{Command, Stdio};

    /// OCR-1.5: the image comes in on standard input and the text goes out on standard output.
    pub const ARGS: [&str; 2] = ["stdin", "stdout"];

    pub fn available() -> bool {
        Command::new("tesseract").arg("--version").stdout(Stdio::null()).stderr(Stdio::null()).status().is_ok_and(|s| s.success())
    }

    pub fn read(img: &RgbImage) -> Result<String, String> {
        let mut png = Vec::new();
        PngEncoder::new(&mut png).write_image(img.as_raw(), img.width(), img.height(), ExtendedColorType::Rgb8).map_err(|e| e.to_string())?;
        let failed = |e: std::io::Error| format!("Couldn't read the text in the snip: {e}");
        let mut child = Command::new("tesseract").args(ARGS).stdin(Stdio::piped()).stdout(Stdio::piped()).stderr(Stdio::null()).spawn().map_err(failed)?;
        // Tesseract reads all of its input before it writes, so writing first can't deadlock.
        child.stdin.take().expect("piped").write_all(&png).map_err(failed)?;
        let out = child.wait_with_output().map_err(failed)?;
        if !out.status.success() {
            return Err("Couldn't read the text in the snip: Tesseract failed.".into());
        }
        Ok(String::from_utf8_lossy(&out.stdout).into_owned())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ocr_1_2_the_text_keeps_its_lines_and_one_gap_between_paragraphs() {
        assert_eq!(tidy("  Error: file not found  \r\n\n\n\n  at main.rs:12 \n"), "Error: file not found\n\nat main.rs:12");
        assert_eq!(tidy("\n\n  \n"), "");
        assert_eq!(tidy("a\nb"), "a\nb");
    }

    #[cfg(not(windows))]
    #[test]
    fn ocr_1_5_tesseract_gets_the_image_on_standard_input() {
        assert_eq!(os::ARGS, ["stdin", "stdout"]);
    }

    /// A picture of a line of text is read back, by Windows' own recognition or by Tesseract. A computer without an
    /// OCR language or without Tesseract can't run it, and says so.
    #[test]
    fn ocr_1_2_the_text_in_a_picture_is_read_on_this_computer() {
        if !available() {
            return eprintln!("no text recognition on this computer: skipped");
        }
        let img = image::load_from_memory(include_bytes!("fixtures/ocr.png")).unwrap().to_rgb8();
        assert_eq!(read(&img).unwrap(), "Cursora reads snips");
    }
}
