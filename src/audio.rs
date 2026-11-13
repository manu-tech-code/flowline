//! Microphone capture for dictation.
//!
//! The original records at 16 kHz mono and streams the clip to its cloud. Here
//! the same 16 kHz mono WAV goes to the selected local model instead; nothing ever
//! leaves the Mac.
//!
//! This uses Apple's AudioQueue C API directly (AudioToolbox) so the app keeps
//! its tiny dependency list and still records without an external binary. The
//! system resamples the microphone to 16 kHz mono 16-bit PCM for us.

#![allow(non_snake_case)]

use serde_json::Value;
use std::os::raw::c_void;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{Arc, Mutex};

pub const TARGET_RATE: u32 = 16_000;
/// 20 minutes at 16 kHz: the original's maximum dictation length.
const MAX_SAMPLES: usize = TARGET_RATE as usize * 1200;

type OSStatus = i32;
type AudioQueueRef = *mut c_void;
type AudioQueueBufferRef = *mut AudioQueueBuffer;

#[repr(C)]
struct AudioStreamBasicDescription {
    mSampleRate: f64,
    mFormatID: u32,
    mFormatFlags: u32,
    mBytesPerPacket: u32,
    mFramesPerPacket: u32,
    mBytesPerFrame: u32,
    mChannelsPerFrame: u32,
    mBitsPerChannel: u32,
    mReserved: u32,
}

#[repr(C)]
struct AudioQueueBuffer {
    mAudioDataBytesCapacity: u32,
    mAudioData: *mut c_void,
    mAudioDataByteSize: u32,
    mUserData: *mut c_void,
    mPacketDescriptionCapacity: u32,
    mPacketDescriptions: *mut c_void,
    mPacketDescriptionCount: u32,
}

const K_AUDIO_FORMAT_LINEAR_PCM: u32 = 0x6C70_636D; // 'lpcm'
const K_LINEAR_PCM_FLAG_SIGNED_INTEGER: u32 = 4;
const K_LINEAR_PCM_FLAG_PACKED: u32 = 8;

type InputCallback = extern "C" fn(
    *mut c_void,
    AudioQueueRef,
    AudioQueueBufferRef,
    *const c_void,
    u32,
    *const c_void,
);

#[link(name = "AudioToolbox", kind = "framework")]
extern "C" {
    fn AudioQueueNewInput(
        format: *const AudioStreamBasicDescription,
        callback: InputCallback,
        user_data: *mut c_void,
        run_loop: *const c_void,
        run_loop_mode: *const c_void,
        flags: u32,
        out_queue: *mut AudioQueueRef,
    ) -> OSStatus;
    fn AudioQueueAllocateBuffer(
        queue: AudioQueueRef,
        byte_size: u32,
        out_buffer: *mut AudioQueueBufferRef,
    ) -> OSStatus;
    fn AudioQueueEnqueueBuffer(
        queue: AudioQueueRef,
        buffer: AudioQueueBufferRef,
        num_packet_descriptions: u32,
        packet_descriptions: *const c_void,
    ) -> OSStatus;
    fn AudioQueueStart(queue: AudioQueueRef, start_time: *const c_void) -> OSStatus;
    fn AudioQueueStop(queue: AudioQueueRef, immediate: u8) -> OSStatus;
    fn AudioQueueDispose(queue: AudioQueueRef, immediate: u8) -> OSStatus;
}

struct Capture {
    samples: Mutex<Vec<i16>>,
    /// Peak since the last read, 0..32767.
    level: AtomicU32,
    /// Smoothed energy in seven voice bands, 0..1, shared with the UI meters.
    bands: Arc<Mutex<[f32; 7]>>,
}

/// Seven log-spaced bands from ~80 Hz to 8 kHz. The first two carry most of a
/// voice's pitch and the upper ones the consonants; the meters are driven from
/// these rather than from a decorative sine.
const BAND_EDGES: [f32; 8] = [80.0, 160.0, 320.0, 640.0, 1280.0, 2560.0, 5120.0, 8000.0];

fn fft(re: &mut [f32], im: &mut [f32]) {
    let n = re.len();
    let mut j = 0usize;
    for i in 1..n {
        let mut bit = n >> 1;
        while j & bit != 0 {
            j ^= bit;
            bit >>= 1;
        }
        j ^= bit;
        if i < j {
            re.swap(i, j);
            im.swap(i, j);
        }
    }
    let mut len = 2usize;
    while len <= n {
        let ang = -2.0 * std::f32::consts::PI / len as f32;
        let (wr, wi) = (ang.cos(), ang.sin());
        let mut i = 0usize;
        while i < n {
            let (mut cr, mut ci) = (1.0f32, 0.0f32);
            for k in 0..len / 2 {
                let u = i + k;
                let v = u + len / 2;
                let tr = re[v] * cr - im[v] * ci;
                let ti = re[v] * ci + im[v] * cr;
                re[v] = re[u] - tr;
                im[v] = im[u] - ti;
                re[u] += tr;
                im[u] += ti;
                let ncr = cr * wr - ci * wi;
                ci = cr * wi + ci * wr;
                cr = ncr;
            }
            i += len;
        }
        len <<= 1;
    }
}

/// Energy per band for one 100 ms microphone buffer.
fn band_energy(samples: &[i16]) -> [f32; 7] {
    const N: usize = 512;
    let mut out = [0f32; 7];
    if samples.len() < N {
        return out;
    }
    let mut re = [0f32; N];
    let mut im = [0f32; N];
    for i in 0..N {
        let w = 0.5 - 0.5 * (2.0 * std::f32::consts::PI * i as f32 / (N - 1) as f32).cos();
        re[i] = samples[i] as f32 / 32768.0 * w;
    }
    fft(&mut re, &mut im);
    for k in 1..N / 2 {
        let f = k as f32 * TARGET_RATE as f32 / N as f32;
        let mag2 = re[k] * re[k] + im[k] * im[k];
        for b in 0..7 {
            if f >= BAND_EDGES[b] && f < BAND_EDGES[b + 1] {
                out[b] += mag2;
                break;
            }
        }
    }
    // Convert each band to dBFS-ish and map -60 dB…0 dB onto 0…1. The
    // coherent gain of the Hann window is N/4.
    let norm = N as f32 * 0.25;
    for v in &mut out {
        let rms = v.sqrt() / norm;
        let db = 20.0 * rms.max(1e-7).log10();
        *v = ((db + 60.0) / 60.0).clamp(0.0, 1.0);
    }
    out
}

extern "C" fn on_input(
    user_data: *mut c_void,
    queue: AudioQueueRef,
    buffer: AudioQueueBufferRef,
    _start_time: *const c_void,
    _packets: u32,
    _descriptions: *const c_void,
) {
    if user_data.is_null() || buffer.is_null() || queue.is_null() {
        return;
    }
    let capture = unsafe { &*(user_data as *const Capture) };
    unsafe {
        let b = &*buffer;
        if !b.mAudioData.is_null() && b.mAudioDataByteSize >= 2 {
            let count = (b.mAudioDataByteSize as usize) / 2;
            let samples = std::slice::from_raw_parts(b.mAudioData as *const i16, count);
            let mut peak = 0i32;
            if let Ok(mut buf) = capture.samples.lock() {
                if buf.len() < MAX_SAMPLES {
                    buf.extend_from_slice(samples);
                }
                let start = samples.len().saturating_sub(64);
                for s in &samples[start..] {
                    peak = peak.max((*s as i32).abs());
                }
            }
            capture
                .level
                .store(peak.min(32767) as u32, Ordering::Relaxed);
            let raw = band_energy(samples);
            if let Ok(mut bands) = capture.bands.lock() {
                for (i, target) in raw.iter().enumerate() {
                    let prev = bands[i];
                    // Fast attack, slower release: the meter follows a voice
                    // rather than flickering with every buffer.
                    bands[i] = if *target > prev {
                        prev + (target - prev) * 0.65
                    } else {
                        prev * 0.80 + target * 0.20
                    };
                }
            }
        }
        let _ = AudioQueueEnqueueBuffer(queue, buffer, 0, std::ptr::null());
    }
}

pub struct Recorder {
    queue: AudioQueueRef,
    capture: Arc<Capture>,
    /// The extra Arc handed to the audio thread, freed in Drop.
    user_data: *mut c_void,
    started: std::time::Instant,
}

unsafe impl Send for Recorder {}

fn error_text(status: OSStatus) -> String {
    format!(
        "the microphone could not start (AudioQueue code {status}). Allow Flowline in System Settings › Privacy & Security › Microphone."
    )
}

impl Recorder {
    /// Start recording 16 kHz mono PCM. `preferred` is only a hint; macOS uses
    /// the system default input, which is the one the settings row shows.
    pub fn start(preferred: Option<&str>) -> Result<Recorder, String> {
        let _ = preferred;
        let format = AudioStreamBasicDescription {
            mSampleRate: TARGET_RATE as f64,
            mFormatID: K_AUDIO_FORMAT_LINEAR_PCM,
            mFormatFlags: K_LINEAR_PCM_FLAG_SIGNED_INTEGER | K_LINEAR_PCM_FLAG_PACKED,
            mBytesPerPacket: 2,
            mFramesPerPacket: 1,
            mBytesPerFrame: 2,
            mChannelsPerFrame: 1,
            mBitsPerChannel: 16,
            mReserved: 0,
        };
        let capture = Arc::new(Capture {
            samples: Mutex::new(Vec::with_capacity(TARGET_RATE as usize * 30)),
            level: AtomicU32::new(0),
            bands: Arc::new(Mutex::new([0f32; 7])),
        });
        let user_data = Arc::into_raw(capture.clone()) as *mut c_void;
        let mut queue: AudioQueueRef = std::ptr::null_mut();
        let status = unsafe {
            AudioQueueNewInput(
                &format,
                on_input,
                user_data,
                std::ptr::null(),
                std::ptr::null(),
                0,
                &mut queue,
            )
        };
        if status != 0 {
            unsafe { drop(Arc::from_raw(user_data as *const Capture)) };
            return Err(error_text(status));
        }
        // Four 50 ms buffers: 20 meter updates a second, which the UI then
        // interpolates to the display refresh rate.
        for _ in 0..4 {
            let mut buffer: AudioQueueBufferRef = std::ptr::null_mut();
            let status = unsafe { AudioQueueAllocateBuffer(queue, 1600, &mut buffer) };
            if status != 0 {
                unsafe {
                    AudioQueueDispose(queue, 1);
                    drop(Arc::from_raw(user_data as *const Capture));
                }
                return Err(error_text(status));
            }
            let status = unsafe { AudioQueueEnqueueBuffer(queue, buffer, 0, std::ptr::null()) };
            if status != 0 {
                unsafe {
                    AudioQueueDispose(queue, 1);
                    drop(Arc::from_raw(user_data as *const Capture));
                }
                return Err(error_text(status));
            }
        }
        let status = unsafe { AudioQueueStart(queue, std::ptr::null()) };
        if status != 0 {
            unsafe {
                AudioQueueDispose(queue, 1);
                drop(Arc::from_raw(user_data as *const Capture));
            }
            return Err(error_text(status));
        }
        Ok(Recorder {
            queue,
            capture,
            user_data,
            started: std::time::Instant::now(),
        })
    }

    /// Peak input level, 0..1, for the listening meter.
    pub fn level(&self) -> f32 {
        self.capture.level.load(Ordering::Relaxed) as f32 / 32767.0
    }

    /// Seven smoothed voice bands, 0..1, updated every 100 ms.
    pub fn bands(&self) -> [f32; 7] {
        self.capture
            .bands
            .lock()
            .map(|b| *b)
            .unwrap_or([0f32; 7])
    }

    /// Share the live bands with the floating overlay, which repaints on its
    /// own thread while the hub is hidden.
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
        if !self.queue.is_null() {
            unsafe {
                let _ = AudioQueueStop(self.queue, 1);
                let _ = AudioQueueDispose(self.queue, 1);
            }
            self.queue = std::ptr::null_mut();
        }
        if !self.user_data.is_null() {
            unsafe { drop(Arc::from_raw(self.user_data as *const Capture)) };
            self.user_data = std::ptr::null_mut();
        }
    }
}

/// Microphones with an input, read from the system's own audio report.
pub fn input_names() -> Vec<String> {
    let mut out = Vec::new();
    if let Ok(report) = std::process::Command::new("system_profiler")
        .args(["SPAudioDataType", "-json"])
        .output()
    {
        if let Ok(value) = serde_json::from_slice::<Value>(&report.stdout) {
            walk_devices(&value, &mut out);
        }
    }
    out
}

fn walk_devices(value: &Value, out: &mut Vec<String>) {
    if let Some(object) = value.as_object() {
        let name = object.get("_name").and_then(|v| v.as_str());
        let inputs = object
            .get("coreaudio_device_input")
            .and_then(|v| v.as_u64())
            .unwrap_or(0);
        if let Some(name) = name {
            if inputs > 0 && !out.iter().any(|n| n == name) {
                out.push(name.to_string());
            }
        }
        for child in object.values() {
            walk_devices(child, out);
        }
    } else if let Some(array) = value.as_array() {
        for child in array {
            walk_devices(child, out);
        }
    }
}

pub fn wav_from_samples(samples: &[f32]) -> Vec<u8> {
    let pcm: Vec<i16> = samples
        .iter()
        .map(|s| (s.clamp(-1.0, 1.0) * 32767.0) as i16)
        .collect();
    wav_from_pcm16(&pcm)
}

pub fn wav_from_pcm16(samples: &[i16]) -> Vec<u8> {
    let data_len = samples.len() * 2;
    let mut out = Vec::with_capacity(44 + data_len);
    out.extend_from_slice(b"RIFF");
    out.extend_from_slice(&((36 + data_len) as u32).to_le_bytes());
    out.extend_from_slice(b"WAVEfmt ");
    out.extend_from_slice(&16u32.to_le_bytes());
    out.extend_from_slice(&1u16.to_le_bytes());
    out.extend_from_slice(&1u16.to_le_bytes());
    out.extend_from_slice(&TARGET_RATE.to_le_bytes());
    out.extend_from_slice(&(TARGET_RATE * 2).to_le_bytes());
    out.extend_from_slice(&2u16.to_le_bytes());
    out.extend_from_slice(&16u16.to_le_bytes());
    out.extend_from_slice(b"data");
    out.extend_from_slice(&(data_len as u32).to_le_bytes());
    for s in samples {
        out.extend_from_slice(&s.to_le_bytes());
    }
    out
}

/// Length of a 16 kHz mono 16-bit WAV, used for the "3.2 s audio" label.
pub fn wav_seconds(bytes: &[u8]) -> f32 {
    if bytes.len() <= 44 {
        0.0
    } else {
        (bytes.len() - 44) as f32 / (TARGET_RATE as f32 * 2.0)
    }
}

pub fn is_wav(bytes: &[u8]) -> bool {
    bytes.len() > 44 && &bytes[..4] == b"RIFF" && &bytes[8..12] == b"WAVE"
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn voice_bands_follow_the_dominant_tone() {
        let tone = |f: f32| -> Vec<i16> {
            (0..1600)
                .map(|i| {
                    ((i as f32 * 2.0 * std::f32::consts::PI * f / TARGET_RATE as f32).sin()
                        * 12000.0) as i16
                })
                .collect()
        };
        let peak = |b: [f32; 7]| {
            b.iter()
                .enumerate()
                .max_by(|a, b| a.1.partial_cmp(b.1).unwrap())
                .map(|(i, _)| i)
                .unwrap()
        };
        assert_eq!(peak(band_energy(&tone(440.0))), 2);
        assert_eq!(peak(band_energy(&tone(4000.0))), 5);
    }

    #[test]
    fn wav_header_is_16k_mono() {
        let w = wav_from_samples(&[0.0; 16000]);
        assert_eq!(&w[..4], b"RIFF");
        assert_eq!(&w[8..12], b"WAVE");
        assert_eq!(u32::from_le_bytes([w[24], w[25], w[26], w[27]]), 16000);
        assert_eq!(w.len(), 44 + 32000);
        assert!((wav_seconds(&w) - 1.0).abs() < 0.01);
    }

    #[test]
    fn finds_input_devices() {
        let names = input_names();
        assert!(names.iter().any(|n| !n.is_empty()));
    }
}

/// Peak amplitude (0.0–1.0) of a 16-bit PCM WAV. Used to tell a silent clip
/// from speech before a model gets a chance to hallucinate a word for it.
pub fn wav_peak(bytes: &[u8]) -> f32 {
    if bytes.len() < 44 || &bytes[..4] != b"RIFF" {
        return 0.0;
    }
    let mut i = 12usize;
    while i + 8 <= bytes.len() {
        let id = &bytes[i..i + 4];
        let len = u32::from_le_bytes([bytes[i + 4], bytes[i + 5], bytes[i + 6], bytes[i + 7]])
            as usize;
        let start = i + 8;
        if id == b"data" {
            let end = (start + len).min(bytes.len());
            let mut peak = 0i32;
            let mut j = start;
            while j + 2 <= end {
                let s = i16::from_le_bytes([bytes[j], bytes[j + 1]]) as i32;
                peak = peak.max(s.abs());
                j += 2;
            }
            return peak as f32 / 32768.0;
        }
        i = start + len + (len & 1);
    }
    0.0
}

#[cfg(test)]
mod peak_tests {
    use super::*;

    #[test]
    fn peak_of_silence_is_zero_and_speech_is_loud() {
        let silent = wav_from_pcm16(&vec![0i16; 1600]);
        assert!(wav_peak(&silent) < 0.001);
        let loud = wav_from_pcm16(&vec![12000i16; 1600]);
        assert!(wav_peak(&loud) > 0.3);
    }
}

// ---- Quiet-speech mode ------------------------------------------------------

/// Loudness the boosted speech is brought to (RMS, about -23 dBFS).
pub const QUIET_TARGET_RMS: f32 = 0.07;
/// Never boost by more than this (+32 dB): beyond it a clip is mostly mic hiss.
pub const QUIET_MAX_GAIN: f32 = 40.0;
/// A clip whose speech level is below this is treated as silence, not speech.
const QUIET_FLOOR_RMS: f32 = 0.0005;
/// Raw peak a quiet-mode clip must reach to be sent at all (normal: 0.01).
pub const QUIET_MIN_PEAK: f32 = 0.003;
pub const NORMAL_MIN_PEAK: f32 = 0.01;

/// The 16-bit samples of a WAV made by this module.
pub fn wav_samples(bytes: &[u8]) -> Option<Vec<i16>> {
    if bytes.len() < 44 || &bytes[..4] != b"RIFF" {
        return None;
    }
    let mut i = 12usize;
    while i + 8 <= bytes.len() {
        let id = &bytes[i..i + 4];
        let len = u32::from_le_bytes([bytes[i + 4], bytes[i + 5], bytes[i + 6], bytes[i + 7]])
            as usize;
        let start = i + 8;
        if id == b"data" {
            let end = (start + len).min(bytes.len());
            return Some(
                bytes[start..end]
                    .chunks_exact(2)
                    .map(|c| i16::from_le_bytes([c[0], c[1]]))
                    .collect(),
            );
        }
        i = start + len + (len & 1);
    }
    None
}

/// Gain that brings the speech in `samples` to `QUIET_TARGET_RMS`. Speech level
/// is the 90th-percentile 20 ms frame, so a cough or a pause does not decide
/// it. Never attenuates, and returns 1.0 for silence.
pub fn quiet_gain(samples: &[i16]) -> f32 {
    const FRAME: usize = 320; // 20 ms at 16 kHz
    let mut levels: Vec<f32> = samples
        .chunks(FRAME)
        .filter(|f| f.len() == FRAME)
        .map(|f| {
            let sum: f64 = f.iter().map(|&s| (s as f64 / 32768.0).powi(2)).sum();
            (sum / f.len() as f64).sqrt() as f32
        })
        .collect();
    if levels.is_empty() {
        return 1.0;
    }
    levels.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let p90 = levels[((levels.len() - 1) as f32 * 0.9) as usize];
    if p90 < QUIET_FLOOR_RMS {
        return 1.0;
    }
    (QUIET_TARGET_RMS / p90).clamp(1.0, QUIET_MAX_GAIN)
}

/// Smoothly squash anything past 0.8 so a boosted spike cannot clip hard.
fn soft_clip(x: f32) -> f32 {
    let a = x.abs();
    if a <= 0.8 {
        x
    } else {
        x.signum() * (0.8 + 0.19 * ((a - 0.8) / 0.19).tanh())
    }
}

/// Boost a quiet clip. Returns the new WAV and the gain used (1.0 = untouched).
pub fn boost_quiet(wav: &[u8]) -> (Vec<u8>, f32) {
    let Some(samples) = wav_samples(wav) else {
        return (wav.to_vec(), 1.0);
    };
    let gain = quiet_gain(&samples);
    if gain <= 1.001 {
        return (wav.to_vec(), 1.0);
    }
    let boosted: Vec<i16> = samples
        .iter()
        .map(|&s| (soft_clip(s as f32 / 32768.0 * gain) * 32767.0) as i16)
        .collect();
    (wav_from_pcm16(&boosted), gain)
}

#[cfg(test)]
mod quiet_tests {
    use super::*;

    fn tone(amplitude: f32, secs: f32) -> Vec<i16> {
        let n = (TARGET_RATE as f32 * secs) as usize;
        (0..n)
            .map(|i| {
                ((i as f32 * 2.0 * std::f32::consts::PI * 300.0 / TARGET_RATE as f32).sin()
                    * amplitude
                    * 32767.0) as i16
            })
            .collect()
    }

    fn rms(samples: &[i16]) -> f32 {
        let sum: f64 = samples.iter().map(|&s| (s as f64 / 32768.0).powi(2)).sum();
        (sum / samples.len() as f64).sqrt() as f32
    }

    #[test]
    fn soft_speech_is_brought_to_the_target_level() {
        let wav = wav_from_pcm16(&tone(0.01, 1.0));
        let (out, gain) = boost_quiet(&wav);
        assert!(gain > 8.0 && gain < 12.0, "gain {gain}");
        let samples = wav_samples(&out).unwrap();
        assert!((rms(&samples) - QUIET_TARGET_RMS).abs() < 0.01);
        assert!(wav_peak(&out) < 0.99);
    }

    #[test]
    fn loud_speech_is_left_alone() {
        let wav = wav_from_pcm16(&tone(0.5, 1.0));
        let (out, gain) = boost_quiet(&wav);
        assert_eq!(gain, 1.0);
        assert_eq!(out, wav);
    }

    #[test]
    fn silence_and_hiss_are_not_amplified() {
        let silent = wav_from_pcm16(&vec![0i16; 16000]);
        assert_eq!(boost_quiet(&silent).1, 1.0);
        // Faint enough to be mic hiss rather than a voice.
        let hiss = wav_from_pcm16(&tone(0.0004, 1.0));
        assert_eq!(boost_quiet(&hiss).1, 1.0);
    }

    #[test]
    fn gain_is_capped() {
        let wav = wav_from_pcm16(&tone(0.002, 1.0));
        let (_, gain) = boost_quiet(&wav);
        assert_eq!(gain, QUIET_MAX_GAIN);
    }

    #[test]
    fn a_loud_burst_does_not_decide_the_level() {
        // One second of soft speech with a short loud click in it.
        let mut s = tone(0.01, 1.0);
        for v in s.iter_mut().skip(8000).take(160) {
            *v = 30000;
        }
        let gain = quiet_gain(&s);
        assert!(gain > 8.0, "gain {gain}");
    }

    #[test]
    fn boosted_spikes_are_soft_clipped_not_wrapped() {
        let mut s = tone(0.01, 1.0);
        s[100] = 20000;
        let (out, _) = boost_quiet(&wav_from_pcm16(&s));
        assert!(wav_peak(&out) <= 0.99);
    }
}
