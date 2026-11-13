//! Runtime for Transforms and Notetaker: background work, recording slices,
//! and the hook that rewrites a dictation before it is pasted.
//!
//! Everything here talks to the engines the person configured: Whisper on this
//! Mac (or the typed server) for speech, and the text model typed on the
//! Transforms page for rewrites and summaries. Nothing is invented — when a
//! model is missing or fails, the note or transform says so.

use crate::notes::*;
use crate::{audio, engine, meeting, platform, voice, App};
use std::collections::{HashMap, HashSet};
use std::sync::mpsc::{channel, Receiver, Sender};
use std::time::{Duration, Instant};

/// How long a dictation waits for its auto-applied transform before pasting
/// the text as dictated.
pub const AUTO_TRANSFORM_TIMEOUT: Duration = Duration::from_secs(25);

pub enum Msg {
    Segment {
        note: u64,
        idx: usize,
        result: Result<String, String>,
    },
    Summary {
        note: u64,
        result: Result<String, String>,
    },
    Title {
        note: u64,
        result: Result<String, String>,
    },
    Transform {
        auto: bool,
        result: Result<String, String>,
    },
    Models(Result<Vec<String>, String>),
    /// A selection transform finished (the text was already pasted on success).
    Selection {
        name: String,
        result: Result<(), String>,
    },
    /// The text selected when command mode started (`None` = nothing selected).
    CmdSelection(Option<(String, platform::SavedClipboard)>),
    /// A speech-model download finished (the model's file name, or why not).
    ModelDownload(Result<String, String>),
    /// Spelling fixes the person made to a pasted dictation (heard, corrected).
    Learned(Vec<(String, String)>),
    /// The person's answer to the "meeting detected" dialog.
    MeetingChoice {
        key: String,
        label: String,
        choice: MeetingChoice,
    },
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum MeetingChoice {
    Start,
    Ignore,
    Later,
}

/// Parse osascript's `button returned:X, gave up:false` answer.
pub fn parse_meeting_choice(out: &str) -> MeetingChoice {
    if out.contains("gave up:true") {
        MeetingChoice::Later
    } else if out.contains("button returned:Start note") {
        MeetingChoice::Start
    } else if out.contains("button returned:Not a meeting") {
        MeetingChoice::Ignore
    } else {
        MeetingChoice::Later
    }
}

/// The selection captured for a command, while it is being copied or held.
pub enum CmdSel {
    Idle,
    Pending,
    Got(String, platform::SavedClipboard),
    Nothing,
}

pub const COMMAND_PROMPT: &str = "You edit text by following a spoken instruction. The user \
gives an instruction, which was transcribed from speech and may contain small mistakes, and a \
text. Apply the instruction to the text and reply with the resulting text only: no preamble, \
no quotes, no explanation. Keep everything the instruction does not ask you to change. If the \
instruction cannot be applied to the text, reply with the text unchanged.";

pub fn command_input(instruction: &str, text: &str) -> String {
    format!("Instruction: {}\n\nText:\n{}", instruction.trim(), text)
}

pub struct NoteRec {
    pub id: u64,
    pub rec: audio::Recorder,
    pub started: Instant,
    pub seg_started: Instant,
    pub next_idx: usize,
}

/// An open transform editor: `None` index means a new transform.
pub struct TransformEdit {
    pub index: Option<usize>,
    pub name: String,
    pub prompt: String,
}

pub enum Auto {
    Idle,
    Waiting(Instant),
    Ready(Option<String>),
}

pub struct Extras {
    pub tx: Sender<Msg>,
    rx: Receiver<Msg>,
    // Notetaker
    pub nt_sel: Option<u64>,
    /// 0 Summary, 1 Notes, 2 Transcript
    pub nt_tab: usize,
    pub nt_rec: Option<NoteRec>,
    pub pending: HashMap<u64, usize>,
    pub finishing: HashSet<u64>,
    pub nt_confirm_delete: Option<u64>,
    // Transforms
    pub tf_input: String,
    pub tf_output: String,
    pub tf_sel: String,
    pub tf_busy: bool,
    pub tf_error: Option<String>,
    pub tf_edit: Option<TransformEdit>,
    pub models: Vec<String>,
    pub models_status: String,
    pub models_busy: bool,
    pub auto: Auto,
    /// A selection transform is running; further shortcuts are ignored.
    pub sel_busy: bool,
    /// The current recording is a spoken command, not a dictation.
    pub cmd_active: bool,
    pub cmd_sel: CmdSel,
    // Meeting auto-detect
    pub last_meeting_check: Instant,
    /// The app the current dialog/episode was for; cleared when the meeting ends.
    pub prompted: Option<String>,
    pub meeting_dialog: bool,
    /// The running note was started by a detected meeting.
    pub auto_started: bool,
    /// That meeting's app has a call-only helper, so we can see it end.
    pub auto_in_call: bool,
    pub absent_since: Option<Instant>,
    pub last_own_mic: Option<Instant>,
    /// Bumped by every paste so an older correction watcher stands down.
    pub watch_gen: std::sync::Arc<std::sync::atomic::AtomicU64>,
    /// Tone tab chosen for the dictation being finished (see `tone_tab`).
    pub tone_tab: Option<usize>,
    // Speech-model download (Settings → Transcription engine)
    pub dl: Option<&'static engine::Download>,
    pub dl_progress: std::sync::Arc<std::sync::atomic::AtomicU64>,
    pub dl_error: Option<String>,
}

impl Default for Extras {
    fn default() -> Self {
        let (tx, rx) = channel();
        Self {
            tx,
            rx,
            nt_sel: None,
            nt_tab: 0,
            nt_rec: None,
            pending: HashMap::new(),
            finishing: HashSet::new(),
            nt_confirm_delete: None,
            tf_input: String::new(),
            tf_output: String::new(),
            tf_sel: "polish".into(),
            tf_busy: false,
            tf_error: None,
            tf_edit: None,
            models: Vec::new(),
            models_status: String::new(),
            models_busy: false,
            auto: Auto::Idle,
            sel_busy: false,
            cmd_active: false,
            cmd_sel: CmdSel::Idle,
            last_meeting_check: Instant::now(),
            prompted: None,
            meeting_dialog: false,
            auto_started: false,
            auto_in_call: false,
            absent_since: None,
            last_own_mic: None,
            watch_gen: std::sync::Arc::new(std::sync::atomic::AtomicU64::new(0)),
            tone_tab: None,
            dl: None,
            dl_progress: std::sync::Arc::new(std::sync::atomic::AtomicU64::new(0)),
            dl_error: None,
        }
    }
}

/// What the dictation pipeline should do with its cleaned text.
pub enum Gate {
    Proceed(String),
    Wait,
}

fn short(e: &str) -> String {
    e.chars().take(120).collect()
}

impl App {
    // ---- Text model -------------------------------------------------------

    /// Ask the typed endpoint which chat models it serves.
    pub fn detect_text_models(&mut self) {
        if self.ext.models_busy {
            return;
        }
        let base = self.state.text.endpoint.trim().to_string();
        if !engine::is_valid(&base) {
            self.ext.models_status = "Enter an http:// endpoint first.".into();
            return;
        }
        self.ext.models_busy = true;
        self.ext.models_status = "Checking…".into();
        let tx = self.ext.tx.clone();
        std::thread::spawn(move || {
            let _ = tx.send(Msg::Models(engine::text_models(&base)));
        });
    }

    // ---- Transforms -------------------------------------------------------

    /// Run the selected transform on the "Try it" text.
    pub fn run_transform(&mut self) {
        if self.ext.tf_busy {
            return;
        }
        let text = self.ext.tf_input.trim().to_string();
        if text.is_empty() {
            self.ext.tf_error = Some("Type or paste some text first.".into());
            return;
        }
        if !self.state.text_model_ready() {
            self.ext.tf_error = Some("Choose a text model above first.".into());
            return;
        }
        let Some(t) = self.state.transform_by_id(&self.ext.tf_sel) else {
            self.ext.tf_error = Some("That transform no longer exists.".into());
            return;
        };
        self.ext.tf_busy = true;
        self.ext.tf_error = None;
        self.ext.tf_output.clear();
        self.spawn_transform(false, t.prompt, text);
    }

    fn spawn_transform(&self, auto: bool, prompt: String, text: String) {
        let base = self.state.text.endpoint.trim().to_string();
        let model = self.state.text.model.trim().to_string();
        let tx = self.ext.tx.clone();
        std::thread::spawn(move || {
            let result = engine::chat(&base, &model, &prompt, &text, Duration::from_secs(120));
            let _ = tx.send(Msg::Transform { auto, result });
        });
    }

    /// Option+digit: copy the selection in the frontmost app, rewrite it with
    /// transform number `slot`, and paste it back over the selection.
    pub fn selection_transform(&mut self, slot: usize) {
        if !self.state.transforms.enabled || !self.state.text_model_ready() {
            return;
        }
        let Some(t) = self.state.all_transforms().into_iter().nth(slot) else {
            return;
        };
        if self.ext.sel_busy {
            return;
        }
        if !platform::accessibility_granted() {
            platform::request_accessibility();
            self.toast("Allow Accessibility so Flowline can copy and paste for Transforms".into());
            return;
        }
        self.ext.sel_busy = true;
        self.toast(format!("{}: rewriting the selection…", t.name));
        let base = self.state.text.endpoint.trim().to_string();
        let model = self.state.text.model.trim().to_string();
        let tx = self.ext.tx.clone();
        std::thread::spawn(move || {
            // Let the Option key come up before ⌘C goes out.
            std::thread::sleep(Duration::from_millis(80));
            let result = match platform::copy_selection() {
                None => Err("Select some text first".to_string()),
                Some((text, saved)) => {
                    match engine::chat(&base, &model, &t.prompt, &text, Duration::from_secs(120)) {
                        Ok(out) => {
                            platform::paste_and_restore(&out, saved);
                            Ok(())
                        }
                        Err(e) => {
                            platform::restore_saved(saved);
                            Err(e)
                        }
                    }
                }
            };
            let _ = tx.send(Msg::Selection {
                name: t.name,
                result,
            });
        });
    }

    // ---- Command mode ------------------------------------------------------

    /// ⌥Space went down: copy the selection and start listening.
    pub fn start_command(&mut self) {
        let t = &self.state.transforms;
        if !(t.enabled && t.command && self.state.text_model_ready()) {
            return;
        }
        if self.ext.sel_busy || !matches!(self.dict, crate::dictation::Phase::Idle | crate::dictation::Phase::Done { .. }) {
            return;
        }
        if !platform::accessibility_granted() {
            platform::request_accessibility();
            self.toast("Allow Accessibility so Flowline can copy and paste for commands".into());
            return;
        }
        self.ext.cmd_active = true;
        self.ext.cmd_sel = CmdSel::Pending;
        self.global_target = None;
        let tx = self.ext.tx.clone();
        std::thread::spawn(move || {
            std::thread::sleep(Duration::from_millis(80));
            let _ = tx.send(Msg::CmdSelection(platform::copy_selection()));
        });
        self.start_dictation_with(false);
    }

    /// The instruction has been transcribed: apply it to the saved selection.
    /// Returns false while the selection is still being copied.
    pub fn finish_command(&mut self, raw: &str) -> bool {
        if matches!(self.ext.cmd_sel, CmdSel::Pending) {
            return false;
        }
        self.ext.cmd_active = false;
        let sel = std::mem::replace(&mut self.ext.cmd_sel, CmdSel::Idle);
        let instruction = raw.trim().to_string();
        let end = |app: &mut App, msg: String| {
            app.dict = crate::dictation::Phase::Done {
                deadline: Some(Instant::now() + Duration::from_millis(2500)),
                text: msg.clone(),
                pasted: false,
                copied: false,
                head: Some("Command".into()),
            };
            app.toast(msg);
        };
        match sel {
            CmdSel::Got(text, saved) => {
                if instruction.is_empty() {
                    platform::restore_saved(saved);
                    end(self, "I didn't catch a command".into());
                    return true;
                }
                self.ext.sel_busy = true;
                let heard: String = instruction.chars().take(80).collect();
                self.dict = crate::dictation::Phase::Done {
                    deadline: Some(Instant::now() + Duration::from_secs(150)),
                    text: heard,
                    pasted: false,
                    copied: false,
                    head: Some("Command · applying…".into()),
                };
                let base = self.state.text.endpoint.trim().to_string();
                let model = self.state.text.model.trim().to_string();
                let tx = self.ext.tx.clone();
                std::thread::spawn(move || {
                    let user = command_input(&instruction, &text);
                    let result = match engine::chat(&base, &model, COMMAND_PROMPT, &user, Duration::from_secs(120)) {
                        Ok(out) => {
                            platform::paste_and_restore(&out, saved);
                            Ok(())
                        }
                        Err(e) => {
                            platform::restore_saved(saved);
                            Err(e)
                        }
                    };
                    let _ = tx.send(Msg::Selection {
                        name: "Command".into(),
                        result,
                    });
                });
            }
            _ => end(self, "Select some text first, then hold ⌥Space".into()),
        }
        true
    }

    /// Called from the dictation pipeline with the cleaned text. Returns the
    /// text to paste, or `Wait` while the auto-applied transform is running.
    pub fn auto_transform_gate(&mut self, cleaned: String) -> Gate {
        let t = &self.state.transforms;
        let wanted = t.enabled && t.auto && self.state.text_model_ready() && !cleaned.is_empty();
        let transform = if wanted {
            self.state.transform_by_id(&self.state.transforms.auto_id)
        } else {
            None
        };
        let Some(transform) = transform else {
            self.ext.auto = Auto::Idle;
            return Gate::Proceed(cleaned);
        };
        match std::mem::replace(&mut self.ext.auto, Auto::Idle) {
            Auto::Idle => {
                self.ext.auto = Auto::Waiting(Instant::now());
                self.spawn_transform(true, transform.prompt, cleaned);
                Gate::Wait
            }
            Auto::Waiting(since) => {
                if since.elapsed() > AUTO_TRANSFORM_TIMEOUT {
                    self.toast("Transform timed out — pasted as dictated".into());
                    Gate::Proceed(cleaned)
                } else {
                    self.ext.auto = Auto::Waiting(since);
                    Gate::Wait
                }
            }
            Auto::Ready(Some(text)) => Gate::Proceed(text),
            Auto::Ready(None) => Gate::Proceed(cleaned),
        }
    }

    // ---- Notetaker --------------------------------------------------------

    pub fn start_note(&mut self) {
        self.start_note_for(None);
        self.ext.auto_started = false;
    }

    /// Start a note, naming the meeting app it came from when there is one.
    pub fn start_note_for(&mut self, source: Option<&str>) {
        if self.ext.nt_rec.is_some() {
            return;
        }
        let rec = match audio::Recorder::start(None) {
            Ok(r) => r,
            Err(e) => {
                self.toast(short(&e));
                return;
            }
        };
        let now = chrono::Local::now();
        let id = now.timestamp_millis() as u64;
        self.notes.insert(Note {
            id,
            // Keep the "Note · " prefix: a generated title replaces it later.
            title: match source {
                Some(app) => format!("Note · {app} · {}", now.format("%b %-d, %-I:%M %p")),
                None => format!("Note · {}", now.format("%b %-d, %-I:%M %p")),
            },
            created: now.timestamp(),
            status: NoteStatus::Recording,
            ..Default::default()
        });
        self.notes.save();
        self.ext.nt_sel = Some(id);
        self.ext.nt_tab = 2;
        self.ext.nt_rec = Some(NoteRec {
            id,
            rec,
            started: Instant::now(),
            seg_started: Instant::now(),
            next_idx: 0,
        });
    }

    pub fn stop_note(&mut self) {
        let Some(id) = self.ext.nt_rec.as_ref().map(|r| r.id) else {
            return;
        };
        self.ext.auto_started = false;
        self.cut_segment(false);
        self.ext.finishing.insert(id);
        if let Some(n) = self.notes.get_mut(id) {
            n.status = NoteStatus::Transcribing;
        }
        self.notes.save();
        if self.ext.pending.get(&id).copied().unwrap_or(0) == 0 {
            self.finalize_note(id);
        }
    }

    /// Cut the running recording, send that slice for transcription, and (when
    /// `keep_going`) start the next slice straight away.
    fn cut_segment(&mut self, keep_going: bool) {
        let Some(cur) = self.ext.nt_rec.take() else {
            return;
        };
        let NoteRec {
            id,
            rec,
            started,
            next_idx,
            ..
        } = cur;
        // Start the next slice before closing this one so no speech is lost.
        let next = if keep_going {
            audio::Recorder::start(None).ok()
        } else {
            None
        };
        let secs = started.elapsed().as_secs_f32();
        let wav = rec.finish();
        if let Some(n) = self.notes.get_mut(id) {
            n.seconds = secs;
            while n.segments.len() <= next_idx {
                n.segments.push(String::new());
            }
        }
        if let Some(wav) = wav {
            *self.ext.pending.entry(id).or_insert(0) += 1;
            let local = self.uses_local_engine();
            let base = engine::requested_base(&self.state.settings.engine_endpoint);
            let model = engine::requested_model(&self.state.settings.model);
            let local_model = self.state.settings.local_model.clone();
            let language = engine::language_code(&self.state.settings.language).to_string();
            let tx = self.ext.tx.clone();
            std::thread::spawn(move || {
                let result = if local {
                    engine::transcribe_local(&wav, &local_model, &language)
                } else {
                    engine::transcribe(&base, &model, &wav, &language)
                };
                let _ = tx.send(Msg::Segment {
                    note: id,
                    idx: next_idx,
                    result,
                });
            });
        }
        if keep_going {
            match next {
                Some(rec) => {
                    self.ext.nt_rec = Some(NoteRec {
                        id,
                        rec,
                        started,
                        seg_started: Instant::now(),
                        next_idx: next_idx + 1,
                    });
                }
                None => {
                    self.toast("The microphone stopped — ending the note".into());
                    self.ext.finishing.insert(id);
                    if let Some(n) = self.notes.get_mut(id) {
                        n.status = NoteStatus::Transcribing;
                    }
                    if self.ext.pending.get(&id).copied().unwrap_or(0) == 0 {
                        self.finalize_note(id);
                    }
                }
            }
        }
        self.notes.save();
    }

    /// All slices are back: summarise when a text model is set, else finish.
    fn finalize_note(&mut self, id: u64) {
        self.ext.finishing.remove(&id);
        let Some(n) = self.notes.get(id) else { return };
        let has_input = n.has_transcript() || !n.notes.trim().is_empty();
        if !has_input {
            if let Some(n) = self.notes.get_mut(id) {
                n.status = NoteStatus::Done;
                if n.error.is_empty() {
                    n.error = "Nothing was transcribed — check the engine in Settings.".into();
                }
            }
            self.notes.save();
            return;
        }
        if self.state.text_model_ready() {
            self.summarize_note(id);
        } else if let Some(n) = self.notes.get_mut(id) {
            n.status = NoteStatus::Done;
            self.notes.save();
        }
    }

    pub fn summarize_note(&mut self, id: u64) {
        if !self.state.text_model_ready() {
            self.toast("Choose a text model on the Transforms page to summarise".into());
            return;
        }
        let Some(n) = self.notes.get_mut(id) else { return };
        if !(n.has_transcript() || !n.notes.trim().is_empty()) {
            self.toast("There is nothing to summarise yet".into());
            return;
        }
        n.status = NoteStatus::Summarizing;
        n.error.clear();
        let input = n.summary_input();
        self.notes.save();
        let base = self.state.text.endpoint.trim().to_string();
        let model = self.state.text.model.trim().to_string();
        let tx = self.ext.tx.clone();
        std::thread::spawn(move || {
            let result = engine::chat(&base, &model, SUMMARY_PROMPT, &input, Duration::from_secs(300));
            let _ = tx.send(Msg::Summary { note: id, result });
        });
    }

    fn request_title(&self, id: u64, summary: &str) {
        let base = self.state.text.endpoint.trim().to_string();
        let model = self.state.text.model.trim().to_string();
        let tx = self.ext.tx.clone();
        let summary = summary.to_string();
        std::thread::spawn(move || {
            let result = engine::chat(&base, &model, TITLE_PROMPT, &summary, Duration::from_secs(60));
            let _ = tx.send(Msg::Title { note: id, result });
        });
    }

    pub fn delete_note(&mut self, id: u64) {
        if self.ext.nt_rec.as_ref().map(|r| r.id) == Some(id) {
            return;
        }
        self.notes.remove(id);
        self.notes.save();
        if self.ext.nt_sel == Some(id) {
            self.ext.nt_sel = None;
        }
        self.ext.pending.remove(&id);
        self.ext.finishing.remove(&id);
    }

    // ---- Speech-model download --------------------------------------------

    /// Fetch a Whisper model into the models folder, on request. The only
    /// network traffic is a download from huggingface.co.
    pub fn start_model_download(&mut self, id: &'static str) {
        let Some(choice) = engine::DOWNLOADS.iter().find(|d| d.id == id) else {
            return;
        };
        if self.ext.dl.is_some() {
            return;
        }
        self.ext.dl = Some(choice);
        self.ext.dl_error = None;
        self.ext.dl_progress.store(0, std::sync::atomic::Ordering::Relaxed);
        let progress = self.ext.dl_progress.clone();
        let tx = self.ext.tx.clone();
        std::thread::spawn(move || {
            let _ = tx.send(Msg::ModelDownload(engine::download_model(id, &progress)));
        });
    }

    // ---- My Voice ---------------------------------------------------------

    /// Watch the field a dictation was just pasted into, for up to 20 s, and
    /// report any spelling fix the person makes to it. Opt-in; nothing is kept
    /// except the corrected word or phrase.
    pub fn start_learning(&mut self, pasted: String) {
        use std::sync::atomic::Ordering;
        let gen = self.ext.watch_gen.fetch_add(1, Ordering::SeqCst) + 1;
        if !self.state.voice.learn || !platform::accessibility_granted() || pasted.trim().is_empty() {
            return;
        }
        let current = self.ext.watch_gen.clone();
        let tx = self.ext.tx.clone();
        std::thread::spawn(move || {
            let alive = || current.load(Ordering::SeqCst) == gen;
            // Let the paste land and the clipboard settle.
            std::thread::sleep(Duration::from_millis(1500));
            let Some(baseline) = platform::read_focused_text() else {
                return;
            };
            if !baseline.contains(&pasted) {
                return; // not readable, or the paste went somewhere else
            }
            let deadline = Instant::now() + Duration::from_secs(20);
            let mut latest = baseline.clone();
            let mut changed_at: Option<Instant> = None;
            while alive() && Instant::now() < deadline {
                std::thread::sleep(Duration::from_secs(1));
                let Some(now) = platform::read_focused_text() else {
                    continue;
                };
                if now != latest {
                    latest = now;
                    changed_at = Some(Instant::now());
                } else if changed_at.map_or(false, |t| t.elapsed() >= Duration::from_secs(3)) {
                    break; // edited, then left alone
                }
            }
            if alive() && changed_at.is_some() {
                let pairs = voice::learn_pairs(&pasted, &baseline, &latest);
                if !pairs.is_empty() {
                    let _ = tx.send(Msg::Learned(pairs));
                }
            }
        });
    }

    // ---- Meeting auto-detect ----------------------------------------------

    fn check_meeting(&mut self) {
        let snap = meeting::snapshot();

        // End a note that a Zoom meeting started once the meeting is over.
        if self.ext.auto_started && self.ext.auto_in_call && self.ext.nt_rec.is_some() {
            let in_call = snap.app.as_ref().map_or(false, |a| a.in_call);
            if in_call {
                self.ext.absent_since = None;
            } else {
                let since = *self.ext.absent_since.get_or_insert_with(Instant::now);
                if since.elapsed() >= Duration::from_secs(20) {
                    self.stop_note();
                    self.toast("The meeting ended — note stopped".into());
                    return;
                }
            }
        }

        // Which meeting, if any, is this? Zoom's call helper is proof on its
        // own; other apps need the microphone busy for a few seconds, and only
        // once it has been busy since we last used it ourselves, so our own
        // dictation never counts.
        let own_ago = self
            .ext
            .last_own_mic
            .map(|t| t.elapsed().as_secs_f32())
            .unwrap_or(f32::MAX);
        let someone_else = own_ago >= 4.0;
        let candidate: Option<(String, String)> = match &snap.app {
            Some(a) if a.in_call => Some((a.name.to_string(), a.name.to_string())),
            Some(a) if snap.mic_secs >= 6.0 && someone_else => {
                Some((a.name.to_string(), a.name.to_string()))
            }
            None if snap.mic_secs >= 8.0 && someone_else => {
                let front = platform::frontmost_app().unwrap_or_default();
                if meeting::BROWSERS.iter().any(|b| front == *b) {
                    Some(("a browser call".to_string(), "Browser".to_string()))
                } else {
                    None
                }
            }
            _ => None,
        };

        match candidate {
            None => {
                if snap.app.is_none() && snap.mic_secs == 0.0 {
                    self.ext.prompted = None;
                }
            }
            Some((label, key)) => {
                let busy = self.ext.nt_rec.is_some()
                    || self.ext.meeting_dialog
                    || self.rec.is_some()
                    || self.dict.is_active();
                if busy
                    || self.ext.prompted.as_deref() == Some(key.as_str())
                    || self.state.notetaker.ignored.contains(&key)
                {
                    return;
                }
                self.ext.prompted = Some(key.clone());
                self.ext.meeting_dialog = true;
                let tx = self.ext.tx.clone();
                std::thread::spawn(move || {
                    let script = format!(
                        "display dialog \"{label} looks like a meeting. Start a note?\" \
                         with title \"Flowline\" \
                         buttons {{\"Not a meeting\", \"Not now\", \"Start note\"}} \
                         default button \"Start note\" giving up after 40"
                    );
                    let out = std::process::Command::new("osascript")
                        .args(["-e", &script])
                        .output()
                        .map(|o| String::from_utf8_lossy(&o.stdout).into_owned())
                        .unwrap_or_default();
                    let _ = tx.send(Msg::MeetingChoice {
                        key,
                        label,
                        choice: parse_meeting_choice(&out),
                    });
                });
            }
        }
    }

    // ---- Per-frame --------------------------------------------------------

    /// Drain background results and advance the recording. Cheap when idle.
    pub fn tick_features(&mut self, ctx: &eframe::egui::Context) {
        // Keep the global Option+digit shortcuts in step with the settings.
        let live = if self.state.transforms.enabled && self.state.text_model_ready() {
            self.state.all_transforms().len().min(9) as u32
        } else {
            0
        };
        platform::set_transform_slots((1u32 << live) - 1);
        platform::set_command_enabled(
            self.state.transforms.enabled
                && self.state.transforms.command
                && self.state.text_model_ready(),
        );

        // Meeting auto-detect (opt-in).
        meeting::set_enabled(self.state.notetaker.auto_detect);
        if self.ext.nt_rec.is_some() || self.rec.is_some() || self.dict.is_active() {
            self.ext.last_own_mic = Some(Instant::now());
        }
        if self.state.notetaker.auto_detect
            && self.ext.last_meeting_check.elapsed() >= Duration::from_millis(1500)
        {
            self.ext.last_meeting_check = Instant::now();
            self.check_meeting();
        }

        // A command that was cancelled (Esc, too short) must not claim the
        // next dictation, and must give the clipboard back.
        if self.ext.cmd_active && matches!(self.dict, crate::dictation::Phase::Idle) {
            self.ext.cmd_active = false;
            if let CmdSel::Got(_, saved) = std::mem::replace(&mut self.ext.cmd_sel, CmdSel::Idle) {
                platform::restore_saved(saved);
            } else {
                self.ext.cmd_sel = CmdSel::Idle;
            }
        }

        if !matches!(self.dict, crate::dictation::Phase::Transcribing { .. }) {
            self.ext.tone_tab = None;
        }

        // A dictation that was cancelled must not leave a stale transform.
        if !matches!(self.dict, crate::dictation::Phase::Transcribing { .. })
            && !matches!(self.ext.auto, Auto::Idle)
        {
            self.ext.auto = Auto::Idle;
        }

        while let Ok(msg) = self.ext.rx.try_recv() {
            match msg {
                Msg::Models(result) => {
                    self.ext.models_busy = false;
                    match result {
                        Ok(list) => {
                            self.ext.models_status = if list.is_empty() {
                                "Connected, but the server lists no chat models.".into()
                            } else {
                                format!("Connected · {} model(s)", list.len())
                            };
                            let cur = self.state.text.model.clone();
                            if !list.is_empty() && !list.iter().any(|m| *m == cur) {
                                self.state.text.model = list[0].clone();
                                self.save();
                            }
                            self.ext.models = list;
                        }
                        Err(e) => {
                            self.ext.models.clear();
                            self.ext.models_status = format!("Could not connect: {}", short(&e));
                        }
                    }
                }
                Msg::Selection { name, result } => {
                    self.ext.sel_busy = false;
                    let (head, detail) = match &result {
                        Ok(()) => (format!("{name} applied"), String::new()),
                        Err(e) => (format!("{name} failed"), short(e)),
                    };
                    if matches!(self.dict, crate::dictation::Phase::Done { .. }) && name == "Command" {
                        self.dict = crate::dictation::Phase::Done {
                            deadline: Some(Instant::now() + Duration::from_millis(2500)),
                            text: detail.clone(),
                            pasted: false,
                            copied: false,
                            head: Some(head.clone()),
                        };
                    }
                    self.toast(if detail.is_empty() { head } else { format!("{head}: {detail}") });
                }
                Msg::ModelDownload(result) => {
                    let label = self.ext.dl.take().map(|d| d.label).unwrap_or("Model");
                    match result {
                        Ok(file) => {
                            if self.state.settings.local_model.trim().is_empty() {
                                self.state.settings.local_model = file;
                                self.save();
                            }
                            self.probe_engine();
                            self.toast(format!("{label} model ready"));
                        }
                        Err(e) => {
                            self.ext.dl_error = Some(short(&e));
                        }
                    }
                }
                Msg::Learned(pairs) => {
                    let mut added = Vec::new();
                    for (heard, corrected) in pairs {
                        let known = self
                            .state
                            .dictionary
                            .iter()
                            .any(|d| d.word.eq_ignore_ascii_case(&heard));
                        if !known {
                            self.state.dictionary.push(crate::model::DictEntry {
                                word: heard.clone(),
                                replacement: corrected.clone(),
                                starred: false,
                                learned: true,
                            });
                            added.push(format!("{heard} → {corrected}"));
                        }
                    }
                    if !added.is_empty() {
                        self.save();
                        self.toast(format!("Learned: {}", added.join(", ")));
                    }
                }
                Msg::MeetingChoice { key, label, choice } => {
                    self.ext.meeting_dialog = false;
                    match choice {
                        MeetingChoice::Start => {
                            if self.ext.nt_rec.is_none() {
                                self.start_note_for(Some(&label));
                                if self.ext.nt_rec.is_some() {
                                    self.ext.auto_started = true;
                                    self.ext.auto_in_call = label == "Zoom";
                                    self.ext.absent_since = None;
                                    self.toast(format!("Recording a note for {label}"));
                                }
                            }
                        }
                        MeetingChoice::Ignore => {
                            if !self.state.notetaker.ignored.contains(&key) {
                                self.state.notetaker.ignored.push(key);
                                self.save();
                            }
                            self.toast(format!("Won't ask about {label} again"));
                        }
                        MeetingChoice::Later => {}
                    }
                }
                Msg::CmdSelection(sel) => {
                    if self.ext.cmd_active || matches!(self.ext.cmd_sel, CmdSel::Pending) {
                        self.ext.cmd_sel = match sel {
                            Some((text, saved)) => CmdSel::Got(text, saved),
                            None => CmdSel::Nothing,
                        };
                    } else if let Some((_, saved)) = sel {
                        // The command was cancelled before the copy finished.
                        platform::restore_saved(saved);
                    }
                }
                Msg::Transform { auto: false, result } => {
                    self.ext.tf_busy = false;
                    match result {
                        Ok(text) => self.ext.tf_output = text,
                        Err(e) => self.ext.tf_error = Some(short(&e)),
                    }
                }
                Msg::Transform { auto: true, result } => {
                    if matches!(self.ext.auto, Auto::Waiting(_)) {
                        match result {
                            Ok(text) => self.ext.auto = Auto::Ready(Some(text)),
                            Err(e) => {
                                self.toast(format!("Transform failed: {} — pasted as dictated", short(&e)));
                                self.ext.auto = Auto::Ready(None);
                            }
                        }
                    }
                }
                Msg::Segment { note, idx, result } => {
                    if let Some(c) = self.ext.pending.get_mut(&note) {
                        *c = c.saturating_sub(1);
                    }
                    if let Some(n) = self.notes.get_mut(note) {
                        while n.segments.len() <= idx {
                            n.segments.push(String::new());
                        }
                        match result {
                            Ok(text) => n.segments[idx] = text,
                            Err(e) => n.error = format!("Transcription failed: {}", short(&e)),
                        }
                    }
                    self.notes.save();
                    if self.ext.finishing.contains(&note)
                        && self.ext.pending.get(&note).copied().unwrap_or(0) == 0
                    {
                        self.finalize_note(note);
                    }
                }
                Msg::Summary { note, result } => {
                    let mut title_from: Option<String> = None;
                    if let Some(n) = self.notes.get_mut(note) {
                        n.status = NoteStatus::Done;
                        match result {
                            Ok(text) => {
                                if n.title.starts_with("Note · ") {
                                    title_from = Some(text.clone());
                                }
                                n.summary = text;
                                n.error.clear();
                            }
                            Err(e) => n.error = format!("Summary failed: {}", short(&e)),
                        }
                    }
                    self.notes.save();
                    if let Some(s) = title_from {
                        self.request_title(note, &s);
                    }
                }
                Msg::Title { note, result } => {
                    if let (Ok(t), Some(n)) = (result, self.notes.get_mut(note)) {
                        let t = t.trim().trim_matches('"').trim_end_matches('.').to_string();
                        if n.title.starts_with("Note · ") && !t.is_empty() && t.len() < 80 {
                            n.title = t;
                            self.notes.save();
                        }
                    }
                }
            }
        }

        // Rotate the recording slice and enforce the 2 h cap.
        let mut cut = false;
        let mut stop = false;
        if let Some(r) = &self.ext.nt_rec {
            let total = r.started.elapsed().as_secs_f32();
            if total >= MAX_NOTE_SECS {
                stop = true;
            } else if r.seg_started.elapsed().as_secs_f32() >= SEGMENT_SECS {
                cut = true;
            }
            if let Some(n) = self.notes.get_mut(r.id) {
                n.seconds = total;
            }
        }
        if stop {
            self.stop_note();
            self.toast("Note stopped at the 2 hour limit".into());
        } else if cut {
            self.cut_segment(true);
        }

        let busy = self.ext.nt_rec.is_some()
            || self.ext.tf_busy
            || self.ext.sel_busy
            || self.ext.cmd_active
            || self.ext.dl.is_some()
            || self.ext.models_busy
            || !matches!(self.ext.auto, Auto::Idle)
            || self.ext.pending.values().any(|c| *c > 0)
            || self.notes.notes.iter().any(|n| n.status == NoteStatus::Summarizing);
        if busy {
            ctx.request_repaint_after(Duration::from_millis(100));
        }
    }

    pub fn copy_to_clipboard(&mut self, text: &str, what: &str) {
        platform::copy_text(text);
        self.toast(format!("{what} copied"));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dialog_answers_are_parsed() {
        assert_eq!(
            parse_meeting_choice("button returned:Start note, gave up:false"),
            MeetingChoice::Start
        );
        assert_eq!(
            parse_meeting_choice("button returned:Not a meeting, gave up:false"),
            MeetingChoice::Ignore
        );
        assert_eq!(
            parse_meeting_choice("button returned:Start note, gave up:true"),
            MeetingChoice::Later
        );
        assert_eq!(parse_meeting_choice(""), MeetingChoice::Later);
    }

    #[test]
    fn command_input_puts_instruction_before_text() {
        let s = command_input("  make it shorter ", "A very long sentence.");
        assert_eq!(s, "Instruction: make it shorter\n\nText:\nA very long sentence.");
    }
}
