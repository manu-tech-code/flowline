//! Two real transcription engines:
//!
//! 1. **On this Mac** — the whisper.cpp CLI (`whisper-cli`) with a local
//!    `ggml-*.bin` model. Default, requires no network and no server.
//! 2. **Server** — any OpenAI-compatible endpoint the person configures,
//!    including a self-hosted one on the tailnet. Only that endpoint is ever
//!    contacted; there is no hidden fallback to another host.
//!
//! There is no sample transcript and no demo mode. When the engine fails the
//! clip is kept for retry and the error is shown, because fake text is worse
//! than no text.
//!
//! Endpoint resolution for Server mode:
//!   1. `FLOWLINE_ENGINE_URL` / `LMF3_BASE_URL` environment variable
//!   2. the endpoint saved in Settings
//!
//! Model resolution:
//!   1. `FLOWLINE_MODEL` / `LMF3_MODEL` environment variable
//!   2. the model saved in Settings
//!   3. [`DEFAULT_MODEL`]

use serde_json::Value;
use std::io::{Read, Write};
use std::process::Command;
use std::net::{TcpStream, ToSocketAddrs};
use std::time::Duration;

/// Server mode starts from the id OpenAI-compatible servers use for Whisper.
pub const DEFAULT_MODEL: &str = "whisper-1";
pub const DEFAULT_BASE: &str = "http://127.0.0.1:9000/v1";

/// One server the person can pick in Settings. Picking a preset fills in its
/// endpoint and a model that server commonly serves; both stay editable.
pub struct Preset {
    pub name: &'static str,
    pub base: &'static str,
    pub model: &'static str,
    pub note: &'static str,
}

pub const PRESETS: [Preset; 4] = [
    Preset {
        name: "oMLX",
        base: "http://127.0.0.1:9000/v1",
        model: "whisper-large-v3-turbo",
        note: "oMLX on this Mac",
    },
    Preset {
        name: "LM Studio",
        base: "http://127.0.0.1:1234/v1",
        model: "whisper-large-v3-turbo",
        note: "LM Studio's local server",
    },
    Preset {
        name: "Ollama",
        base: "http://127.0.0.1:11434/v1",
        model: "whisper",
        note: "Ollama's OpenAI-compatible API",
    },
    Preset {
        name: "llama.cpp",
        base: "http://127.0.0.1:8080/v1",
        model: "whisper-1",
        note: "llama.cpp server with an ASR model",
    },
];

/// Model ids worth offering when the server lists none. None of these is
/// required: the field accepts anything the server serves.
pub const SUGGESTED_MODELS: [&str; 7] = [
    "whisper-1",
    "whisper-large-v3",
    "whisper-large-v3-turbo",
    "distil-whisper-large-v3",
    "faster-whisper",
    "parakeet-tdt-0.6b",
    "canary-1b",
];

/// Servers that commonly run local models on a Mac. 9000 is oMLX, 8931 is the
/// harness's local provider, 1234 is LM Studio, 8080 is llama.cpp and 11434 is
/// Ollama (it answered on this Mac with seven chat models, no ASR model).
pub fn env_first(keys: &[&str]) -> Option<String> {
    for key in keys {
        if let Ok(v) = std::env::var(key) {
            let v = v.trim().to_string();
            if !v.is_empty() {
                return Some(v);
            }
        }
    }
    None
}

/// The model id to send, combining the environment and the saved setting.
pub fn requested_model(configured: &str) -> String {
    env_first(&["FLOWLINE_MODEL", "LMF3_MODEL"])
        .or_else(|| {
            let c = configured.trim();
            if c.is_empty() {
                None
            } else {
                Some(c.to_string())
            }
        })
        .unwrap_or_else(|| DEFAULT_MODEL.to_string())
}

pub fn requested_base(configured: &str) -> String {
    env_first(&["FLOWLINE_ENGINE_URL", "LMF3_BASE_URL"])
        .map(|s| s.trim_end_matches('/').to_string())
        .unwrap_or_else(|| {
            let c = configured.trim();
            if c.is_empty() {
                DEFAULT_BASE.to_string()
            } else {
                c.trim_end_matches('/').to_string()
            }
        })
}

/// The preset whose endpoint matches, if any ("Custom" otherwise).
/// True when the endpoint is a well-formed http URL.
pub fn is_valid(url: &str) -> bool {
    parse_url(url).is_ok()
}

/// True when audio would leave this Mac.
pub fn is_remote(url: &str) -> bool {
    match parse_url(url) {
        Ok(u) => !matches!(u.host.as_str(), "127.0.0.1" | "localhost" | "::1"),
        Err(_) => false,
    }
}

/// Host of a configured endpoint, for the "audio is sent to …" label.
pub fn host_of(url: &str) -> Option<String> {
    parse_url(url).ok().map(|u| u.host)
}

pub fn preset_for(base: &str) -> Option<&'static Preset> {
    let base = base.trim().trim_end_matches('/');
    PRESETS.iter().find(|p| p.base == base)
}

/// A rough label, used only to sort and warn in the picker. It never decides
/// whether a model may be used: the field accepts anything.
pub fn looks_like_speech(id: &str) -> bool {
    let l = id.to_lowercase();
    [
        "whisper",
        "parakeet",
        "canary",
        "wav2vec",
        "sensevoice",
        "transcribe",
        "asr",
        "speech",
        "audio",
    ]
    .iter()
    .any(|k| l.contains(k))
}

/// True when an id is a text-to-speech voice/model, not a transcriber.
/// Kokoro, Piper and friends expose `/v1/audio/speech`; they cannot dictation.
pub fn looks_like_tts(id: &str) -> bool {
    let l = id.to_lowercase();
    ["tts", "kokoro", "piper", "vits", "xtts", "bark", "orpheus", "chatterbox"]
        .iter()
        .any(|k| l.contains(k))
}

/// True when any advertised id looks like a speech-to-text model.
pub fn has_speech_model(models: &[String]) -> bool {
    models.iter().any(|m| looks_like_speech(m))
}

/// True when the list looks like text-to-speech rather than dictation.
pub fn has_tts_model(models: &[String]) -> bool {
    models.iter().any(|m| looks_like_tts(m))
}

/// True when a server's model list contains the wanted id. Servers suffix tags
/// (`lmf3:latest`), so an exact match wins but a tagged or longer id counts.
pub fn lists_model(models: &[String], wanted: &str) -> bool {
    let wanted = wanted.trim().to_lowercase();
    if wanted.is_empty() {
        return false;
    }
    models.iter().any(|m| {
        let m = m.to_lowercase();
        m == wanted || m.starts_with(&format!("{wanted}:")) || m.contains(&wanted)
    })
}

/// Messages a background thread sends back to the app.
pub enum Event {
    Probe(Discovery),
    Transcript {
        result: Result<String, String>,
        secs: f32,
        model: String,
    },
}

#[derive(Clone, Debug)]
pub struct Discovery {
    pub base: String,
    pub models: Vec<String>,
    /// Why the chosen model may not work. `None` when the server lists it.
    pub problem: Option<String>,
    /// The server answered `GET /v1/models`.
    pub reachable: bool,
    /// The wanted model is in that list (or the list is empty and the server
    /// is up, in which case we cannot tell and still try).
    pub listed: bool,
}

impl Discovery {
    /// True when the server is up and the model is listed (or unknown).
    pub fn ready(&self) -> bool {
        self.reachable && (self.listed || self.models.is_empty())
    }
}

struct Url {
    host: String,
    port: u16,
    path: String,
}

fn parse_url(url: &str) -> Result<Url, String> {
    let rest = url
        .trim()
        .strip_prefix("http://")
        .ok_or_else(|| "only http:// endpoints are supported (Tailscale included)".to_string())?;
    let rest = rest.trim_end_matches('/');
    let (authority, path) = match rest.find('/') {
        Some(i) => (&rest[..i], &rest[i..]),
        None => (rest, ""),
    };
    if authority.is_empty() {
        return Err("missing host".to_string());
    }
    let (host, port) = match authority.rsplit_once(':') {
        Some((h, p)) => (
            h.to_string(),
            p.parse::<u16>().map_err(|_| "bad port".to_string())?,
        ),
        None => (authority.to_string(), 80u16),
    };
    // Remote hosts are allowed: the endpoint is typed by hand, never guessed.
    Ok(Url {
        host,
        port,
        path: path.to_string(),
    })
}

struct Resp {
    status: u16,
    body: Vec<u8>,
}

fn find(hay: &[u8], needle: &[u8]) -> Option<usize> {
    hay.windows(needle.len()).position(|w| w == needle)
}

fn dechunk(mut b: &[u8]) -> Vec<u8> {
    let mut out = Vec::new();
    loop {
        let Some(pos) = find(b, b"\r\n") else { break };
        let line = String::from_utf8_lossy(&b[..pos]);
        let size =
            usize::from_str_radix(line.split(';').next().unwrap_or("").trim(), 16).unwrap_or(0);
        b = &b[pos + 2..];
        if size == 0 || b.is_empty() {
            break;
        }
        let take = size.min(b.len());
        out.extend_from_slice(&b[..take]);
        b = &b[take..];
        if b.starts_with(b"\r\n") {
            b = &b[2..];
        }
    }
    out
}

fn http(
    base: &str,
    method: &str,
    path: &str,
    content_type: Option<&str>,
    body: &[u8],
    timeout: Duration,
) -> Result<Resp, String> {
    let url = parse_url(base)?;
    let target = format!("{}{}", url.path, path);
    let addr = (url.host.as_str(), url.port)
        .to_socket_addrs()
        .map_err(|e| format!("{e}"))?
        .next()
        .ok_or_else(|| "address not found".to_string())?;
    let mut stream =
        TcpStream::connect_timeout(&addr, timeout).map_err(|e| format!("not running ({e})"))?;
    let _ = stream.set_read_timeout(Some(timeout));
    let _ = stream.set_write_timeout(Some(timeout));

    let mut req = format!(
        "{method} {target} HTTP/1.1\r\nHost: {}:{}\r\nConnection: close\r\nAccept: application/json\r\n",
        url.host, url.port
    );
    if let Some(ct) = content_type {
        req.push_str(&format!("Content-Type: {ct}\r\n"));
    }
    if let Some(key) = env_first(&["FLOWLINE_API_KEY", "LMF3_API_KEY"]) {
        req.push_str(&format!("Authorization: Bearer {key}\r\n"));
    }
    req.push_str(&format!("Content-Length: {}\r\n\r\n", body.len()));
    stream
        .write_all(req.as_bytes())
        .map_err(|e| format!("{e}"))?;
    stream.write_all(body).map_err(|e| format!("{e}"))?;
    let _ = stream.flush();

    let mut raw = Vec::new();
    let _ = stream.read_to_end(&mut raw);
    let split = find(&raw, b"\r\n\r\n").ok_or_else(|| "empty response".to_string())?;
    let head = String::from_utf8_lossy(&raw[..split]).to_string();
    let mut body = raw[split + 4..].to_vec();
    let mut status = 0u16;
    let mut chunked = false;
    for (i, line) in head.split("\r\n").enumerate() {
        if i == 0 {
            status = line
                .split_whitespace()
                .nth(1)
                .and_then(|s| s.parse::<u16>().ok())
                .unwrap_or(0);
        } else if let Some((name, value)) = line.split_once(':') {
            if name.eq_ignore_ascii_case("transfer-encoding")
                && value.to_ascii_lowercase().contains("chunked")
            {
                chunked = true;
            }
        }
    }
    if chunked {
        body = dechunk(&body);
    }
    Ok(Resp { status, body })
}

/// Whisper and friends emit bracketed non-speech markers (`[BLANK_AUDIO]`,
/// `[MUSIC]`, `♪`) for silence. Strip them so silence never becomes history.
pub fn strip_non_speech(text: &str) -> String {
    let mut kept = String::new();
    let mut depth = 0usize;
    for ch in text.chars() {
        match ch {
            '[' => depth += 1,
            ']' if depth > 0 => depth -= 1,
            _ if depth == 0 => kept.push(ch),
            _ => {}
        }
    }
    let mut cleaned = kept.replace('♪', " ");
    for tag in [
        "(silence)",
        "(music)",
        "(applause)",
        "(laughter)",
        "(noise)",
        "(inaudible)",
        "(blank_audio)",
    ] {
        cleaned = cleaned.replace(tag, " ");
    }
    cleaned.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn snippet(bytes: &[u8]) -> String {
    let text = String::from_utf8_lossy(bytes);
    let text = text.split_whitespace().collect::<Vec<_>>().join(" ");
    text.chars().take(180).collect()
}

pub fn models(base: &str) -> Result<Vec<String>, String> {
    let r = http(
        base,
        "GET",
        "/models",
        None,
        &[],
        Duration::from_millis(1400),
    )?;
    if r.status == 401 || r.status == 403 {
        return Err("the server wants an API key".to_string());
    }
    if r.status != 200 {
        return Err(format!("HTTP {}: {}", r.status, snippet(&r.body)));
    }
    let v: Value =
        serde_json::from_slice(&r.body).map_err(|e| format!("unreadable answer: {e}"))?;
    let mut out = Vec::new();
    if let Some(arr) = v.get("data").and_then(|d| d.as_array()) {
        for m in arr {
            if let Some(id) = m.get("id").and_then(|x| x.as_str()) {
                out.push(id.to_string());
            }
        }
    }
    Ok(out)
}

/// Probe exactly the configured endpoint. Only that server is ever contacted:
/// silently falling back to another local server made the model list show a
/// different machine's models after a remote endpoint went down.
pub fn discover(configured: &str, wanted: &str) -> Discovery {
    let wanted = requested_model(wanted);
    let base = requested_base(configured);
    match models(&base) {
        Ok(list) => {
            let listed = lists_model(&list, &wanted);
            let problem = if listed {
                None
            } else {
                Some(format!(
                    "server is up but {wanted} is not in its model list{}",
                    if list.is_empty() {
                        String::new()
                    } else {
                        format!(" (it lists: {})", list.join(", "))
                    }
                ))
            };
            Discovery {
                base,
                models: list,
                problem,
                reachable: true,
                listed,
            }
        }
        Err(e) => Discovery {
            base: base.clone(),
            models: Vec::new(),
            problem: Some(format!("{base}: {e}")),
            reachable: false,
            listed: false,
        },
    }
}

fn push_field(body: &mut Vec<u8>, boundary: &str, name: &str, value: &str) {
    body.extend_from_slice(format!("--{boundary}\r\n").as_bytes());
    body.extend_from_slice(
        format!("Content-Disposition: form-data; name=\"{name}\"\r\n\r\n").as_bytes(),
    );
    body.extend_from_slice(value.as_bytes());
    body.extend_from_slice(b"\r\n");
}

fn push_file(body: &mut Vec<u8>, boundary: &str, name: &str, filename: &str, bytes: &[u8]) {
    body.extend_from_slice(format!("--{boundary}\r\n").as_bytes());
    body.extend_from_slice(
        format!("Content-Disposition: form-data; name=\"{name}\"; filename=\"{filename}\"\r\nContent-Type: audio/wav\r\n\r\n")
            .as_bytes(),
    );
    body.extend_from_slice(bytes);
    body.extend_from_slice(b"\r\n");
}

/// Send a 16 kHz mono WAV clip to the chosen model and return its transcript.
pub fn transcribe(base: &str, model: &str, wav: &[u8], language: &str) -> Result<String, String> {
    if wav.len() < 44 || &wav[..4] != b"RIFF" {
        return Err("that file is not a WAV recording".to_string());
    }
    // The caller has already resolved env + settings; use its id verbatim so
    // the request always carries exactly the model shown in the UI.
    let model = if model.trim().is_empty() {
        DEFAULT_MODEL
    } else {
        model.trim()
    };
    let boundary = format!("----flowline{}", std::process::id());
    let mut body = Vec::with_capacity(wav.len() + 512);
    push_field(&mut body, &boundary, "model", &model);
    push_field(&mut body, &boundary, "language", language);
    push_field(&mut body, &boundary, "response_format", "json");
    push_field(&mut body, &boundary, "temperature", "0");
    push_file(&mut body, &boundary, "file", "dictation.wav", wav);
    body.extend_from_slice(format!("--{boundary}--\r\n").as_bytes());

    let ct = format!("multipart/form-data; boundary={boundary}");
    let r = http(
        base,
        "POST",
        "/audio/transcriptions",
        Some(&ct),
        &body,
        Duration::from_secs(90),
    )?;
    if r.status != 200 {
        return Err(format!("HTTP {}: {}", r.status, snippet(&r.body)));
    }
    let v: Value = serde_json::from_slice(&r.body)
        .map_err(|_| format!("unexpected answer: {}", snippet(&r.body)))?;
    let text = v
        .get("text")
        .and_then(|t| t.as_str())
        .or_else(|| v.get("transcript").and_then(|t| t.as_str()))
        .ok_or_else(|| format!("no transcript in answer: {}", snippet(&r.body)))?;
    Ok(strip_non_speech(text))
}

/// Models on a server that can answer a chat prompt: everything except speech,
/// text-to-speech, embedding and image models.
pub fn text_models(base: &str) -> Result<Vec<String>, String> {
    let all = models(base)?;
    Ok(all
        .into_iter()
        .filter(|m| {
            let l = m.to_lowercase();
            !looks_like_speech(m)
                && !looks_like_tts(m)
                && !l.contains("embed")
                && !l.contains("image")
                && !l.contains("flux")
                && !l.contains("diffusion")
        })
        .collect())
}

/// Drop a reasoning model's `<think>…</think>` preamble and stray code fences.
fn clean_chat_text(text: &str) -> String {
    let mut s = text.to_string();
    while let (Some(a), Some(b)) = (s.find("<think>"), s.find("</think>")) {
        if b < a {
            break;
        }
        s.replace_range(a..b + "</think>".len(), "");
    }
    s.trim().to_string()
}

/// One non-streaming chat completion against an OpenAI-compatible server.
/// Used by Transforms (rewrite a passage) and Notetaker (summarise a meeting).
/// Only the endpoint typed in Settings or on the Transforms page is contacted.
pub fn chat(
    base: &str,
    model: &str,
    system: &str,
    user: &str,
    timeout: Duration,
) -> Result<String, String> {
    let model = model.trim();
    if model.is_empty() {
        return Err("no text model chosen — pick one on the Transforms page".to_string());
    }
    let body = serde_json::json!({
        "model": model,
        "stream": false,
        "temperature": 0.3,
        "messages": [
            {"role": "system", "content": system},
            {"role": "user", "content": user},
        ],
    });
    let bytes = serde_json::to_vec(&body).map_err(|e| e.to_string())?;
    let r = http(
        base,
        "POST",
        "/chat/completions",
        Some("application/json"),
        &bytes,
        timeout,
    )?;
    if r.status != 200 {
        return Err(format!("HTTP {}: {}", r.status, snippet(&r.body)));
    }
    let v: Value = serde_json::from_slice(&r.body)
        .map_err(|_| format!("unexpected answer: {}", snippet(&r.body)))?;
    let text = v
        .pointer("/choices/0/message/content")
        .and_then(|t| t.as_str())
        .ok_or_else(|| format!("no text in answer: {}", snippet(&r.body)))?;
    let text = clean_chat_text(text);
    if text.is_empty() {
        return Err("the model returned an empty answer".to_string());
    }
    Ok(text)
}

/// The app sends the language label it shows in Settings, not an ISO code.
pub fn language_code(label: &str) -> &'static str {
    match label {
        "Spanish" => "es",
        "French" => "fr",
        "German" => "de",
        "English (UK)" => "en",
        _ => "en",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn silence_markers_are_not_transcripts() {
        assert_eq!(strip_non_speech("[BLANK_AUDIO] [BLANK_AUDIO]"), "");
        assert_eq!(strip_non_speech("hi [MUSIC] there ♪"), "hi there");
        assert_eq!(strip_non_speech("(silence)"), "");
        assert_eq!(strip_non_speech("  Hello,   world. "), "Hello, world.");
    }

    #[test]
    fn accepts_self_hosted_remote_endpoints() {
        assert!(parse_url("http://10.0.0.5:9000/v1").is_ok());
        assert!(parse_url("http://100.95.206.58:11434/v1").is_ok());
        assert!(parse_url("http://127.0.0.1:9000/v1").is_ok());
        assert!(is_remote("http://100.95.206.58:11434/v1"));
        assert!(!is_remote("http://127.0.0.1:9000/v1"));
        assert_eq!(
            host_of("http://100.95.206.58:11434/v1").as_deref(),
            Some("100.95.206.58")
        );
    }

    #[test]
    fn parses_base_path() {
        let u = parse_url("http://127.0.0.1:9000/v1/").unwrap();
        assert_eq!(u.port, 9000);
        assert_eq!(u.path, "/v1");
    }

    #[test]
    fn dechunks() {
        let raw = b"4\r\nWiki\r\n5\r\npedia\r\n0\r\n\r\n";
        assert_eq!(dechunk(raw), b"Wikipedia");
    }

    #[test]
    fn matches_tagged_and_contained_models() {
        let list = vec!["lmf3:latest".to_string(), "whisper-large-v3".to_string()];
        assert!(lists_model(&list, "lmf3"));
        assert!(lists_model(&list, "WHISPER-LARGE-V3"));
        assert!(!lists_model(&list, "parakeet"));
    }

    #[test]
    fn finds_preset_and_custom() {
        assert_eq!(
            preset_for("http://127.0.0.1:1234/v1").unwrap().name,
            "LM Studio"
        );
        assert!(preset_for("http://127.0.0.1:7777/v1").is_none());
    }

    #[test]
    fn empty_setting_falls_back_to_default_model() {
        assert_eq!(requested_model(""), DEFAULT_MODEL);
        assert_eq!(requested_model("whisper-1"), "whisper-1");
    }
}

// ---------------------------------------------------------------------------
// On-device Whisper (whisper.cpp)
// ---------------------------------------------------------------------------

use std::path::PathBuf;

/// `~/Library/Application Support/Flowline/models`
pub fn local_dir() -> Option<PathBuf> {
    crate::paths::models_dir()
}

/// Where a release build keeps its own copy of the engine, relative to the
/// executable.
/// * macOS: `Flowline.app/Contents/MacOS/flowline` →
///   `Flowline.app/Contents/Resources/bin/whisper-cli`.
/// * Linux archive: `flowline` beside `bin/whisper-cli`.
/// * Linux AppImage: `usr/bin/flowline` beside `usr/bin/whisper-cli`.
pub fn bundled_whisper_paths(exe: &std::path::Path) -> Vec<PathBuf> {
    let Some(dir) = exe.parent() else { return Vec::new() };
    let mut out = vec![dir.join("bin/whisper-cli"), dir.join("whisper-cli")];
    if let Some(up) = dir.parent() {
        out.push(up.join("Resources/bin/whisper-cli"));
    }
    out
}

/// The first of those that exists (kept for callers that want one path).
pub fn bundled_whisper_path(exe: &std::path::Path) -> Option<PathBuf> {
    bundled_whisper_paths(exe).into_iter().find(|p| p.is_file())
}

/// Locate the whisper.cpp CLI. In order: `FLOWLINE_WHISPER_BIN`, the copy
/// inside the app bundle (what a downloaded Flowline uses, so nothing else
/// needs installing), Homebrew, then `PATH` (development runs).
pub fn whisper_bin() -> Option<PathBuf> {
    if let Some(p) = env_first(&["FLOWLINE_WHISPER_BIN"]).map(PathBuf::from) {
        if p.is_file() {
            return Some(p);
        }
    }
    if let Some(p) = std::env::current_exe()
        .ok()
        .and_then(|exe| bundled_whisper_path(&exe))
    {
        return Some(p);
    }
    for candidate in [
        "/opt/homebrew/bin/whisper-cli",
        "/usr/local/bin/whisper-cli",
    ] {
        let p = PathBuf::from(candidate);
        if p.exists() {
            return Some(p);
        }
    }
    std::env::var_os("PATH").and_then(|paths| {
        std::env::split_paths(&paths)
            .map(|dir| dir.join("whisper-cli"))
            .find(|p| p.exists())
    })
}

/// A speech model the app can fetch for the person. Sizes are approximate.
pub struct Download {
    pub id: &'static str,
    pub label: &'static str,
    pub mb: u32,
    pub note: &'static str,
}

pub const DOWNLOADS: [Download; 3] = [
    Download { id: "small", label: "Small", mb: 466, note: "Recommended: accurate and fast" },
    Download { id: "base", label: "Base", mb: 142, note: "Smallest and fastest, less accurate" },
    Download {
        id: "large-v3-turbo-q5_0",
        label: "Turbo",
        mb: 574,
        note: "Most accurate, slower on older Macs",
    },
];

const MODEL_HOST: &str = "https://huggingface.co/ggerganov/whisper.cpp/resolve/main";
const VAD_URL: &str =
    "https://huggingface.co/ggml-org/whisper-vad/resolve/main/ggml-silero-v6.2.0.bin";

pub fn model_url(id: &str) -> String {
    format!("{MODEL_HOST}/ggml-{id}.bin")
}

/// Download one file with the system `curl`, writing to a `.part` file that is
/// renamed only on success. `progress` is the bytes received so far.
fn fetch(url: &str, dest: &std::path::Path, progress: &std::sync::atomic::AtomicU64) -> Result<(), String> {
    use std::sync::atomic::Ordering;
    let part = dest.with_extension("part");
    let _ = std::fs::remove_file(&part);
    let mut child = Command::new("/usr/bin/curl")
        .args(["-L", "--fail", "--silent", "--show-error", "--connect-timeout", "20", "-o"])
        .arg(&part)
        .arg(url)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .map_err(|e| format!("could not start curl: {e}"))?;
    let status = loop {
        match child.try_wait() {
            Ok(Some(st)) => break st,
            Ok(None) => {
                let got = std::fs::metadata(&part).map(|m| m.len()).unwrap_or(0);
                progress.store(got, Ordering::Relaxed);
                std::thread::sleep(Duration::from_millis(200));
            }
            Err(e) => {
                let _ = child.kill();
                let _ = std::fs::remove_file(&part);
                return Err(format!("download interrupted: {e}"));
            }
        }
    };
    if !status.success() {
        let mut msg = String::new();
        if let Some(mut e) = child.stderr.take() {
            let _ = e.read_to_string(&mut msg);
        }
        let _ = std::fs::remove_file(&part);
        let first = msg.lines().next().unwrap_or("download failed").to_string();
        return Err(first);
    }
    let size = std::fs::metadata(&part).map(|m| m.len()).unwrap_or(0);
    if size < 100_000 {
        let _ = std::fs::remove_file(&part);
        return Err("the download was incomplete".to_string());
    }
    progress.store(size, Ordering::Relaxed);
    std::fs::rename(&part, dest).map_err(|e| format!("could not save the model: {e}"))
}

/// Download a speech model (and the small voice-detection model Whisper uses
/// to ignore silence) into the models folder. Returns the model's file name.
pub fn download_model(id: &str, progress: &std::sync::atomic::AtomicU64) -> Result<String, String> {
    if !DOWNLOADS.iter().any(|d| d.id == id) {
        return Err(format!("unknown model {id}"));
    }
    let dir = local_dir().ok_or_else(|| "no home directory".to_string())?;
    std::fs::create_dir_all(&dir).map_err(|e| format!("could not create the models folder: {e}"))?;
    let name = format!("ggml-{id}.bin");
    let dest = dir.join(&name);
    if !dest.is_file() {
        fetch(&model_url(id), &dest, progress)?;
    }
    if vad_model().is_none() {
        // Best effort: dictation still works without it.
        let _ = fetch(
            VAD_URL,
            &dir.join("ggml-silero-v6.2.0.bin"),
            &std::sync::atomic::AtomicU64::new(0),
        );
    }
    Ok(name)
}

/// Installed `.bin` models, newest-looking names sorted for a stable menu.
pub fn local_models() -> Vec<String> {
    let Some(dir) = local_dir() else {
        return Vec::new();
    };
    let mut out: Vec<String> = std::fs::read_dir(dir)
        .map(|rd| {
            rd.filter_map(|e| e.ok())
                .map(|e| e.path())
                .filter(|p| p.extension().map(|x| x == "bin").unwrap_or(false))
                .filter(|p| {
                    let name = p.file_name().unwrap_or_default().to_string_lossy().to_lowercase();
                    !name.contains("silero") && !name.contains("vad")
                })
                .filter_map(|p| p.file_name().map(|n| n.to_string_lossy().into_owned()))
                .collect()
        })
        .unwrap_or_default();
    out.sort();
    out
}

/// Pick the configured model file, or the first installed one.
pub fn local_model_file(configured: &str) -> Result<PathBuf, String> {
    let dir = local_dir().ok_or_else(|| "no home directory".to_string())?;
    let wanted = configured.trim();
    if !wanted.is_empty() {
        let p = dir.join(wanted);
        if p.is_file() {
            return Ok(p);
        }
    }
    local_models()
        .into_iter()
        .map(|m| dir.join(m))
        .next()
        .ok_or_else(|| {
            crate::keys::local(
                "No speech model installed yet. Download one below; it is stored on this Mac and \
                 audio never leaves it.",
            )
        })
}

/// Optional Silero VAD model; when present, whisper.cpp drops non-speech
/// segments instead of hallucinating words for noise.
pub fn vad_model() -> Option<PathBuf> {
    let dir = local_dir()?;
    std::fs::read_dir(dir)
        .ok()?
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .find(|p| {
            let n = p.file_name().unwrap_or_default().to_string_lossy().to_lowercase();
            n.contains("silero") && n.ends_with(".bin")
        })
}

pub fn local_ready(configured: &str) -> (bool, String) {
    match (whisper_bin(), local_model_file(configured)) {
        (Some(_), Ok(file)) => {
            let name = file
                .file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_else(|| "whisper".into());
            (true, name)
        }
        (None, _) => (
            false,
            if cfg!(target_os = "macos") {
                "The on-device engine is missing from this copy of Flowline. Reinstall it, or when \
                 building from source run scripts/build-whisper.sh (or `brew install whisper-cpp`)."
            } else {
                "The on-device engine (whisper-cli) was not found. Use the release archive, which \
                 includes it, or put whisper-cli on your PATH."
            }
            .into(),
        ),
        (_, Err(e)) => (false, e),
    }
}

/// Run whisper.cpp on a 16 kHz WAV and return its transcript. This is a real
/// local transcription: the binary and model must both exist.
pub fn transcribe_local(wav: &[u8], model: &str, language: &str) -> Result<String, String> {
    transcribe_local_opts(wav, model, language, false)
}

/// `quiet` relaxes Whisper's voice-activity threshold (0.5 → 0.3) so soft
/// speech is not dropped as silence. It has no effect without a VAD model.
pub fn transcribe_local_opts(
    wav: &[u8],
    model: &str,
    language: &str,
    quiet: bool,
) -> Result<String, String> {
    // Digital silence makes Whisper invent a word ("you", "Thank you"). Check
    // the clip itself before running the model.
    if crate::audio::wav_peak(wav) < 0.01 {
        return Ok(String::new());
    }
    let bin = whisper_bin().ok_or_else(|| "whisper.cpp (whisper-cli) is not installed".to_string())?;
    let model_file = local_model_file(model)?;
    let dir = std::env::temp_dir().join(format!(
        "flowline-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0)
    ));
    std::fs::create_dir_all(&dir).map_err(|e| format!("temp dir: {e}"))?;
    let wav_path = dir.join("clip.wav");
    let out_base = dir.join("clip");
    std::fs::write(&wav_path, wav).map_err(|e| format!("write wav: {e}"))?;

    let mut cmd = Command::new(&bin);
    cmd.arg("-m")
        .arg(&model_file)
        .arg("-f")
        .arg(&wav_path)
        .arg("-l")
        .arg(if language.trim().is_empty() {
            "auto"
        } else {
            language
        })
        .arg("-otxt")
        .arg("-of")
        .arg(&out_base)
        .arg("-np")
        .arg("-nt");
    if let Some(vad) = vad_model() {
        cmd.arg("--vad").arg("-vm").arg(vad);
        if quiet {
            cmd.arg("-vt").arg("0.3");
        }
    }
    let output = cmd.output().map_err(|e| format!("whisper-cli: {e}"))?;
    if !output.status.success() {
        let err = String::from_utf8_lossy(&output.stderr);
        let _ = std::fs::remove_dir_all(&dir);
        return Err(format!(
            "whisper-cli failed: {}",
            err.lines().next().unwrap_or("unknown error")
        ));
    }
    let mut txt = out_base.with_extension("txt");
    if !txt.exists() {
        // whisper.cpp appends .txt to the base name.
        txt = PathBuf::from(format!("{}.txt", out_base.display()));
    }
    let text = std::fs::read_to_string(&txt)
        .map_err(|e| format!("read transcript: {e}"))?;
    let _ = std::fs::remove_dir_all(&dir);
    Ok(strip_non_speech(&text))
}

#[cfg(test)]
mod portable_tests {
    use super::*;

    #[test]
    fn bundled_engine_is_looked_for_where_each_package_puts_it() {
        let mac = std::path::Path::new("/Applications/Flowline.app/Contents/MacOS/flowline");
        assert!(bundled_whisper_paths(mac).contains(&std::path::PathBuf::from(
            "/Applications/Flowline.app/Contents/Resources/bin/whisper-cli"
        )));
        let archive = std::path::Path::new("/opt/flowline-0.3.0/flowline");
        assert_eq!(
            bundled_whisper_paths(archive)[0],
            std::path::PathBuf::from("/opt/flowline-0.3.0/bin/whisper-cli")
        );
        let appimage = std::path::Path::new("/tmp/.mount_x/usr/bin/flowline");
        assert!(bundled_whisper_paths(appimage)
            .contains(&std::path::PathBuf::from("/tmp/.mount_x/usr/bin/whisper-cli")));
    }

    #[test]
    fn download_ids_are_fixed_and_urls_are_well_formed() {
        assert!(DOWNLOADS.iter().any(|d| d.id == "small"));
        assert_eq!(
            model_url("small"),
            "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-small.bin"
        );
        let progress = std::sync::atomic::AtomicU64::new(0);
        assert!(download_model("../../etc/passwd", &progress).is_err());
    }

    #[test]
    fn fetch_saves_a_file_and_leaves_no_partial_download() {
        let dir = std::env::temp_dir().join(format!("flowline-fetch-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let src = dir.join("source.bin");
        std::fs::write(&src, vec![7u8; 300_000]).unwrap();
        let dest = dir.join("model.bin");
        let progress = std::sync::atomic::AtomicU64::new(0);
        fetch(&format!("file://{}", src.display()), &dest, &progress).unwrap();
        assert_eq!(std::fs::metadata(&dest).unwrap().len(), 300_000);
        assert_eq!(progress.load(std::sync::atomic::Ordering::Relaxed), 300_000);
        assert!(!dest.with_extension("part").exists());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_failed_or_tiny_download_is_an_error_and_cleans_up() {
        let dir = std::env::temp_dir().join(format!("flowline-fetch-bad-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let progress = std::sync::atomic::AtomicU64::new(0);
        let missing = dir.join("missing.bin");
        let dest = dir.join("model.bin");
        assert!(fetch(&format!("file://{}", missing.display()), &dest, &progress).is_err());
        assert!(!dest.exists() && !dest.with_extension("part").exists());
        let tiny = dir.join("tiny.bin");
        std::fs::write(&tiny, b"<html>not a model</html>").unwrap();
        assert!(fetch(&format!("file://{}", tiny.display()), &dest, &progress).is_err());
        assert!(!dest.exists() && !dest.with_extension("part").exists());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn default_model_is_not_tied_to_one_persons_server() {
        assert_eq!(DEFAULT_MODEL, "whisper-1");
        assert!(!SUGGESTED_MODELS.contains(&"lmf3"));
    }
}
