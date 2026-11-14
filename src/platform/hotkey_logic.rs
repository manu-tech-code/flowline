//! The Linux global-shortcut state machine, kept free of any device code so it
//! can be tested anywhere.
//!
//! Linux lets Flowline *read* the keyboard but not take keys away from other
//! apps, so the shortcuts are chosen not to collide with what apps use:
//!
//! * hold-to-talk: **Right Ctrl** or **Right Alt** held alone for 120 ms (a
//!   chord such as Right Ctrl+C never starts dictation), or **Ctrl+Space**;
//! * transforms: **Ctrl+Alt+Shift+1…9**;
//! * command mode: hold **Ctrl+Alt+Space**;
//! * **Esc** cancels.
//!
//! Key codes are Linux evdev codes (`linux/input-event-codes.h`).

use super::shared::HotkeyEvent;

pub const KEY_ESC: u16 = 1;
pub const KEY_1: u16 = 2;
pub const KEY_9: u16 = 10;
pub const KEY_SPACE: u16 = 57;
pub const KEY_LEFTCTRL: u16 = 29;
pub const KEY_RIGHTCTRL: u16 = 97;
pub const KEY_LEFTSHIFT: u16 = 42;
pub const KEY_RIGHTSHIFT: u16 = 54;
pub const KEY_LEFTALT: u16 = 56;
pub const KEY_RIGHTALT: u16 = 100;
pub const KEY_LEFTMETA: u16 = 125;
pub const KEY_RIGHTMETA: u16 = 126;

/// How long a lone modifier must be held before dictation starts. Long enough
/// that pressing it as part of a chord never triggers dictation.
pub const HOLD_TO_START_MS: u64 = 120;

const LCTRL: u32 = 1;
const RCTRL: u32 = 2;
const LALT: u32 = 4;
const RALT: u32 = 8;
const LSHIFT: u32 = 16;
const RSHIFT: u32 = 32;
const META: u32 = 64;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Ptt {
    RightCtrl,
    RightAlt,
    CtrlSpace,
}

impl Ptt {
    pub fn from_label(label: &str) -> Ptt {
        match label {
            "Right Alt" => Ptt::RightAlt,
            "Ctrl Space" => Ptt::CtrlSpace,
            _ => Ptt::RightCtrl,
        }
    }
    pub fn to_index(self) -> i32 {
        match self {
            Ptt::RightCtrl => 0,
            Ptt::RightAlt => 1,
            Ptt::CtrlSpace => 2,
        }
    }
    pub fn from_index(i: i32) -> Ptt {
        match i {
            1 => Ptt::RightAlt,
            2 => Ptt::CtrlSpace,
            _ => Ptt::RightCtrl,
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Config {
    pub ptt: Ptt,
    /// Bit n set = Ctrl+Alt+Shift+(n+1) is a live transform shortcut.
    pub xform_slots: u32,
    pub command: bool,
}

#[derive(Default, Debug)]
pub struct Chords {
    held: u32,
    /// When a lone push-to-talk modifier went down, until it has been held long enough.
    pending: Option<u64>,
    ptt_down: bool,
    space_ptt_down: bool,
    cmd_down: bool,
}

fn modifier_bit(code: u16) -> Option<u32> {
    Some(match code {
        KEY_LEFTCTRL => LCTRL,
        KEY_RIGHTCTRL => RCTRL,
        KEY_LEFTALT => LALT,
        KEY_RIGHTALT => RALT,
        KEY_LEFTSHIFT => LSHIFT,
        KEY_RIGHTSHIFT => RSHIFT,
        KEY_LEFTMETA | KEY_RIGHTMETA => META,
        _ => return None,
    })
}

impl Chords {
    fn ctrl(&self) -> bool {
        self.held & (LCTRL | RCTRL) != 0
    }
    fn alt(&self) -> bool {
        self.held & (LALT | RALT) != 0
    }
    fn shift(&self) -> bool {
        self.held & (LSHIFT | RSHIFT) != 0
    }
    fn meta(&self) -> bool {
        self.held & META != 0
    }

    /// The modifier keys currently down, as a bit mask (0 = none).
    pub fn held_mask(&self) -> u32 {
        self.held
    }

    /// Feed one key event: `value` is 1 for press, 0 for release, 2 for repeat.
    pub fn key(&mut self, code: u16, value: i32, now_ms: u64, cfg: &Config) -> Vec<HotkeyEvent> {
        let mut out = Vec::new();
        if value == 2 {
            return out; // auto-repeat never means anything here
        }
        let down = value == 1;

        if let Some(bit) = modifier_bit(code) {
            if down {
                self.held |= bit;
            } else {
                self.held &= !bit;
            }
            let is_ptt_key = matches!(
                (cfg.ptt, code),
                (Ptt::RightCtrl, KEY_RIGHTCTRL) | (Ptt::RightAlt, KEY_RIGHTALT)
            );
            if is_ptt_key {
                if down {
                    // Only a modifier pressed on its own can start dictation.
                    if self.held & !bit == 0 {
                        self.pending = Some(now_ms);
                    }
                } else {
                    self.pending = None;
                    if self.ptt_down {
                        self.ptt_down = false;
                        out.push(HotkeyEvent::Stop);
                    }
                }
            } else if down {
                // Another modifier arrived first: this is a chord, not push-to-talk.
                self.pending = None;
            }
            if !down && self.cmd_down && !(self.ctrl() && self.alt()) {
                self.cmd_down = false;
                out.push(HotkeyEvent::CommandStop);
            }
            return out;
        }

        if down {
            self.pending = None; // a normal key with the modifier is a chord
            if code == KEY_ESC {
                out.push(HotkeyEvent::Cancel);
            }
            if (KEY_1..=KEY_9).contains(&code) {
                let slot = (code - KEY_1) as u32;
                if self.ctrl()
                    && self.alt()
                    && self.shift()
                    && !self.meta()
                    && cfg.xform_slots >> slot & 1 == 1
                {
                    out.push(HotkeyEvent::Transform(slot as u8));
                }
            }
            if code == KEY_SPACE {
                if cfg.command
                    && !self.cmd_down
                    && self.ctrl()
                    && self.alt()
                    && !self.shift()
                    && !self.meta()
                {
                    self.cmd_down = true;
                    out.push(HotkeyEvent::CommandStart);
                } else if cfg.ptt == Ptt::CtrlSpace
                    && !self.space_ptt_down
                    && self.ctrl()
                    && !self.alt()
                    && !self.shift()
                    && !self.meta()
                {
                    self.space_ptt_down = true;
                    out.push(HotkeyEvent::Start);
                }
            }
        } else if code == KEY_SPACE {
            if self.cmd_down {
                self.cmd_down = false;
                out.push(HotkeyEvent::CommandStop);
            }
            if self.space_ptt_down {
                self.space_ptt_down = false;
                out.push(HotkeyEvent::Stop);
            }
        }
        out
    }

    /// Call regularly: turns a modifier held long enough into a start.
    pub fn tick(&mut self, now_ms: u64) -> Vec<HotkeyEvent> {
        match self.pending {
            Some(since) if now_ms.saturating_sub(since) >= HOLD_TO_START_MS => {
                self.pending = None;
                self.ptt_down = true;
                vec![HotkeyEvent::Start]
            }
            _ => Vec::new(),
        }
    }

    /// A keyboard went away or the settings changed: let go of everything.
    pub fn reset(&mut self) -> Vec<HotkeyEvent> {
        let mut out = Vec::new();
        if self.ptt_down || self.space_ptt_down {
            out.push(HotkeyEvent::Stop);
        }
        if self.cmd_down {
            out.push(HotkeyEvent::CommandStop);
        }
        *self = Chords::default();
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use HotkeyEvent::*;

    fn cfg(ptt: Ptt) -> Config {
        Config { ptt, xform_slots: 0b11, command: true }
    }

    #[test]
    fn right_ctrl_held_alone_starts_then_stops() {
        let (mut c, k) = (Chords::default(), cfg(Ptt::RightCtrl));
        assert!(c.key(KEY_RIGHTCTRL, 1, 0, &k).is_empty());
        assert!(c.tick(50).is_empty(), "too early");
        assert_eq!(c.tick(130), vec![Start]);
        assert!(c.tick(500).is_empty(), "starts only once");
        assert_eq!(c.key(KEY_RIGHTCTRL, 0, 900, &k), vec![Stop]);
    }

    #[test]
    fn a_quick_tap_does_nothing() {
        let (mut c, k) = (Chords::default(), cfg(Ptt::RightCtrl));
        c.key(KEY_RIGHTCTRL, 1, 0, &k);
        assert!(c.key(KEY_RIGHTCTRL, 0, 60, &k).is_empty());
        assert!(c.tick(500).is_empty());
    }

    #[test]
    fn right_ctrl_in_a_chord_never_dictates() {
        let (mut c, k) = (Chords::default(), cfg(Ptt::RightCtrl));
        c.key(KEY_RIGHTCTRL, 1, 0, &k);
        c.key(46, 1, 30, &k); // C
        assert!(c.tick(300).is_empty());
        c.key(46, 0, 320, &k);
        assert!(c.key(KEY_RIGHTCTRL, 0, 400, &k).is_empty());
    }

    #[test]
    fn another_modifier_held_first_blocks_push_to_talk() {
        let (mut c, k) = (Chords::default(), cfg(Ptt::RightCtrl));
        c.key(KEY_LEFTSHIFT, 1, 0, &k);
        c.key(KEY_RIGHTCTRL, 1, 10, &k);
        assert!(c.tick(500).is_empty());
    }

    #[test]
    fn right_alt_mode_ignores_altgr_typing() {
        let (mut c, k) = (Chords::default(), cfg(Ptt::RightAlt));
        c.key(KEY_RIGHTALT, 1, 0, &k);
        c.key(18, 1, 20, &k); // AltGr+E types an accented letter
        assert!(c.tick(400).is_empty());
        // Right Ctrl does nothing in this mode.
        let mut c = Chords::default();
        c.key(KEY_RIGHTCTRL, 1, 0, &k);
        assert!(c.tick(400).is_empty());
    }

    #[test]
    fn ctrl_space_mode_starts_at_once_and_stops_on_release() {
        let (mut c, k) = (Chords::default(), cfg(Ptt::CtrlSpace));
        c.key(KEY_LEFTCTRL, 1, 0, &k);
        assert_eq!(c.key(KEY_SPACE, 1, 5, &k), vec![Start]);
        assert!(c.key(KEY_SPACE, 2, 50, &k).is_empty(), "repeat ignored");
        assert_eq!(c.key(KEY_SPACE, 0, 800, &k), vec![Stop]);
    }

    #[test]
    fn transforms_need_ctrl_alt_shift_and_a_live_slot() {
        let (mut c, k) = (Chords::default(), cfg(Ptt::RightCtrl));
        c.key(KEY_LEFTCTRL, 1, 0, &k);
        c.key(KEY_LEFTALT, 1, 1, &k);
        assert!(c.key(KEY_1 + 1, 1, 2, &k).is_empty(), "no shift yet");
        c.key(KEY_LEFTSHIFT, 1, 3, &k);
        assert_eq!(c.key(KEY_1, 1, 4, &k), vec![Transform(0)]);
        assert_eq!(c.key(KEY_1 + 1, 1, 5, &k), vec![Transform(1)]);
        assert!(c.key(KEY_1 + 2, 1, 6, &k).is_empty(), "slot 3 is not live");
    }

    #[test]
    fn command_mode_is_held_ctrl_alt_space() {
        let (mut c, k) = (Chords::default(), cfg(Ptt::RightCtrl));
        c.key(KEY_LEFTCTRL, 1, 0, &k);
        c.key(KEY_LEFTALT, 1, 1, &k);
        assert_eq!(c.key(KEY_SPACE, 1, 2, &k), vec![CommandStart]);
        assert_eq!(c.key(KEY_SPACE, 0, 900, &k), vec![CommandStop]);
        // Letting go of Alt first also ends it, and only once.
        c.key(KEY_SPACE, 1, 1000, &k);
        assert_eq!(c.key(KEY_LEFTALT, 0, 1500, &k), vec![CommandStop]);
        assert!(c.key(KEY_SPACE, 0, 1600, &k).is_empty());
    }

    #[test]
    fn command_mode_off_does_not_claim_the_chord() {
        let (mut c, mut k) = (Chords::default(), cfg(Ptt::RightCtrl));
        k.command = false;
        c.key(KEY_LEFTCTRL, 1, 0, &k);
        c.key(KEY_LEFTALT, 1, 1, &k);
        assert!(c.key(KEY_SPACE, 1, 2, &k).is_empty());
    }

    #[test]
    fn escape_cancels() {
        let (mut c, k) = (Chords::default(), cfg(Ptt::RightCtrl));
        assert_eq!(c.key(KEY_ESC, 1, 0, &k), vec![Cancel]);
    }

    #[test]
    fn losing_the_keyboard_releases_everything() {
        let (mut c, k) = (Chords::default(), cfg(Ptt::RightCtrl));
        c.key(KEY_RIGHTCTRL, 1, 0, &k);
        c.tick(200);
        assert_eq!(c.reset(), vec![Stop]);
        assert!(c.reset().is_empty());
        assert!(c.tick(900).is_empty());
    }

    #[test]
    fn labels_round_trip() {
        for label in ["Right Ctrl", "Right Alt", "Ctrl Space"] {
            let p = Ptt::from_label(label);
            assert_eq!(Ptt::from_index(p.to_index()), p);
        }
        assert_eq!(Ptt::from_label("Fn"), Ptt::RightCtrl, "a macOS value falls back safely");
    }
}
