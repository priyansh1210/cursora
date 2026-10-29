//! Voice input (spec 19): the microphone through cpal, and whisper.cpp on this computer. The recording stays in memory
//! (VIN-1.3): a thread owns the input stream, and its samples are handed over once to be turned into text, or dropped.
use crate::AppState;
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tauri::{AppHandle, Manager};
use whisper_rs::{FullParams, SamplingStrategy, WhisperContext, WhisperContextParameters};

/// VIN-1.1: recording stops by itself after this; the samples past it are dropped even if Stop never comes.
const MAX_SECS: usize = 60;
/// The microphone closes by itself this long after recording starts, if nothing stopped it: a window closed while
/// recording, for one. The page stops at 60 s, so this leaves it time.
const LIMIT: Duration = Duration::from_secs(MAX_SECS as u64 + 5);
const RATE: usize = 16_000; // what whisper expects
const MISSING: &str = "The speech model is missing. Download it again in Settings → General → Voice input.";
const NOTHING: &str = "Didn't catch that. Try again, a little closer to the microphone.";

enum End {
    Stop,
    Cancel,
}

/// A running recording: tell its thread how it ends, then (on Stop) receive the samples with their rate and channels.
struct Recording {
    end: Sender<End>,
    done: Receiver<Option<(Vec<f32>, u32, u16)>>,
}

/// The recording in progress, if any.
#[derive(Default)]
pub struct Voice(Mutex<Option<Recording>>);

impl Voice {
    /// VIN-1.5: ends the recording, if any, and drops what was recorded.
    fn cancel(&self) {
        if let Some(r) = self.0.lock().unwrap().take() {
            let _ = r.end.send(End::Cancel);
        }
    }
}

/// The recording thread's last step: the samples on Stop; nothing on Cancel, when the sender is gone, or when nothing came
/// within `limit`.
fn wait(end: Receiver<End>, samples: Arc<Mutex<Vec<f32>>>, limit: Duration) -> Option<Vec<f32>> {
    matches!(end.recv_timeout(limit), Ok(End::Stop)).then(|| std::mem::take(&mut *samples.lock().unwrap()))
}

/// VIN-1.4: plain words for the bubble. The system's own error (such as "OS Error -2004287487") goes to the log only.
fn mic(e: impl std::fmt::Display) -> String {
    log::warn!("microphone: {e}");
    let check = if cfg!(windows) { "that Settings → Privacy & security → Microphone → Let desktop apps access your microphone is on" } else { "that no other app is using it" };
    format!("Cursora couldn't use the microphone. Check that one is connected, and {check}.")
}

/// An input stream that appends samples as f32, up to `cap` of them.
fn stream<T: cpal::SizedSample>(device: &cpal::Device, config: &cpal::StreamConfig, cap: usize, sink: Arc<Mutex<Vec<f32>>>) -> Result<cpal::Stream, String>
where
    f32: cpal::FromSample<T>,
{
    let data = move |d: &[T], _: &cpal::InputCallbackInfo| {
        let mut s = sink.lock().unwrap();
        let room = cap.saturating_sub(s.len());
        s.extend(d.iter().take(room).map(|&x| <f32 as cpal::Sample>::from_sample(x)));
    };
    device.build_input_stream(*config, data, |e| log::warn!("microphone: {e}"), None).map_err(mic)
}

/// Opens the default microphone in its own default format, which every device supports, on a thread that owns the stream
/// (a cpal stream isn't `Send` everywhere). Errors opening it come back before recording starts (VIN-1.4).
fn record() -> Result<Recording, String> {
    let (end_tx, end_rx) = mpsc::channel();
    let (done_tx, done_rx) = mpsc::channel();
    let (ready_tx, ready_rx) = mpsc::channel::<Result<(), String>>();
    std::thread::spawn(move || {
        let started = (|| {
            let device = cpal::default_host().default_input_device().ok_or("No microphone was found.")?;
            let supported = device.default_input_config().map_err(mic)?;
            let config = supported.config();
            let cap = MAX_SECS * config.sample_rate as usize * config.channels as usize;
            let samples = Arc::new(Mutex::new(Vec::new()));
            let s = samples.clone();
            let stream = match supported.sample_format() {
                cpal::SampleFormat::F32 => stream::<f32>(&device, &config, cap, s),
                cpal::SampleFormat::I16 => stream::<i16>(&device, &config, cap, s),
                cpal::SampleFormat::U16 => stream::<u16>(&device, &config, cap, s),
                cpal::SampleFormat::I32 => stream::<i32>(&device, &config, cap, s),
                f => Err(mic(format!("its sample format {f} isn't supported"))),
            }?;
            stream.play().map_err(mic)?;
            Ok::<_, String>((stream, samples, config.sample_rate, config.channels))
        })();
        match started {
            Err(e) => drop(ready_tx.send(Err(e))),
            Ok((stream, samples, rate, channels)) => {
                let _ = ready_tx.send(Ok(()));
                let got = wait(end_rx, samples, LIMIT);
                drop(stream);
                let _ = done_tx.send(got.map(|s| (s, rate, channels)));
            }
        }
    });
    ready_rx.recv().map_err(|_| mic("it stopped"))??;
    Ok(Recording { end: end_tx, done: done_rx })
}

/// Averages the channels, then resamples to 16 kHz by averaging each output sample's span of input samples, which also
/// keeps higher sounds from folding into the speech range. Speech needs nothing better.
pub fn to_16k_mono(samples: &[f32], rate: u32, channels: u16) -> Vec<f32> {
    let mono: Vec<f32> = samples.chunks(channels.max(1) as usize).map(|f| f.iter().sum::<f32>() / f.len() as f32).collect();
    if rate as usize == RATE {
        return mono;
    }
    let step = f64::from(rate) / RATE as f64;
    let n = (mono.len() as f64 / step) as usize;
    (0..n)
        .map(|i| {
            let a = (i as f64 * step) as usize;
            let b = (((i + 1) as f64 * step) as usize).clamp(a + 1, mono.len());
            mono[a..b].iter().sum::<f32>() / (b - a) as f32
        })
        .collect()
}

/// Whisper reads a 30-second window (1500) unless told less; this fits it to the recording, with some room.
pub fn audio_ctx(samples_16k: usize) -> i32 {
    (samples_16k / (RATE / 50) + 128).min(1500) as i32
}

/// VIN-1.2: the text without whisper's marks for sounds that aren't speech, such as `[BLANK_AUDIO]`, `(music)` or `♪`.
pub fn clean(text: &str) -> String {
    static MARKS: std::sync::LazyLock<regex::Regex> = std::sync::LazyLock::new(|| regex::Regex::new(r"\[[^\]]*\]|\([^)]*\)|♪").unwrap());
    MARKS.replace_all(text, " ").split_whitespace().collect::<Vec<_>>().join(" ")
}

/// VIN-2.4: never guessed. Urdu and Persian share much of spoken Hindi, so a Hindi sentence with an Urdu word came out in
/// Urdu script; the other languages of India are left out at the user's word (2026-10-29).
const NOT_GUESSED: [&str; 15] = ["ur", "fa", "bn", "ta", "te", "mr", "gu", "kn", "ml", "pa", "ne", "si", "sa", "as", "sd"];

/// VIN-2.4: the most likely language in whisper's probabilities, leaving out `NOT_GUESSED`.
fn language(probs: &[f32]) -> &'static str {
    let allowed = |&(i, _): &(usize, &f32)| whisper_rs::get_lang_str(i as i32).is_some_and(|code| !NOT_GUESSED.contains(&code));
    let best = probs.iter().enumerate().filter(allowed).max_by(|a, b| a.1.total_cmp(b.1)).map(|(i, _)| i as i32);
    best.and_then(whisper_rs::get_lang_str).unwrap_or("en")
}

/// Loads the model for this recording only, so its memory is held only while it works.
fn transcribe(model: &str, mut pcm: Vec<f32>) -> Result<String, String> {
    if !std::path::Path::new(model).is_file() {
        return Err(MISSING.into());
    }
    let ctx = WhisperContext::new_with_params(model, WhisperContextParameters::default()).map_err(|_| MISSING.to_string())?;
    let mut state = ctx.create_state().map_err(|e| e.to_string())?;
    let ac = audio_ctx(pcm.len());
    if pcm.len() < RATE {
        pcm.resize(RATE + RATE / 10, 0.0); // whisper wants at least a second
    }
    let threads = std::thread::available_parallelism().map_or(4, |n| n.get());
    // VIN-2.4: whisper's own guess ranges over all its languages and wrote Hindi with an Urdu word in Urdu script.
    state.pcm_to_mel(&pcm, threads).map_err(|e| e.to_string())?;
    let (_, probs) = state.lang_detect(0, threads).map_err(|e| e.to_string())?;
    let mut p = FullParams::new(SamplingStrategy::Greedy { best_of: 1 });
    p.set_language(Some(language(&probs)));
    p.set_n_threads(threads as i32);
    p.set_audio_ctx(ac);
    p.set_suppress_nst(true);
    p.set_no_timestamps(true);
    p.set_print_special(false);
    p.set_print_progress(false);
    p.set_print_realtime(false);
    p.set_print_timestamps(false);
    state.full(p, &pcm).map_err(|e| format!("Couldn't turn the speech into text: {e}"))?;
    Ok(clean(&state.as_iter().map(|s| s.to_string()).collect::<String>()))
}

/// VIN-1.1: starts recording from the default microphone, dropping any recording left over (one that ran out of time, or
/// from a window that closed).
#[tauri::command]
pub fn voice_start(app: AppHandle) -> Result<(), String> {
    let voice = app.state::<Voice>();
    voice.cancel();
    let recording = record()?;
    *voice.0.lock().unwrap() = Some(recording);
    Ok(())
}

/// VIN-1.2: stops recording and returns what was said.
#[tauri::command]
pub async fn voice_stop(app: AppHandle) -> Result<String, String> {
    let rec = app.state::<Voice>().0.lock().unwrap().take().ok_or("Nothing is being recorded.")?;
    let model = app.state::<AppState>().settings.lock().unwrap().voice_model.clone().ok_or(MISSING)?;
    let _ = rec.end.send(End::Stop);
    tokio::task::spawn_blocking(move || {
        let (samples, rate, channels) = rec.done.recv().ok().flatten().ok_or_else(|| mic("it stopped"))?;
        let text = transcribe(&model, to_16k_mono(&samples, rate, channels))?;
        if text.is_empty() { Err(NOTHING.to_string()) } else { Ok(text) }
    })
    .await
    .map_err(|e| e.to_string())?
}

/// VIN-1.5: the bubble drops the recording when it hides.
pub fn cancel(app: &AppHandle) {
    app.state::<Voice>().cancel();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn vin_1_4_a_microphone_error_is_said_in_plain_words() {
        let said = mic("Failed to initialize audio client: OS Error -2004287487 (FormatMessageW() returned error 317)");
        assert!(said.starts_with("Cursora couldn't use the microphone. Check that one is connected"), "{said}");
        assert!(!said.contains("2004287487") && !said.contains("FormatMessageW"), "the system's error stays in the log: {said}");
    }

    #[test]
    fn vin_2_4_urdu_persian_and_other_indian_languages_are_never_guessed() {
        let id = |code| whisper_rs::get_lang_id(code).unwrap() as usize;
        let mut probs = vec![0.0; whisper_rs::get_lang_max_id() as usize + 1];
        (probs[id("ur")], probs[id("fa")], probs[id("hi")], probs[id("en")]) = (0.5, 0.2, 0.2, 0.1);
        assert_eq!(language(&probs), "hi", "Hindi with Urdu and Persian words stays Hindi");
        (probs[id("ta")], probs[id("bn")]) = (0.9, 0.8);
        assert_eq!(language(&probs), "hi", "nor another language of India");
        probs[id("fr")] = 0.3;
        assert_eq!(language(&probs), "fr", "foreign languages still are");
        probs[id("en")] = 0.4;
        assert_eq!(language(&probs), "en");
        assert_eq!(language(&[]), "en", "nothing detected");
    }

    #[test]
    fn vin_1_2_audio_becomes_16khz_mono() {
        // 48 kHz stereo: left 0.2, right 0.6 → mono 0.4, a third as many samples.
        let stereo: Vec<f32> = (0..4800).flat_map(|_| [0.2, 0.6]).collect();
        let out = to_16k_mono(&stereo, 48_000, 2);
        assert_eq!(out.len(), 1600);
        assert!(out.iter().all(|s| (s - 0.4).abs() < 1e-6));
        // A ramp keeps its shape: each output is the average of its three inputs.
        let ramp: Vec<f32> = (0..30).map(|i| i as f32).collect();
        assert_eq!(to_16k_mono(&ramp, 48_000, 1), vec![1.0, 4.0, 7.0, 10.0, 13.0, 16.0, 19.0, 22.0, 25.0, 28.0]);
        // 16 kHz mono passes through; 8 kHz is stretched to twice the length.
        assert_eq!(to_16k_mono(&[0.1, 0.2], 16_000, 1), vec![0.1, 0.2]);
        assert_eq!(to_16k_mono(&[0.1, 0.2], 8_000, 1).len(), 4);
        assert!(to_16k_mono(&[], 44_100, 2).is_empty());
    }

    #[test]
    fn vin_1_2_whisper_reads_only_as_much_as_was_recorded() {
        assert_eq!(audio_ctx(81_600), 383); // 5.1 s: 255 frames and room
        assert_eq!(audio_ctx(0), 128);
        assert_eq!(audio_ctx(60 * RATE), 1500); // a full minute is whisper's whole window
    }

    #[test]
    fn vin_1_2_marks_for_sounds_that_are_not_speech_are_left_out() {
        assert_eq!(clean(" [BLANK_AUDIO]"), "");
        assert_eq!(clean(" (music) Explain this  paragraph [Music] ♪ please."), "Explain this paragraph please.");
        assert_eq!(clean(" इस पैराग्राफ को समझाओ"), "इस पैराग्राफ को समझाओ");
        assert_eq!(clean("Hello\n world"), "Hello world");
    }

    #[test]
    fn vin_1_3_the_recording_is_never_written_or_sent() {
        let code = include_str!("voice.rs").split("#[cfg(test)]").next().unwrap();
        for banned in ["std::fs", "File::", "reqwest", "TcpStream", "UdpSocket", "emit("] {
            assert!(!code.contains(banned), "voice.rs uses {banned}");
        }
    }

    #[test]
    fn vin_1_5_cancel_drops_the_recording() {
        let voice = Voice::default();
        let samples = Arc::new(Mutex::new(vec![0.5f32; 1000]));
        let gone = Arc::downgrade(&samples);
        let (end_tx, end_rx) = mpsc::channel();
        let (done_tx, done_rx) = mpsc::channel();
        let thread = std::thread::spawn(move || drop(done_tx.send(wait(end_rx, samples, LIMIT).map(|s| (s, 16_000, 1)))));
        *voice.0.lock().unwrap() = Some(Recording { end: end_tx, done: done_rx });
        voice.cancel();
        thread.join().unwrap();
        assert!(voice.0.lock().unwrap().is_none());
        assert!(gone.upgrade().is_none(), "the samples were freed");

        // Stop hands them over instead.
        let (end_tx, end_rx) = mpsc::channel();
        let samples = Arc::new(Mutex::new(vec![0.5f32; 3]));
        end_tx.send(End::Stop).unwrap();
        assert_eq!(wait(end_rx, samples, LIMIT), Some(vec![0.5; 3]));
    }

    #[test]
    fn vin_1_1_the_microphone_closes_by_itself_when_nobody_stops_it() {
        let samples = Arc::new(Mutex::new(vec![0.5f32; 1000]));
        let gone = Arc::downgrade(&samples);
        let (_end_tx, end_rx) = mpsc::channel::<End>(); // kept: a window that closed never sends Stop or Cancel
        assert_eq!(wait(end_rx, samples, Duration::from_millis(50)), None);
        assert!(gone.upgrade().is_none(), "the samples were freed");
        assert_eq!(LIMIT, Duration::from_secs(65));
    }
}
