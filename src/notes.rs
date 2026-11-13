//! Notetaker storage. Notes live in `notes.json` beside `state.json` so a long
//! transcript never makes the per-keystroke settings save slow.

use serde::{Deserialize, Serialize};

/// The original's default maximum recording length ("2hr").
pub const MAX_NOTE_SECS: f32 = 7200.0;
/// The live transcript refreshes by transcribing the microphone in slices of
/// this length; a slice is sent to the engine as soon as it is cut.
pub const SEGMENT_SECS: f32 = 25.0;

#[derive(Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default, Debug)]
pub enum NoteStatus {
    Recording,
    Transcribing,
    Summarizing,
    #[default]
    Done,
}

#[derive(Clone, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct Note {
    pub id: u64,
    pub title: String,
    /// Unix seconds when the note was started.
    pub created: i64,
    pub seconds: f32,
    /// One transcript per `SEGMENT_SECS` slice, in order. Empty while the
    /// slice is still being transcribed or when nothing was said.
    pub segments: Vec<String>,
    /// The person's own notes, typed during or after the recording.
    pub notes: String,
    pub summary: String,
    pub status: NoteStatus,
    /// Why the last transcription or summary failed, shown on the note.
    pub error: String,
}

impl Note {
    pub fn transcript(&self) -> String {
        self.segments
            .iter()
            .enumerate()
            .filter(|(_, s)| !s.trim().is_empty())
            .map(|(i, s)| {
                let at = (i as f32 * SEGMENT_SECS) as u32;
                format!("[{}:{:02}] {}", at / 60, at % 60, s.trim())
            })
            .collect::<Vec<_>>()
            .join("\n")
    }

    pub fn has_transcript(&self) -> bool {
        self.segments.iter().any(|s| !s.trim().is_empty())
    }

    pub fn is_busy(&self) -> bool {
        self.status != NoteStatus::Done
    }

    /// What the summary prompt receives: the transcript and anything the
    /// person typed themselves.
    pub fn summary_input(&self) -> String {
        let mut out = String::new();
        if !self.notes.trim().is_empty() {
            out.push_str("My own notes:\n");
            out.push_str(self.notes.trim());
            out.push_str("\n\n");
        }
        out.push_str("Transcript:\n");
        let plain = self
            .segments
            .iter()
            .filter(|s| !s.trim().is_empty())
            .map(|s| s.trim())
            .collect::<Vec<_>>()
            .join(" ");
        out.push_str(&plain);
        out
    }
}

pub const SUMMARY_PROMPT: &str = "You write meeting notes from a transcript made by speech \\
recognition, which may contain mistakes. Write: one short paragraph saying what the \\
conversation was about; then a heading \"### Key points\" with bullets; then a heading \
\"### Action items\" with one bullet per task, naming who owns it when the transcript says so. \
Use only what is in the transcript and notes, invent nothing, and write \"None\" under a \
heading that has nothing. Reply with the notes only.";

pub const TITLE_PROMPT: &str = "Give this conversation a title of at most six words. Reply \
with the title only, no quotes or punctuation at the end.";

#[derive(Default)]
pub struct NoteStore {
    pub notes: Vec<Note>,
}

fn path() -> Option<std::path::PathBuf> {
    crate::model::State::path().map(|p| p.with_file_name("notes.json"))
}

impl NoteStore {
    pub fn load() -> NoteStore {
        let mut store = NoteStore::default();
        if let Some(p) = path() {
            if let Ok(text) = std::fs::read_to_string(p) {
                if let Ok(mut notes) = serde_json::from_str::<Vec<Note>>(&text) {
                    // A note left "recording" by a quit or crash is finished
                    // with whatever was transcribed.
                    for n in &mut notes {
                        if n.is_busy() {
                            n.status = NoteStatus::Done;
                            if n.error.is_empty() {
                                n.error = "Flowline closed before this note finished.".into();
                            }
                        }
                    }
                    store.notes = notes;
                }
            }
        }
        store
    }

    pub fn save(&self) {
        if let Some(p) = path() {
            if let Some(dir) = p.parent() {
                let _ = std::fs::create_dir_all(dir);
            }
            if let Ok(text) = serde_json::to_string_pretty(&self.notes) {
                let _ = std::fs::write(p, text);
            }
        }
    }

    pub fn get(&self, id: u64) -> Option<&Note> {
        self.notes.iter().find(|n| n.id == id)
    }

    pub fn get_mut(&mut self, id: u64) -> Option<&mut Note> {
        self.notes.iter_mut().find(|n| n.id == id)
    }

    /// Newest first, which is how the list shows them.
    pub fn insert(&mut self, note: Note) {
        self.notes.insert(0, note);
    }

    pub fn remove(&mut self, id: u64) {
        self.notes.retain(|n| n.id != id);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn transcript_numbers_slices_and_skips_empty() {
        let n = Note {
            segments: vec!["hello there".into(), "".into(), "  last bit ".into()],
            ..Default::default()
        };
        assert_eq!(n.transcript(), "[0:00] hello there\n[0:50] last bit");
        assert!(n.has_transcript());
    }

    #[test]
    fn summary_input_includes_own_notes_first() {
        let n = Note {
            notes: "remember budget".into(),
            segments: vec!["we spoke".into(), "about money".into()],
            ..Default::default()
        };
        let s = n.summary_input();
        assert!(s.starts_with("My own notes:\nremember budget"));
        assert!(s.ends_with("Transcript:\nwe spoke about money"));
    }
}
