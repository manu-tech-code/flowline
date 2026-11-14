use eframe::egui::{self, Color32, FontData, FontDefinitions, FontFamily, FontId};

pub const SAND_50: Color32 = Color32::from_rgb(0xfc, 0xfc, 0xfb);
pub const SAND_100: Color32 = Color32::from_rgb(0xfa, 0xf9, 0xf7);
pub const SAND_200: Color32 = Color32::from_rgb(0xf8, 0xf7, 0xf3);
pub const SAND_300: Color32 = Color32::from_rgb(0xf7, 0xf6, 0xf2);
pub const SAND_500: Color32 = Color32::from_rgb(0xf5, 0xf4, 0xf0);
pub const SAND_600: Color32 = Color32::from_rgb(0xee, 0xeb, 0xe3);
pub const SAND_700: Color32 = Color32::from_rgb(0xc5, 0xc0, 0xb3);
pub const SAND_800: Color32 = Color32::from_rgb(0xa3, 0x9d, 0x8e);
pub const SAND_900: Color32 = Color32::from_rgb(0x73, 0x6e, 0x62);

pub const VAST_50: Color32 = Color32::from_rgb(0xe9, 0xe8, 0xe1);
pub const VAST_200: Color32 = Color32::from_rgb(0xc8, 0xc8, 0xc2);
pub const VAST_400: Color32 = Color32::from_rgb(0x9d, 0x9c, 0x98);
pub const VAST_600: Color32 = Color32::from_rgb(0x71, 0x71, 0x6e);
pub const VAST_800: Color32 = Color32::from_rgb(0x46, 0x45, 0x44);
pub const VAST_900: Color32 = Color32::from_rgb(0x30, 0x30, 0x2f);
pub const VAST_925: Color32 = Color32::from_rgb(0x20, 0x20, 0x20);
pub const VAST_950: Color32 = Color32::from_rgb(0x1a, 0x1a, 0x1a);

pub const SIGNAL_500: Color32 = Color32::from_rgb(0xff, 0xbc, 0xf2);
pub const GLOW_300: Color32 = Color32::from_rgb(0xff, 0xa9, 0x46);
pub const FATHOM_500: Color32 = Color32::from_rgb(0x68, 0xbd, 0xb0);
pub const CHORD_500: Color32 = Color32::from_rgb(0x96, 0x5a, 0xce);
pub const RED_450: Color32 = Color32::from_rgb(0xff, 0x57, 0x57);
pub const LUMEN_500: Color32 = Color32::from_rgb(0xf7, 0xf7, 0xdb);

#[derive(Clone, Copy, PartialEq, Eq)]
pub struct Palette {
    pub dark: bool,
}

impl Palette {
    pub fn bg(&self) -> Color32 {
        if self.dark {
            VAST_950
        } else {
            SAND_100
        }
    }
    pub fn canvas(&self) -> Color32 {
        if self.dark {
            Color32::from_rgb(0x16, 0x16, 0x16)
        } else {
            SAND_50
        }
    }
    pub fn sidebar(&self) -> Color32 {
        if self.dark {
            Color32::from_rgb(0x1f, 0x1f, 0x1e)
        } else {
            SAND_500
        }
    }
    pub fn card(&self) -> Color32 {
        if self.dark {
            VAST_925
        } else {
            Color32::WHITE
        }
    }
    pub fn card_alt(&self) -> Color32 {
        if self.dark {
            Color32::from_rgb(0x26, 0x26, 0x25)
        } else {
            SAND_100
        }
    }
    pub fn border(&self) -> Color32 {
        if self.dark {
            Color32::from_rgb(0x34, 0x34, 0x32)
        } else {
            SAND_600
        }
    }
    pub fn border_strong(&self) -> Color32 {
        if self.dark {
            Color32::from_rgb(0x46, 0x46, 0x44)
        } else {
            SAND_700
        }
    }
    pub fn text(&self) -> Color32 {
        if self.dark {
            VAST_50
        } else {
            VAST_950
        }
    }
    pub fn text_muted(&self) -> Color32 {
        if self.dark {
            VAST_200
        } else {
            VAST_600
        }
    }
    pub fn text_faint(&self) -> Color32 {
        if self.dark {
            VAST_400
        } else {
            SAND_900
        }
    }
    pub fn accent(&self) -> Color32 {
        if self.dark {
            SAND_100
        } else {
            VAST_950
        }
    }
    pub fn accent_text(&self) -> Color32 {
        if self.dark {
            VAST_950
        } else {
            Color32::WHITE
        }
    }
    pub fn hover(&self) -> Color32 {
        if self.dark {
            Color32::from_rgb(0x2b, 0x2b, 0x2a)
        } else {
            SAND_600
        }
    }
    /// Background behind selected text. Not the accent: the accent is the
    /// opposite of the page, so text selected on it would be unreadable.
    pub fn text_selection(&self) -> Color32 {
        if self.dark {
            Color32::from_rgb(0x3d, 0x57, 0x85)
        } else {
            Color32::from_rgb(0xbf, 0xd8, 0xff)
        }
    }
    pub fn selection(&self) -> Color32 {
        if self.dark {
            Color32::from_rgb(0x3a, 0x3a, 0x38)
        } else {
            SAND_600
        }
    }
}

pub fn install_fonts(ctx: &egui::Context) {
    let mut defs = FontDefinitions::default();

    // Put `bytes` first in `family`, registered as `name`.
    fn add(defs: &mut FontDefinitions, name: &str, bytes: Vec<u8>, family: FontFamily) {
        defs.font_data.insert(
            name.to_owned(),
            std::sync::Arc::new(FontData::from_owned(bytes)),
        );
        defs.families.entry(family).or_default().insert(0, name.to_owned());
    }
    // The first readable file among `paths`.
    fn first(paths: &[&str]) -> Option<Vec<u8>> {
        paths.iter().find_map(|p| std::fs::read(p).ok())
    }

    let (sans, serif, serif_italic): (&[&str], &[&str], &[&str]) = if cfg!(target_os = "macos") {
        (
            &["/System/Library/Fonts/SFNS.ttf", "/System/Library/Fonts/Supplemental/Arial.ttf"],
            &["/System/Library/Fonts/Supplemental/Times New Roman.ttf"],
            &["/System/Library/Fonts/Supplemental/Times New Roman Italic.ttf"],
        )
    } else {
        // Common locations on Debian/Ubuntu, Fedora and Arch. If none is
        // installed the fonts bundled with egui are used.
        (
            &[
                "/usr/share/fonts/truetype/noto/NotoSans-Regular.ttf",
                "/usr/share/fonts/noto/NotoSans-Regular.ttf",
                "/usr/share/fonts/google-noto/NotoSans-Regular.ttf",
                "/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf",
                "/usr/share/fonts/TTF/DejaVuSans.ttf",
                "/usr/share/fonts/dejavu-sans-fonts/DejaVuSans.ttf",
                "/usr/share/fonts/liberation/LiberationSans-Regular.ttf",
            ],
            &[
                "/usr/share/fonts/truetype/noto/NotoSerif-Regular.ttf",
                "/usr/share/fonts/noto/NotoSerif-Regular.ttf",
                "/usr/share/fonts/google-noto/NotoSerif-Regular.ttf",
                "/usr/share/fonts/truetype/dejavu/DejaVuSerif.ttf",
                "/usr/share/fonts/TTF/DejaVuSerif.ttf",
                "/usr/share/fonts/dejavu-serif-fonts/DejaVuSerif.ttf",
                "/usr/share/fonts/liberation/LiberationSerif-Regular.ttf",
            ],
            &[
                "/usr/share/fonts/truetype/noto/NotoSerif-Italic.ttf",
                "/usr/share/fonts/noto/NotoSerif-Italic.ttf",
                "/usr/share/fonts/google-noto/NotoSerif-Italic.ttf",
                "/usr/share/fonts/truetype/dejavu/DejaVuSerif-Italic.ttf",
                "/usr/share/fonts/TTF/DejaVuSerif-Italic.ttf",
                "/usr/share/fonts/dejavu-serif-fonts/DejaVuSerif-Italic.ttf",
                "/usr/share/fonts/liberation/LiberationSerif-Italic.ttf",
            ],
        )
    };
    if let Some(bytes) = first(sans) {
        add(&mut defs, "sf", bytes, FontFamily::Proportional);
    }
    if let Some(bytes) = first(serif) {
        add(&mut defs, "serif", bytes, FontFamily::Name("serif".into()));
    }
    if let Some(bytes) = first(serif_italic) {
        add(&mut defs, "serif_i", bytes, FontFamily::Name("serif_i".into()));
    }
    // egui panics on a named family that has no fonts, so a machine without a
    // serif font gets the regular proportional one under that name.
    for name in ["serif", "serif_i"] {
        let family = FontFamily::Name(name.into());
        if !defs.families.contains_key(&family) {
            let fallback = defs.families[&FontFamily::Proportional].clone();
            defs.families.insert(family, fallback);
        }
    }
    ctx.set_fonts(defs);
}

pub fn sans(size: f32) -> FontId {
    FontId::new(size, FontFamily::Proportional)
}

pub fn serif(size: f32) -> FontId {
    FontId::new(size, FontFamily::Name("serif".into()))
}

pub fn serif_italic(size: f32) -> FontId {
    FontId::new(size, FontFamily::Name("serif_i".into()))
}

pub fn apply_style(ctx: &egui::Context, pal: Palette) {
    let mut style = (*ctx.style()).clone();
    style.visuals.dark_mode = pal.dark;
    style.visuals.override_text_color = Some(pal.text());
    style.visuals.panel_fill = pal.bg();
    style.visuals.window_fill = pal.card();
    style.visuals.extreme_bg_color = pal.card_alt();
    style.visuals.faint_bg_color = pal.card_alt();
    style.visuals.widgets.noninteractive.bg_stroke = egui::Stroke::new(1.0, pal.border());
    style.visuals.widgets.inactive.weak_bg_fill = pal.card();
    style.visuals.widgets.inactive.bg_stroke = egui::Stroke::new(1.0, pal.border());
    style.visuals.widgets.hovered.weak_bg_fill = pal.hover();
    style.visuals.widgets.hovered.bg_stroke = egui::Stroke::new(1.0, pal.border_strong());
    style.visuals.selection.bg_fill = pal.text_selection();
    style.visuals.selection.stroke = egui::Stroke::new(1.0, pal.accent());
    style.visuals.text_cursor.stroke = egui::Stroke::new(1.5, pal.text());
    style.spacing.item_spacing = egui::vec2(8.0, 8.0);
    ctx.set_style(style);
}

#[cfg(test)]
mod selection_tests {
    use super::*;

    fn luminance(c: Color32) -> f32 {
        let f = |v: u8| {
            let v = v as f32 / 255.0;
            if v <= 0.03928 { v / 12.92 } else { ((v + 0.055) / 1.055).powf(2.4) }
        };
        0.2126 * f(c.r()) + 0.7152 * f(c.g()) + 0.0722 * f(c.b())
    }

    fn contrast(a: Color32, b: Color32) -> f32 {
        let (la, lb) = (luminance(a), luminance(b));
        (la.max(lb) + 0.05) / (la.min(lb) + 0.05)
    }

    #[test]
    fn selected_text_stays_readable_in_both_themes() {
        for dark in [false, true] {
            let pal = Palette { dark };
            let bg = pal.text_selection();
            assert!(contrast(pal.text(), bg) >= 4.5, "text, dark={dark}");
            assert!(contrast(pal.text_muted(), bg) >= 3.0, "muted, dark={dark}");
            assert!(contrast(pal.text_faint(), bg) >= 2.4, "faint, dark={dark}");
        }
    }
}
