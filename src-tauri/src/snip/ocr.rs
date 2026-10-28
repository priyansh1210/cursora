//! Text recognition for snips (spec 20): the system's own, so a model that can't see still gets a snip's text. The image
//! stays in memory: Windows reads it from a buffer, Tesseract from its standard input (OCR-1.5).
use crate::bubble::Rect;
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

/// A recognized word and its box, in the image's pixels (HOV-1.2).
#[derive(Debug, PartialEq)]
pub struct Word {
    pub text: String,
    pub rect: Rect,
}

/// Each word in the image, with its box (HOV-1.2).
pub fn words(img: &RgbImage) -> Result<Vec<Word>, String> {
    os::words(img)
}

/// The word whose box, with a few pixels of slack, holds the point; without the punctuation around it (HOV-1.2).
pub fn word_at(words: &[Word], x: i32, y: i32) -> Option<String> {
    const SLACK: i32 = 3;
    let r = &words.iter().find(|w| (w.rect.x - SLACK..=w.rect.x + w.rect.w + SLACK).contains(&x) && (w.rect.y - SLACK..=w.rect.y + w.rect.h + SLACK).contains(&y))?.text;
    Some(r.trim_matches(|c: char| !c.is_alphanumeric()).to_string()).filter(|t| !t.is_empty())
}

/// Tesseract's `tsv` table: a header, then level, page, block, paragraph, line, word, left, top, width, height,
/// confidence and text. Level 5 rows are words.
#[cfg(any(not(windows), test))]
fn tsv(out: &str) -> Vec<Word> {
    out.lines()
        .filter_map(|line| {
            let f: Vec<&str> = line.split('\t').collect();
            let n = |i: usize| f.get(i)?.parse::<i32>().ok();
            let text = f.get(11)?.trim();
            if f[0] != "5" || text.is_empty() {
                return None;
            }
            Some(Word { text: text.into(), rect: Rect { x: n(6)?, y: n(7)?, w: n(8)?, h: n(9)? } })
        })
        .collect()
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
    use super::Word;
    use crate::bubble::Rect;
    use image::RgbImage;
    use windows::Graphics::Imaging::{BitmapPixelFormat, SoftwareBitmap};
    use windows::Media::Ocr::{OcrEngine, OcrResult};
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

    fn err(e: windows::core::Error) -> String {
        format!("Couldn't read the text on the screen: {}", e.message())
    }

    /// The engine's result, and how many of the image's pixels make one of the result's: the engine has a size limit.
    fn recognize(img: &RgbImage) -> Result<(OcrResult, f32), String> {
        let engine = engine().map_err(err)?;
        let fitted = crate::snip::fit(img.clone(), OcrEngine::MaxImageDimension().map_err(err)?);
        let bgra: Vec<u8> = fitted.pixels().flat_map(|p| [p[2], p[1], p[0], 255]).collect();
        let writer = DataWriter::new().map_err(err)?;
        writer.WriteBytes(&bgra).map_err(err)?;
        let bitmap = SoftwareBitmap::CreateCopyFromBuffer(&writer.DetachBuffer().map_err(err)?, BitmapPixelFormat::Bgra8, fitted.width() as i32, fitted.height() as i32).map_err(err)?;
        Ok((engine.RecognizeAsync(&bitmap).map_err(err)?.join().map_err(err)?, img.width() as f32 / fitted.width() as f32))
    }

    pub fn read(img: &RgbImage) -> Result<String, String> {
        let (result, _) = recognize(img)?;
        // Line by line, so lists and code keep their shape.
        Ok(result.Lines().map_err(err)?.into_iter().filter_map(|l| l.Text().ok()).map(|t| t.to_string()).collect::<Vec<_>>().join("\n"))
    }

    pub fn words(img: &RgbImage) -> Result<Vec<Word>, String> {
        let (result, k) = recognize(img)?;
        let px = |v: f32| (v * k).round() as i32;
        let mut out = vec![];
        for line in result.Lines().map_err(err)? {
            for w in line.Words().map_err(err)? {
                let r = w.BoundingRect().map_err(err)?;
                out.push(Word { text: w.Text().map_err(err)?.to_string(), rect: Rect { x: px(r.X), y: px(r.Y), w: px(r.Width), h: px(r.Height) } });
            }
        }
        Ok(out)
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
        run(img, &[])
    }

    pub fn words(img: &RgbImage) -> Result<Vec<super::Word>, String> {
        run(img, &["tsv"]).map(|t| super::tsv(&t))
    }

    /// Tesseract on the image, with `extra` after the input and output: a config such as `tsv`.
    fn run(img: &RgbImage, extra: &[&str]) -> Result<String, String> {
        let mut png = Vec::new();
        PngEncoder::new(&mut png).write_image(img.as_raw(), img.width(), img.height(), ExtendedColorType::Rgb8).map_err(|e| e.to_string())?;
        let failed = |e: std::io::Error| format!("Couldn't read the text on the screen: {e}");
        let mut child = Command::new("tesseract").args(ARGS).args(extra).stdin(Stdio::piped()).stdout(Stdio::piped()).stderr(Stdio::null()).spawn().map_err(failed)?;
        // Tesseract reads all of its input before it writes, so writing first can't deadlock.
        child.stdin.take().expect("piped").write_all(&png).map_err(failed)?;
        let out = child.wait_with_output().map_err(failed)?;
        if !out.status.success() {
            return Err("Couldn't read the text on the screen: Tesseract failed.".into());
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

    fn word(text: &str, x: i32, y: i32, w: i32, h: i32) -> Word {
        Word { text: text.into(), rect: Rect { x, y, w, h } }
    }

    #[test]
    fn hov_1_2_the_word_under_the_point_without_its_punctuation() {
        let line = [word("Hello,", 10, 10, 50, 16), word("(world)!", 66, 10, 60, 16), word("\u{2014}", 130, 10, 10, 16), word("next", 10, 40, 40, 16)];
        assert_eq!(word_at(&line, 30, 18).as_deref(), Some("Hello"));
        assert_eq!(word_at(&line, 125, 9).as_deref(), Some("world"), "a few pixels of slack");
        assert_eq!(word_at(&line, 135, 18), None, "a dash alone");
        assert_eq!(word_at(&line, 30, 32), None, "between the lines");
        assert_eq!(word_at(&[word("well-known", 0, 0, 80, 16), word("don't", 90, 0, 40, 16)], 100, 8).as_deref(), Some("don't"), "inner marks stay");
    }

    #[test]
    fn hov_1_2_tesseracts_table_gives_each_word_and_its_box() {
        let out = "level\tpage_num\tblock_num\tpar_num\tline_num\tword_num\tleft\ttop\twidth\theight\tconf\ttext\n\
                   1\t1\t0\t0\t0\t0\t0\t0\t400\t60\t-1\t\n\
                   5\t1\t1\t1\t1\t1\t12\t20\t96\t18\t96.1\tCursora\n\
                   5\t1\t1\t1\t1\t2\t118\t20\t60\t18\t95.0\treads\n\
                   5\t1\t1\t1\t1\t3\t190\t20\t10\t18\t20.0\t \n";
        assert_eq!(tsv(out), [word("Cursora", 12, 20, 96, 18), word("reads", 118, 20, 60, 18)]);
    }

    /// Word by word, where this computer can read text: the fixture's middle word is found under a point inside it.
    #[test]
    fn hov_1_2_the_words_in_a_picture_are_read_with_their_boxes() {
        if !available() {
            return eprintln!("no text recognition on this computer: skipped");
        }
        let img = image::load_from_memory(include_bytes!("fixtures/ocr.png")).unwrap().to_rgb8();
        let found = words(&img).unwrap();
        assert_eq!(found.iter().map(|w| w.text.as_str()).collect::<Vec<_>>(), ["Cursora", "reads", "snips"]);
        let reads = &found[1].rect;
        assert_eq!(word_at(&found, reads.x + reads.w / 2, reads.y + reads.h / 2).as_deref(), Some("reads"));
    }
}
