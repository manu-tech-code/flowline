use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Debug)]
pub enum Style {
    Default,
    Formal,
    Casual,
    VeryCasual,
    Excited,
}

impl Style {
    pub fn label(&self) -> &'static str {
        match self {
            Style::Default => "Default",
            Style::Formal => "Formal",
            Style::Casual => "Casual",
            Style::VeryCasual => "very casual",
            Style::Excited => "Excited!",
        }
    }
    pub fn subtitle(&self) -> &'static str {
        match self {
            Style::Default => "Balanced",
            Style::Formal => "Caps + Punctuation",
            Style::Casual => "Caps + Less punctuation",
            Style::VeryCasual => "No Caps + Less punctuation",
            Style::Excited => "More exclamations",
        }
    }
    pub fn index(&self) -> usize {
        match self {
            Style::Default => 0,
            Style::Formal => 1,
            Style::Casual => 2,
            Style::VeryCasual => 3,
            Style::Excited => 4,
        }
    }
    pub fn from_index(i: usize) -> Style {
        match i {
            0 => Style::Default,
            1 => Style::Formal,
            2 => Style::Casual,
            3 => Style::VeryCasual,
            _ => Style::Excited,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Cleanup {
    None,
    Light,
    Medium,
    High,
}

impl Cleanup {
    pub fn label(&self) -> &'static str {
        match self {
            Cleanup::None => "None",
            Cleanup::Light => "Light",
            Cleanup::Medium => "Medium",
            Cleanup::High => "High",
        }
    }
    pub fn description(&self) -> &'static str {
        match self {
            Cleanup::None => "Transcribes exactly what you said, including mistakes.",
            Cleanup::Light => "Cleans up filler words and grammar.",
            Cleanup::Medium => "Edits for clarity and conciseness.",
            Cleanup::High => "Rewrites for brevity and polish.",
        }
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub struct DictEntry {
    pub word: String,
    pub replacement: String,
    pub starred: bool,
    pub learned: bool,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct Snippet {
    pub trigger: String,
    pub expansion: String,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct Dictation {
    pub text: String,
    pub raw: String,
    pub app: String,
    pub at: String,
    /// Unix seconds when it was dictated; 0 for entries saved before this
    /// field existed (they are excluded from "this week" maths).
    #[serde(default)]
    pub ts: i64,
    pub seconds: f32,
    pub words: usize,
    /// Which model produced the text, or "sample" when none answered.
    #[serde(default)]
    pub engine: String,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    pub dark: bool,
    pub sounds: bool,
    /// Silence other audio while dictating. Off by default.
    pub mute_media: bool,
    pub launch_at_login: bool,
    pub show_in_dock: bool,
    pub language: String,
    pub microphone: String,
    pub hands_free: bool,
    /// Boost soft or whispered speech before transcribing it.
    pub quiet_mode: bool,
    pub ptt: String,
    pub storage: u8,
    /// OpenAI-compatible local endpoint. Any loopback server works.
    #[serde(alias = "lmf3_endpoint")]
    pub engine_endpoint: String,
    /// Model id sent with every transcription. The person can type any id or
    /// pick one the server reports; `lmf3` is only the default.
    pub model: String,
    /// `"local"` runs the bundled Whisper CLI; `"server"` uses the
    /// OpenAI-compatible endpoint below.
    pub engine: String,
    /// Whisper model file name inside the Flowline models folder.
    pub local_model: String,
    /// bottom-center | bottom-right | top-center | custom
    pub popup_position: String,
    /// Custom popup origin in screen points (only used for `custom`).
    pub popup_x: f32,
    pub popup_y: f32,
    /// How long the "Pasted" pill stays; 0 keeps it until dismissed.
    pub popup_timeout_ms: u64,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            dark: false,
            sounds: true,
            mute_media: false,
            launch_at_login: true,
            show_in_dock: true,
            language: "English (US)".into(),
            microphone: "System default".into(),
            hands_free: false,
            quiet_mode: false,
            ptt: crate::keys::default_ptt().into(),
            storage: 0,
            engine_endpoint: crate::engine::DEFAULT_BASE.into(),
            model: crate::engine::DEFAULT_MODEL.into(),
            engine: "local".into(),
            local_model: String::new(),
            popup_position: "bottom-center".into(),
            popup_x: 0.0,
            popup_y: 0.0,
            popup_timeout_ms: 2000,
        }
    }
}

/// A saved rewrite instruction. Built-ins are fixed; custom ones are edited by
/// the person (the original allows eight beside the two built-ins).
#[derive(Clone, Serialize, Deserialize)]
pub struct Transform {
    pub id: String,
    pub name: String,
    pub prompt: String,
    #[serde(default)]
    pub builtin: bool,
}

pub const MAX_CUSTOM_TRANSFORMS: usize = 8;

pub const POLISH_PROMPT: &str = "You are an editor. Rewrite the user's text so it is clear, \
concise and well written. Keep their meaning, facts, names and voice, and fix grammar and \
punctuation. Do not add information. Reply with the rewritten text only, with no preamble, \
quotes or commentary.";

pub const PROMPT_ENGINEER_PROMPT: &str = "You write prompts for AI assistants. Turn the user's \
rough text into one clear, well-structured prompt: state the goal, give the needed context, list \
constraints, and say what the answer should look like. Keep every detail the user gave and invent \
none. Reply with the finished prompt only, with no preamble or commentary.";

pub fn builtin_transforms() -> Vec<Transform> {
    vec![
        Transform {
            id: "polish".into(),
            name: "Polish".into(),
            prompt: POLISH_PROMPT.into(),
            builtin: true,
        },
        Transform {
            id: "prompt-engineer".into(),
            name: "Prompt Engineer".into(),
            prompt: PROMPT_ENGINEER_PROMPT.into(),
            builtin: true,
        },
    ]
}

/// Where Transforms and Notetaker send chat requests. Nothing is contacted
/// until the person points this at a server (Ollama, LM Studio, llama.cpp…).
#[derive(Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct TextModel {
    pub endpoint: String,
    pub model: String,
}

pub const DEFAULT_TEXT_BASE: &str = "http://127.0.0.1:11434/v1";

impl Default for TextModel {
    fn default() -> Self {
        Self {
            endpoint: DEFAULT_TEXT_BASE.into(),
            model: String::new(),
        }
    }
}

#[derive(Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Transforms {
    pub enabled: bool,
    pub custom: Vec<Transform>,
    /// Run `auto_id` on every dictation before it is pasted.
    pub auto: bool,
    pub auto_id: String,
    /// Hold ⌥Space and speak an edit for the selected text.
    pub command: bool,
}

/// Style tab numbers, as used by `style_for_tab`: 1 Work messages, 2 Email,
/// 3 Personal messages, 4 Other.
pub const TONE_TABS: [(usize, &str); 4] = [
    (1, "Work"),
    (2, "Email"),
    (3, "Personal"),
    (4, "Other"),
];

/// The person's own choice of tone category for one app.
#[derive(Clone, Serialize, Deserialize)]
pub struct AppTone {
    pub app: String,
    pub tab: usize,
}

const EMAIL_APPS: &[&str] = &[
    "mail", "outlook", "microsoft outlook", "spark", "superhuman", "airmail", "thunderbird",
    "mimestream", "hey", "proton mail", "canary mail", "newton mail", "mailspring",
    // Linux window classes
    "thunderbird", "evolution", "geary", "kmail", "claws-mail", "protonmail",
];
const WORK_APPS: &[&str] = &[
    "slack", "microsoft teams", "teams", "zoom", "zoom.us", "webex", "mattermost", "rocket.chat",
    "google chat",
    // Linux window classes
    "teams-for-linux", "element",
];
const PERSONAL_APPS: &[&str] = &[
    "messages", "whatsapp", "telegram", "signal", "discord", "messenger", "wechat", "line",
    "facetime", "instagram", "snapchat", "viber",
    // Linux window classes
    "telegramdesktop", "telegram-desktop", "whatsapp-for-linux", "whatsie", "pidgin", "hexchat",
];
const EMAIL_TITLES: &[&str] = &[
    "gmail", "inbox", "outlook", "proton mail", "yahoo mail", "fastmail", "hey.com",
];
const WORK_TITLES: &[&str] = &["slack", "microsoft teams", "google chat"];
const PERSONAL_TITLES: &[&str] = &[
    "whatsapp", "messenger", "telegram", "discord", "instagram", "messages for web",
];

/// Which tone category an app falls into when the person has not chosen one.
/// `window_title` is only consulted for browsers, to tell Gmail from Slack from
/// WhatsApp Web; anything unrecognised is "Other" (4).
pub fn builtin_tab_for_app(app: &str, window_title: Option<&str>) -> usize {
    let name = app.trim().to_lowercase();
    if EMAIL_APPS.contains(&name.as_str()) {
        return 2;
    }
    if WORK_APPS.contains(&name.as_str()) {
        return 1;
    }
    if PERSONAL_APPS.contains(&name.as_str()) {
        return 3;
    }
    if is_browser(app) {
        if let Some(title) = window_title {
            let t = title.to_lowercase();
            let has = |words: &[&str]| words.iter().any(|w| t.contains(w));
            if has(EMAIL_TITLES) {
                return 2;
            }
            if has(WORK_TITLES) {
                return 1;
            }
            if has(PERSONAL_TITLES) {
                return 3;
            }
        }
    }
    4
}

pub fn is_browser(app: &str) -> bool {
    crate::meeting::BROWSERS.iter().any(|b| b.eq_ignore_ascii_case(app.trim()))
}

/// My Voice preferences.
#[derive(Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct VoiceSettings {
    /// Learn spelling fixes from the person's edits after a paste. Off until
    /// they opt in, because it reads the field Flowline just pasted into.
    pub learn: bool,
}

/// Notetaker preferences.
#[derive(Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct NoteSettings {
    /// Offer to start a note when a meeting begins. Off until the person opts in.
    pub auto_detect: bool,
    /// Apps the person answered "Not a meeting" for.
    pub ignored: Vec<String>,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct State {
    pub dictionary: Vec<DictEntry>,
    pub snippets: Vec<Snippet>,
    pub history: Vec<Dictation>,
    pub note: String,
    pub settings: Settings,
    pub cleanup: Cleanup,
    pub work_style: Style,
    pub email_style: Style,
    pub personal_style: Style,
    pub other_style: Style,
    pub text: TextModel,
    pub transforms: Transforms,
    pub notetaker: NoteSettings,
    pub voice: VoiceSettings,
    /// Pick the Style tab from the app being dictated into.
    pub tone_by_app: bool,
    pub app_tones: Vec<AppTone>,
}

impl Default for State {
    fn default() -> Self {
        Self {
            dictionary: Vec::new(),
            snippets: Vec::new(),
            history: Vec::new(),
            note: String::new(),
            settings: Settings::default(),
            cleanup: Cleanup::Light,
            work_style: Style::Formal,
            email_style: Style::Formal,
            personal_style: Style::Formal,
            other_style: Style::Formal,
            text: TextModel::default(),
            transforms: Transforms::default(),
            notetaker: NoteSettings::default(),
            voice: VoiceSettings::default(),
            tone_by_app: true,
            app_tones: Vec::new(),
        }
    }
}

impl State {
    /// The Style tab (1 Work, 2 Email, 3 Personal, 4 Other) for an app: the
    /// person's own choice first, then the built-in guess.
    pub fn tab_for_app(&self, app: &str, window_title: Option<&str>) -> usize {
        if let Some(r) = self
            .app_tones
            .iter()
            .find(|r| r.app.eq_ignore_ascii_case(app.trim()))
        {
            return r.tab;
        }
        builtin_tab_for_app(app, window_title)
    }

    /// Built-ins first, then the person's own.
    pub fn all_transforms(&self) -> Vec<Transform> {
        let mut v = builtin_transforms();
        v.extend(self.transforms.custom.iter().cloned());
        v
    }
    pub fn transform_by_id(&self, id: &str) -> Option<Transform> {
        self.all_transforms().into_iter().find(|t| t.id == id)
    }
    /// A text model is configured when an endpoint and a model are both set.
    pub fn text_model_ready(&self) -> bool {
        !self.text.endpoint.trim().is_empty() && !self.text.model.trim().is_empty()
    }

    pub fn path() -> Option<std::path::PathBuf> {
        // An explicit file makes tests and demos independent of the real
        // profile without changing where a normal run stores its data.
        if let Some(p) = std::env::var_os("FLOWLINE_STATE") {
            if !p.is_empty() {
                return Some(std::path::PathBuf::from(p));
            }
        }
        crate::paths::data_dir().map(|d| d.join("state.json"))
    }
    pub fn load() -> State {
        if let Some(p) = Self::path() {
            if let Ok(text) = std::fs::read_to_string(&p) {
                if let Ok(mut s) = serde_json::from_str::<State>(&text) {
                    if s.migrate_from_demo() {
                        s.save();
                    }
                    return s;
                }
            }
        }
        State::default()
    }
    /// Remove the sample rows the old demo build seeded, so an existing
    /// profile starts clean on the real engine. Only exact known demo values
    /// are removed; anything the person typed is left alone.
    fn migrate_from_demo(&mut self) -> bool {
        let before = (self.history.len(), self.dictionary.len(), self.snippets.len(), self.note.clone());
        self.history
            .retain(|d| d.engine != "sample" && d.engine != "mock-stt");
        const DEMO_NOTE: &str =
            "- Review the onboarding flow\n- Ship the new empty states\n- Write the launch note";
        if self.note == DEMO_NOTE {
            self.note.clear();
        }
        // The old build appended its fake fallback transcripts to the
        // Scratchpad; drop exactly those lines and keep anything typed.
        const DEMO_SAMPLES: [&str; 6] = [
            "um so I think we should ship the new onboarding flow on Friday",
            "can you send me the latest kubernetes roadmap before the standup",
            "hey team quick update the design review is moved to three thirty",
            "my email is below if you want to follow up",
            "let's circle back after the demo and like finalize the pricing page",
            "uh add a follow up task to check the analytics dashboard tomorrow",
        ];
        if !self.note.is_empty() {
            let kept: Vec<&str> = self
                .note
                .lines()
                .filter(|line| {
                    let l = line.trim().to_lowercase();
                    !DEMO_SAMPLES.iter().any(|s| l == s.to_lowercase())
                        && !l.starts_with("- review the onboarding flow")
                        && !l.starts_with("- ship the new empty states")
                        && !l.starts_with("- write the launch note")
                })
                .collect();
            self.note = kept.join("\n");
            if !self.note.is_empty() {
                self.note.push('\n');
            }
        }
        let seeded_dict = [
            ("kubernetes", "Kubernetes"),
            ("figma", "Figma"),
            ("sequoia", "Sequoia"),
        ];
        self.dictionary.retain(|d| {
            !seeded_dict
                .iter()
                .any(|(w, r)| d.word.eq_ignore_ascii_case(w) && d.replacement == *r)
        });
        let seeded_snippets = [
            ("my email", "alex@example.com"),
            ("my calendar link", "cal.com/alex"),
        ];
        self.snippets.retain(|s| {
            !seeded_snippets
                .iter()
                .any(|(t, e)| s.trigger == *t && s.expansion == *e)
        });
        self.snippets.retain(|s| {
            !(s.trigger == "intro"
                && s.expansion.contains("I lead design at a small studio in Lisbon"))
        });
        before
            != (
                self.history.len(),
                self.dictionary.len(),
                self.snippets.len(),
                self.note.clone(),
            )
    }

    pub fn save(&self) {
        if let Some(p) = Self::path() {
            if let Some(dir) = p.parent() {
                let _ = std::fs::create_dir_all(dir);
            }
            if let Ok(text) = serde_json::to_string_pretty(self) {
                let _ = std::fs::write(&p, text);
            }
        }
    }
}

fn is_word_char(c: char) -> bool {
    c.is_alphanumeric() || c == '\''
}

pub fn replace_word_ci(hay: &str, needle: &str, replacement: &str) -> String {
    if needle.trim().is_empty() {
        return hay.to_string();
    }
    let hay_l: Vec<char> = hay.chars().collect();
    let low: String = hay.to_lowercase();
    let low_chars: Vec<char> = low.chars().collect();
    let needle_l = needle.to_lowercase();
    let n: Vec<char> = needle_l.chars().collect();
    if n.is_empty() || n.len() > low_chars.len() {
        return hay.to_string();
    }
    let mut out = String::new();
    let mut i = 0usize;
    while i < hay_l.len() {
        if i + n.len() <= hay_l.len() && low_chars[i..i + n.len()] == n[..] {
            let before_ok = i == 0 || !is_word_char(hay_l[i - 1]);
            let after_ok = i + n.len() >= hay_l.len() || !is_word_char(hay_l[i + n.len()]);
            if before_ok && after_ok {
                out.push_str(replacement);
                i += n.len();
                continue;
            }
        }
        out.push(hay_l[i]);
        i += 1;
    }
    out
}

pub fn apply_dictionary(text: &str, dict: &[DictEntry]) -> String {
    let mut out = text.to_string();
    for d in dict {
        if !d.word.trim().is_empty() && !d.replacement.trim().is_empty() {
            out = replace_word_ci(&out, &d.word, &d.replacement);
        }
    }
    out
}

pub fn apply_snippets(text: &str, snippets: &[Snippet]) -> String {
    let mut out = text.to_string();
    for s in snippets {
        if s.trigger.trim().is_empty() {
            continue;
        }
        let low = out.to_lowercase();
        let trig = s.trigger.to_lowercase();
        if let Some(pos) = low.find(&trig) {
            out.replace_range(pos..pos + trig.len(), &s.expansion);
        }
    }
    out
}

fn capitalize_sentences(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut start = true;
    for c in s.chars() {
        if start && c.is_alphabetic() {
            out.extend(c.to_uppercase());
            start = false;
        } else {
            out.push(c);
            if matches!(c, '.' | '!' | '?') {
                start = true;
            }
        }
    }
    out
}

pub fn cleanup_text(text: &str, level: Cleanup, style: Style) -> String {
    if level == Cleanup::None {
        return text.trim().to_string();
    }
    let mut t = text.trim().to_string();
    // collapse whitespace
    t = t.split_whitespace().collect::<Vec<_>>().join(" ");
    // filler removal
    let fillers: &[&str] = match level {
        Cleanup::None => &[],
        Cleanup::Light => &["um", "uh", "erm", "you know"],
        Cleanup::Medium => &["um", "uh", "erm", "you know", "like", "i mean"],
        Cleanup::High => &[
            "um",
            "uh",
            "erm",
            "you know",
            "like",
            "i mean",
            "sort of",
            "kind of",
            "basically",
            "actually",
        ],
    };
    for f in fillers {
        let mut guard = 0;
        loop {
            let before = t.clone();
            t = replace_word_ci(&t, f, "");
            t = t.replace("  ", " ");
            if t == before || guard > 8 {
                break;
            }
            guard += 1;
        }
    }
    t = t
        .replace(" ,", ",")
        .replace(" .", ".")
        .replace(",.", ".")
        .replace(".,", ".")
        .replace(",,", ",")
        .replace("..", ".");
    while t.contains("  ") {
        t = t.replace("  ", " ");
    }
    t = t.replace(",.", ".").replace(",.", ".").replace(" .", ".");
    t = t.trim().to_string();
    if t.is_empty() {
        return t;
    }
    t = t.trim_end_matches(',').trim_end().to_string();
    let ends_terminal = t.ends_with('.') || t.ends_with('!') || t.ends_with('?');
    t = capitalize_sentences(&t);
    match style {
        Style::Formal | Style::Default => {
            if !ends_terminal {
                t.push('.');
            }
        }
        Style::Casual => {
            t = t.trim_end_matches('.').to_string();
        }
        Style::VeryCasual => {
            t = t.to_lowercase();
            t = t.trim_end_matches(['.', '!', '?']).to_string();
        }
        Style::Excited => {
            t = t.trim_end_matches(['.', '!', '?']).to_string();
            t.push('!');
        }
    }
    t
}

pub fn style_sample(style: Style, tab: usize) -> String {
    match (tab, style) {
        (1, Style::Formal) => {
            "Hi Alex, it was great talking with you today. Looking forward to our next chat.".into()
        }
        (1, Style::Casual) => {
            "Hi Alex, it was great talking with you today. Looking forward to our next chat".into()
        }
        (1, Style::VeryCasual) => {
            "hi alex it was great talking with you today. looking forward to our next chat".into()
        }
        (1, Style::Excited) => {
            "Hi Alex! It was great talking with you today! Looking forward to our next chat!".into()
        }
        (2, Style::Formal) => {
            "Hey, are you free for lunch tomorrow? Let's do 12 if that works for you.".into()
        }
        (2, Style::Casual) => {
            "Hey, are you free for lunch tomorrow? Let's do 12 if that works for you".into()
        }
        (2, Style::VeryCasual) => {
            "hey are you free for lunch tomorrow? let's do 12 if that works for you".into()
        }
        (2, Style::Excited) => {
            "Hey! Are you free for lunch tomorrow! Let's do 12 if that works for you!".into()
        }
        (3, Style::Formal) => "Hey, if you're free let's chat about the great results.".into(),
        (3, Style::Casual) => "Hey, if you're free let's chat about the great results".into(),
        (3, Style::VeryCasual) => "hey if you're free let's chat about the great results".into(),
        (3, Style::Excited) => "Hey! If you're free let's chat about the great results!".into(),
        (_, Style::Default) | (_, Style::Formal) => {
            "The quarterly numbers look strong, and the team is ready for the next push.".into()
        }
        (_, Style::Casual) => {
            "The quarterly numbers look strong, and the team is ready for the next push".into()
        }
        (_, Style::VeryCasual) => {
            "the quarterly numbers look strong and the team is ready for the next push".into()
        }
        (_, Style::Excited) => {
            "The quarterly numbers look strong! The team is ready for the next push!".into()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cleanup_light_removes_fillers() {
        let out = cleanup_text(
            "um so I think we should ship the new onboarding flow on Friday",
            Cleanup::Light,
            Style::Formal,
        );
        assert_eq!(
            out,
            "So I think we should ship the new onboarding flow on Friday."
        );
    }

    #[test]
    fn cleanup_high_removes_more() {
        let out = cleanup_text(
            "it's like sort of finished, you know",
            Cleanup::High,
            Style::Formal,
        );
        assert_eq!(out, "It's finished.");
    }

    #[test]
    fn very_casual_lowercases() {
        let out = cleanup_text(
            "Hello There. This Is A Test",
            Cleanup::Light,
            Style::VeryCasual,
        );
        assert_eq!(out, "hello there. this is a test");
    }

    #[test]
    fn dictionary_replaces_whole_words() {
        let d = vec![DictEntry {
            word: "kubernetes".into(),
            replacement: "Kubernetes".into(),
            starred: false,
            learned: false,
        }];
        assert_eq!(
            apply_dictionary("the kubernetes roadmap", &d),
            "the Kubernetes roadmap"
        );
        assert_eq!(
            apply_dictionary("kubernetess is not a word", &d),
            "kubernetess is not a word"
        );
    }

    #[test]
    fn snippets_expand_case_insensitively() {
        let s = vec![Snippet {
            trigger: "my email".into(),
            expansion: "alex@example.com".into(),
        }];
        assert_eq!(
            apply_snippets("send it to My Email please", &s),
            "send it to alex@example.com please"
        );
    }

    #[test]
    fn audio_silencing_is_off_by_default() {
        // Silencing other audio is opt-in.
        assert!(!Settings::default().mute_media);
    }

    #[test]
    fn excited_style_adds_bang() {
        let out = cleanup_text("the numbers look strong", Cleanup::Light, Style::Excited);
        assert_eq!(out, "The numbers look strong!");
    }
}

#[cfg(test)]
mod migration_tests {
    use super::*;

    #[test]
    fn drops_demo_note_lines_but_keeps_typed_ones() {
        let mut s = State::default();
        s.note = "um so I think we should ship the new onboarding flow on Friday\nmy own sentence\n".into();
        s.migrate_from_demo();
        assert_eq!(s.note, "my own sentence\n");
    }
}

#[cfg(test)]
mod tone_tests {
    use super::*;

    #[test]
    fn apps_map_to_categories() {
        assert_eq!(builtin_tab_for_app("Mail", None), 2);
        assert_eq!(builtin_tab_for_app("Microsoft Outlook", None), 2);
        assert_eq!(builtin_tab_for_app("Slack", None), 1);
        assert_eq!(builtin_tab_for_app("Messages", None), 3);
        assert_eq!(builtin_tab_for_app("WhatsApp", None), 3);
        assert_eq!(builtin_tab_for_app("Terminal", None), 4);
        assert_eq!(builtin_tab_for_app("Code", None), 4);
    }

    #[test]
    fn browsers_are_told_apart_by_window_title() {
        assert_eq!(builtin_tab_for_app("Google Chrome", Some("Inbox (3) - me@example.com - Gmail")), 2);
        assert_eq!(builtin_tab_for_app("Safari", Some("general | Acme | Slack")), 1);
        assert_eq!(builtin_tab_for_app("Arc", Some("WhatsApp")), 3);
        assert_eq!(builtin_tab_for_app("Google Chrome", Some("Rust documentation")), 4);
        assert_eq!(builtin_tab_for_app("Google Chrome", None), 4);
        // A title never changes a non-browser app.
        assert_eq!(builtin_tab_for_app("Terminal", Some("Gmail")), 4);
    }

    #[test]
    fn the_persons_choice_beats_the_built_in_guess() {
        let mut s = State::default();
        assert_eq!(s.tab_for_app("Slack", None), 1);
        s.app_tones.push(AppTone { app: "slack".into(), tab: 3 });
        assert_eq!(s.tab_for_app("Slack", None), 3);
        assert_eq!(s.tab_for_app("Mail", None), 2);
    }
}
