//! Microphone capture on Linux, through ALSA by way of `cpal`. PipeWire and
//! PulseAudio both expose ALSA devices, so this works on current desktops
//! without talking to either directly.
//!
//! The sound card delivers its own rate and channel count (commonly 48 kHz
//! stereo or 44.1 kHz mono); `Resampler` turns that into the 16 kHz mono clip
//! the speech engines expect.

use super::*;
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use std::sync::mpsc;
use std::time::{Duration, Instant};

pub struct Recorder {
    capture: Arc<Capture>,
    /// Dropping this wakes the capture thread, which then closes the stream.
    stop: Option<mpsc::Sender<()>>,
    thread: Option<std::thread::JoinHandle<()>>,
    started: Instant,
}

impl Recorder {
    /// Start recording. `preferred` names an input device; anything that does
    /// not match (including "System default") uses the system's default input.
    pub fn start(preferred: Option<&str>) -> Result<Recorder, String> {
        let capture = Arc::new(Capture {
            samples: Mutex::new(Vec::with_capacity(TARGET_RATE as usize * 30)),
            level: AtomicU32::new(0),
            bands: Arc::new(Mutex::new([0f32; 7])),
        });
        let (ready_tx, ready_rx) = mpsc::channel::<Result<(), String>>();
        let (stop_tx, stop_rx) = mpsc::channel::<()>();
        let cap = capture.clone();
        let wanted = preferred.map(str::to_string);
        // The stream is not Send, so one thread owns it from open to close.
        let thread = std::thread::Builder::new()
            .name("flowline-mic".into())
            .spawn(move || match open_stream(wanted.as_deref(), cap) {
                Ok(stream) => {
                    let _ = ready_tx.send(Ok(()));
                    let _ = stop_rx.recv();
                    drop(stream);
                }
                Err(e) => {
                    let _ = ready_tx.send(Err(e));
                }
            })
            .map_err(|e| format!("could not start the microphone thread: {e}"))?;
        match ready_rx.recv_timeout(Duration::from_secs(5)) {
            Ok(Ok(())) => Ok(Recorder {
                capture,
                stop: Some(stop_tx),
                thread: Some(thread),
                started: Instant::now(),
            }),
            Ok(Err(e)) => Err(e),
            Err(_) => Err("the microphone did not respond".to_string()),
        }
    }

    /// Peak input level, 0..1, for the listening meter.
    pub fn level(&self) -> f32 {
        self.capture.level.load(Ordering::Relaxed) as f32 / 32767.0
    }

    /// Seven smoothed voice bands, 0..1.
    pub fn bands(&self) -> [f32; 7] {
        self.capture.bands.lock().map(|b| *b).unwrap_or([0f32; 7])
    }

    /// Share the live bands with the floating overlay.
    pub fn meter(&self) -> Arc<Mutex<[f32; 7]>> {
        self.capture.bands.clone()
    }

    pub fn seconds(&self) -> f32 {
        self.started.elapsed().as_secs_f32()
    }

    /// Stop capturing and encode the clip as a 16 kHz mono 16-bit WAV.
    pub fn finish(self) -> Option<Vec<u8>> {
        let samples = {
            let mut buf = self.capture.samples.lock().ok()?;
            std::mem::take(&mut *buf)
        };
        if samples.len() < (TARGET_RATE as usize / 20) {
            return None;
        }
        Some(wav_from_pcm16(&samples))
    }
}

impl Drop for Recorder {
    fn drop(&mut self) {
        self.stop.take(); // closes the channel, so the thread's recv returns
        if let Some(t) = self.thread.take() {
            let _ = t.join();
        }
    }
}

fn pick_device(host: &cpal::Host, preferred: Option<&str>) -> Option<cpal::Device> {
    if let Some(name) = preferred.filter(|n| *n != "System default") {
        if let Ok(mut devices) = host.input_devices() {
            if let Some(d) = devices.find(|d| d.name().map_or(false, |n| n == name)) {
                return Some(d);
            }
        }
    }
    host.default_input_device()
}

fn open_stream(preferred: Option<&str>, capture: Arc<Capture>) -> Result<cpal::Stream, String> {
    let host = cpal::default_host();
    let device = pick_device(&host, preferred).ok_or_else(|| {
        "no microphone found. Plug one in and check it in your sound settings.".to_string()
    })?;
    let config = device
        .default_input_config()
        .map_err(|e| format!("the microphone could not be opened ({e})"))?;
    let format = config.sample_format();
    let channels = config.channels();
    let rate = config.sample_rate().0;
    let stream_config: cpal::StreamConfig = config.into();

    let mut resampler = Resampler::new(rate, channels);
    let mut recent: Vec<i16> = Vec::new();
    let mut block: Vec<i16> = Vec::new();
    let mut on_floats = move |data: &[f32]| {
        block.clear();
        resampler.process(data, &mut block);
        ingest(&capture, &block, &mut recent);
    };
    let on_error = |e: cpal::StreamError| eprintln!("microphone stream error: {e}");

    let stream = match format {
        cpal::SampleFormat::F32 => device.build_input_stream(
            &stream_config,
            move |data: &[f32], _: &cpal::InputCallbackInfo| on_floats(data),
            on_error,
            None,
        ),
        cpal::SampleFormat::I16 => device.build_input_stream(
            &stream_config,
            move |data: &[i16], _: &cpal::InputCallbackInfo| {
                let floats: Vec<f32> = data.iter().map(|s| *s as f32 / 32768.0).collect();
                on_floats(&floats)
            },
            on_error,
            None,
        ),
        cpal::SampleFormat::U16 => device.build_input_stream(
            &stream_config,
            move |data: &[u16], _: &cpal::InputCallbackInfo| {
                let floats: Vec<f32> =
                    data.iter().map(|s| (*s as f32 - 32768.0) / 32768.0).collect();
                on_floats(&floats)
            },
            on_error,
            None,
        ),
        other => return Err(format!("the microphone uses an unsupported format ({other:?})")),
    }
    .map_err(|e| format!("the microphone could not start ({e})"))?;
    stream
        .play()
        .map_err(|e| format!("the microphone could not start ({e})"))?;
    Ok(stream)
}

/// Microphones that can record, by name.
pub fn input_names() -> Vec<String> {
    let host = cpal::default_host();
    let mut out: Vec<String> = Vec::new();
    if let Ok(devices) = host.input_devices() {
        for d in devices {
            if d.default_input_config().is_err() {
                continue;
            }
            if let Ok(name) = d.name() {
                if !out.contains(&name) {
                    out.push(name);
                }
            }
        }
    }
    out
}
