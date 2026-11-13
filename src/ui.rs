use crate::theme::*;
use eframe::egui::{
    self, Align2, Color32, CornerRadius, FontFamily, FontId, Pos2, Rect, Sense, Stroke, Vec2,
};

pub fn text(ui: &egui::Ui, pos: Pos2, s: &str, font: FontId, color: Color32) -> Rect {
    let galley = ui.fonts(|f| f.layout_no_wrap(s.to_owned(), font.clone(), color));
    let rect = Rect::from_min_size(pos, galley.size());
    ui.painter().galley(pos, galley, color);
    rect
}

pub fn text_center(ui: &egui::Ui, center: Pos2, s: &str, font: FontId, color: Color32) -> Rect {
    let galley = ui.fonts(|f| f.layout_no_wrap(s.to_owned(), font, color));
    let rect = Rect::from_center_size(center, galley.size());
    ui.painter().galley(rect.min, galley, color);
    rect
}

pub fn text_right(ui: &egui::Ui, right: Pos2, s: &str, font: FontId, color: Color32) -> Rect {
    let galley = ui.fonts(|f| f.layout_no_wrap(s.to_owned(), font, color));
    let rect = Rect::from_min_size(Pos2::new(right.x - galley.size().x, right.y), galley.size());
    ui.painter().galley(rect.min, galley, color);
    rect
}

pub fn wrapped(ui: &egui::Ui, rect: Rect, s: &str, font: FontId, color: Color32) -> f32 {
    let galley = ui.fonts(|f| f.layout(s.to_owned(), font, color, rect.width()));
    let h = galley.size().y;
    ui.painter().galley(rect.min, galley, color);
    h
}

pub fn click(ui: &egui::Ui, rect: Rect, key: &str) -> bool {
    ui.interact(rect, ui.id().with(key), Sense::click())
        .clicked()
}

pub fn hovered(ui: &egui::Ui, rect: Rect, key: &str) -> bool {
    ui.interact(rect, ui.id().with(key).with("hover-state"), Sense::hover())
        .hovered()
}

pub fn round_rect(p: &egui::Painter, rect: Rect, radius: u8, fill: Color32) {
    p.rect_filled(rect, CornerRadius::same(radius), fill);
}

pub fn bordered(p: &egui::Painter, rect: Rect, radius: u8, fill: Color32, stroke: Color32) {
    p.rect(
        rect,
        CornerRadius::same(radius),
        fill,
        Stroke::new(1.0, stroke),
        egui::StrokeKind::Inside,
    );
}

pub fn shadow(p: &egui::Painter, rect: Rect, radius: u8) {
    for i in 0..5 {
        let a = 5 - i;
        let r = rect.expand(i as f32 * 2.4).translate(Vec2::new(0.0, 2.0));
        p.rect_filled(
            r,
            CornerRadius::same(radius + i as u8),
            Color32::from_black_alpha((a * 3) as u8),
        );
    }
}

pub fn primary_button(ui: &egui::Ui, pal: Palette, rect: Rect, label: &str, key: &str) -> bool {
    let hov = hovered(ui, rect, key);
    let fill = if hov {
        if pal.dark {
            Color32::from_rgb(0xe9, 0xe8, 0xe1)
        } else {
            Color32::from_rgb(0x30, 0x30, 0x2f)
        }
    } else {
        pal.accent()
    };
    round_rect(ui.painter(), rect, 10, fill);
    text_center(
        ui,
        rect.center(),
        label,
        sans(14.0),
        if pal.dark { VAST_950 } else { Color32::WHITE },
    );
    click(ui, rect, key)
}

pub fn secondary_button(ui: &egui::Ui, pal: Palette, rect: Rect, label: &str, key: &str) -> bool {
    let hov = hovered(ui, rect, key);
    bordered(
        ui.painter(),
        rect,
        10,
        if hov { pal.hover() } else { pal.card() },
        pal.border(),
    );
    text_center(ui, rect.center(), label, sans(13.5), pal.text());
    click(ui, rect, key)
}

pub fn toc_button(
    ui: &egui::Ui,
    pal: Palette,
    rect: Rect,
    label: &str,
    on: bool,
    key: &str,
) -> bool {
    let fill = if on {
        pal.accent()
    } else if hovered(ui, rect, key) {
        pal.hover()
    } else {
        Color32::TRANSPARENT
    };
    round_rect(ui.painter(), rect, 7, fill);
    text_center(
        ui,
        rect.center(),
        label,
        sans(12.5),
        if on {
            if pal.dark {
                VAST_950
            } else {
                Color32::WHITE
            }
        } else {
            pal.text_muted()
        },
    );
    click(ui, rect, key)
}

pub fn toggle(ui: &egui::Ui, pal: Palette, rect: Rect, on: bool, key: &str) -> bool {
    let bg = if on {
        pal.accent()
    } else {
        if pal.dark {
            VAST_800
        } else {
            SAND_700
        }
    };
    round_rect(ui.painter(), rect, (rect.height() / 2.0) as u8, bg);
    let knob_x = if on {
        rect.right() - rect.height() / 2.0
    } else {
        rect.left() + rect.height() / 2.0
    };
    ui.painter().circle_filled(
        Pos2::new(knob_x, rect.center().y),
        rect.height() / 2.0 - 3.0,
        if pal.dark { VAST_950 } else { Color32::WHITE },
    );
    click(ui, rect, key)
}

pub fn search_field(
    ui: &mut egui::Ui,
    pal: Palette,
    rect: Rect,
    value: &mut String,
    hint: &str,
    key: &str,
) {
    bordered(ui.painter(), rect, 9, pal.card(), pal.border());
    let ic = Rect::from_center_size(
        Pos2::new(rect.left() + 20.0, rect.center().y),
        Vec2::splat(14.0),
    );
    icon(ui.painter(), "search", ic, pal.text_faint(), 1.4);
    let inner = Rect::from_min_max(
        Pos2::new(rect.left() + 34.0, rect.top() + 6.0),
        Pos2::new(rect.right() - 8.0, rect.bottom() - 6.0),
    );
    ui.put(
        inner,
        egui::TextEdit::singleline(value)
            .hint_text(hint)
            .font(sans(13.0))
            .text_color(pal.text())
            .frame(false)
            .margin(Vec2::new(0.0, 2.0))
            .id(egui::Id::new(key)),
    );
}

pub fn icon(p: &egui::Painter, name: &str, rect: Rect, color: Color32, w: f32) {
    let s = Stroke::new(w, color);
    let c = rect.center();
    let r = rect.width().min(rect.height()) / 2.0;
    match name {
        "home" => {
            p.add(egui::Shape::line(
                vec![
                    Pos2::new(c.x - r, c.y - r * 0.05),
                    Pos2::new(c.x, c.y - r * 0.85),
                    Pos2::new(c.x + r, c.y - r * 0.05),
                ],
                s,
            ));
            p.add(egui::Shape::line(
                vec![
                    Pos2::new(c.x - r * 0.68, c.y),
                    Pos2::new(c.x - r * 0.68, c.y + r * 0.8),
                    Pos2::new(c.x + r * 0.68, c.y + r * 0.8),
                    Pos2::new(c.x + r * 0.68, c.y),
                ],
                s,
            ));
        }
        "book" => {
            let rr = Rect::from_center_size(c, Vec2::new(r * 1.6, r * 1.75));
            p.rect_stroke(rr, CornerRadius::same(2), s, egui::StrokeKind::Middle);
            p.line_segment(
                [
                    Pos2::new(c.x - r * 0.2, rr.top()),
                    Pos2::new(c.x - r * 0.2, rr.bottom()),
                ],
                s,
            );
        }
        "snippet" => {
            p.add(egui::Shape::line(
                vec![
                    Pos2::new(c.x - r, c.y - r * 0.55),
                    Pos2::new(c.x, c.y - r * 0.95),
                    Pos2::new(c.x + r, c.y - r * 0.55),
                ],
                s,
            ));
            p.add(egui::Shape::line(
                vec![
                    Pos2::new(c.x - r, c.y + r * 0.05),
                    Pos2::new(c.x, c.y + r * 0.45),
                    Pos2::new(c.x + r, c.y + r * 0.05),
                ],
                s,
            ));
            p.add(egui::Shape::line(
                vec![
                    Pos2::new(c.x - r, c.y + r * 0.55),
                    Pos2::new(c.x, c.y + r * 0.95),
                    Pos2::new(c.x + r, c.y + r * 0.55),
                ],
                s,
            ));
        }
        "style" => {
            let pts = [
                (c.x, c.y - r),
                (c.x + r * 0.28, c.y - r * 0.28),
                (c.x + r, c.y),
                (c.x + r * 0.28, c.y + r * 0.28),
                (c.x, c.y + r),
                (c.x - r * 0.28, c.y + r * 0.28),
                (c.x - r, c.y),
                (c.x - r * 0.28, c.y - r * 0.28),
            ];
            p.add(egui::Shape::closed_line(
                pts.iter().map(|(x, y)| Pos2::new(*x, *y)).collect(),
                s,
            ));
        }
        "pad" => {
            let rr = Rect::from_center_size(c, Vec2::new(r * 1.55, r * 1.8));
            p.rect_stroke(rr, CornerRadius::same(2), s, egui::StrokeKind::Middle);
            for i in 0..3 {
                let y = rr.top() + 4.0 + i as f32 * 4.5;
                p.line_segment(
                    [
                        Pos2::new(rr.left() + 3.0, y),
                        Pos2::new(rr.right() - 3.0, y),
                    ],
                    Stroke::new(1.0, color),
                );
            }
        }
        "chart" => {
            p.line_segment(
                [Pos2::new(c.x - r, c.y + r), Pos2::new(c.x - r, c.y - r)],
                s,
            );
            p.line_segment(
                [Pos2::new(c.x - r, c.y + r), Pos2::new(c.x + r, c.y + r)],
                s,
            );
            for (i, h) in [0.25f32, 0.6, 0.4].iter().enumerate() {
                let x = c.x - r * 0.5 + i as f32 * r * 0.55;
                p.line_segment(
                    [
                        Pos2::new(x, c.y + r * 0.6),
                        Pos2::new(x, c.y + r * 0.6 - r * 1.1 * h),
                    ],
                    Stroke::new(2.0, color),
                );
            }
        }
        "gear" => {
            p.circle_stroke(c, r * 0.55, s);
            for i in 0..8 {
                let a = i as f32 * std::f32::consts::TAU / 8.0;
                p.line_segment(
                    [
                        Pos2::new(c.x + a.cos() * r * 0.72, c.y + a.sin() * r * 0.72),
                        Pos2::new(c.x + a.cos() * r, c.y + a.sin() * r),
                    ],
                    s,
                );
            }
        }
        "search" => {
            p.circle_stroke(Pos2::new(c.x - r * 0.12, c.y - r * 0.12), r * 0.6, s);
            p.line_segment(
                [
                    Pos2::new(c.x + r * 0.32, c.y + r * 0.32),
                    Pos2::new(c.x + r * 0.85, c.y + r * 0.85),
                ],
                s,
            );
        }
        "plus" => {
            p.line_segment(
                [
                    Pos2::new(c.x - r * 0.75, c.y),
                    Pos2::new(c.x + r * 0.75, c.y),
                ],
                s,
            );
            p.line_segment(
                [
                    Pos2::new(c.x, c.y - r * 0.75),
                    Pos2::new(c.x, c.y + r * 0.75),
                ],
                s,
            );
        }
        "star" => {
            let mut pts = Vec::new();
            for i in 0..10 {
                let ang = -std::f32::consts::FRAC_PI_2 + i as f32 * std::f32::consts::PI / 5.0;
                let rad = if i % 2 == 0 { r } else { r * 0.45 };
                pts.push(Pos2::new(c.x + ang.cos() * rad, c.y + ang.sin() * rad));
            }
            p.add(egui::Shape::closed_line(pts, s));
        }
        "trash" => {
            p.line_segment(
                [
                    Pos2::new(c.x - r * 0.8, c.y - r * 0.5),
                    Pos2::new(c.x + r * 0.8, c.y - r * 0.5),
                ],
                s,
            );
            p.line_segment(
                [
                    Pos2::new(c.x - r * 0.4, c.y - r * 0.8),
                    Pos2::new(c.x + r * 0.4, c.y - r * 0.8),
                ],
                s,
            );
            p.add(egui::Shape::line(
                vec![
                    Pos2::new(c.x - r * 0.6, c.y - r * 0.5),
                    Pos2::new(c.x - r * 0.45, c.y + r * 0.8),
                    Pos2::new(c.x + r * 0.45, c.y + r * 0.8),
                    Pos2::new(c.x + r * 0.6, c.y - r * 0.5),
                ],
                s,
            ));
        }
        "edit" => {
            p.line_segment(
                [
                    Pos2::new(c.x - r * 0.7, c.y + r * 0.7),
                    Pos2::new(c.x + r * 0.6, c.y - r * 0.6),
                ],
                s,
            );
            p.line_segment(
                [
                    Pos2::new(c.x - r * 0.75, c.y + r * 0.75),
                    Pos2::new(c.x - r * 0.35, c.y + r * 0.7),
                ],
                s,
            );
            p.line_segment(
                [
                    Pos2::new(c.x - r * 0.75, c.y + r * 0.75),
                    Pos2::new(c.x - r * 0.7, c.y + r * 0.35),
                ],
                s,
            );
        }
        "copy" => {
            let a = Rect::from_min_size(
                Pos2::new(c.x - r * 0.75, c.y - r * 0.75),
                Vec2::new(r * 1.1, r * 1.1),
            );
            let b = Rect::from_min_size(
                Pos2::new(c.x - r * 0.3, c.y - r * 0.3),
                Vec2::new(r * 1.1, r * 1.1),
            );
            p.rect_stroke(
                a,
                CornerRadius::same(2),
                Stroke::new(1.0, color),
                egui::StrokeKind::Middle,
            );
            p.rect_stroke(
                b,
                CornerRadius::same(2),
                Stroke::new(1.0, color),
                egui::StrokeKind::Middle,
            );
        }
        "check" => {
            p.add(egui::Shape::line(
                vec![
                    Pos2::new(c.x - r * 0.7, c.y),
                    Pos2::new(c.x - r * 0.15, c.y + r * 0.55),
                    Pos2::new(c.x + r * 0.75, c.y - r * 0.55),
                ],
                Stroke::new(w.max(1.4), color),
            ));
        }
        "close" => {
            p.line_segment(
                [
                    Pos2::new(c.x - r * 0.6, c.y - r * 0.6),
                    Pos2::new(c.x + r * 0.6, c.y + r * 0.6),
                ],
                s,
            );
            p.line_segment(
                [
                    Pos2::new(c.x + r * 0.6, c.y - r * 0.6),
                    Pos2::new(c.x - r * 0.6, c.y + r * 0.6),
                ],
                s,
            );
        }
        "arrow" => {
            p.line_segment(
                [
                    Pos2::new(c.x - r * 0.6, c.y + r * 0.6),
                    Pos2::new(c.x + r * 0.6, c.y - r * 0.6),
                ],
                s,
            );
            p.line_segment(
                [
                    Pos2::new(c.x - r * 0.1, c.y - r * 0.6),
                    Pos2::new(c.x + r * 0.6, c.y - r * 0.6),
                ],
                s,
            );
            p.line_segment(
                [
                    Pos2::new(c.x + r * 0.6, c.y - r * 0.6),
                    Pos2::new(c.x + r * 0.6, c.y + r * 0.1),
                ],
                s,
            );
        }
        "mic" => {
            let body = Rect::from_center_size(
                Pos2::new(c.x, c.y - r * 0.15),
                Vec2::new(r * 0.62, r * 1.1),
            );
            p.rect_stroke(
                body,
                CornerRadius::same((r * 0.31) as u8),
                s,
                egui::StrokeKind::Middle,
            );
            p.add(egui::Shape::line(
                vec![
                    Pos2::new(c.x - r * 0.55, c.y - r * 0.1),
                    Pos2::new(c.x - r * 0.55, c.y + r * 0.25),
                    Pos2::new(c.x - r * 0.2, c.y + r * 0.65),
                    Pos2::new(c.x + r * 0.2, c.y + r * 0.65),
                    Pos2::new(c.x + r * 0.55, c.y + r * 0.25),
                    Pos2::new(c.x + r * 0.55, c.y - r * 0.1),
                ],
                s,
            ));
            p.line_segment([Pos2::new(c.x, c.y + r * 0.65), Pos2::new(c.x, c.y + r)], s);
        }
        "help" => {
            p.circle_stroke(c, r * 0.9, s);
            p.text(
                c,
                Align2::CENTER_CENTER,
                "?",
                FontId::proportional(r * 1.15),
                color,
            );
        }
        "keyboard" => {
            let rr = Rect::from_center_size(c, Vec2::new(r * 1.8, r * 1.2));
            p.rect_stroke(rr, CornerRadius::same(3), s, egui::StrokeKind::Middle);
            for i in 0..3 {
                let x = rr.left() + 4.0 + i as f32 * 5.0;
                p.line_segment(
                    [
                        Pos2::new(x, rr.center().y - 2.0),
                        Pos2::new(x + 1.0, rr.center().y - 2.0),
                    ],
                    Stroke::new(1.4, color),
                );
            }
        }
        "wave" => {
            for i in 0..7 {
                let x = c.x - r + i as f32 * (r * 2.0 / 6.0);
                let h = r * (0.25 + 0.75 * ((i as f32 * 0.9).sin().abs()));
                p.line_segment(
                    [Pos2::new(x, c.y - h), Pos2::new(x, c.y + h)],
                    Stroke::new(w.max(1.6), color),
                );
            }
        }
        "more" => {
            for i in 0..3 {
                p.circle_filled(
                    Pos2::new(c.x - r * 0.6 + i as f32 * r * 0.6, c.y),
                    1.1,
                    color,
                );
            }
        }
        _ => {
            p.circle_stroke(c, r * 0.6, s);
        }
    }
}

pub fn status_pill(ui: &egui::Ui, pal: Palette, rect: Rect, label: &str) {
    round_rect(
        ui.painter(),
        rect,
        (rect.height() / 2.0) as u8,
        if pal.dark { VAST_800 } else { SAND_600 },
    );
    text_center(ui, rect.center(), label, sans(10.5), pal.text_muted());
}

pub fn waveform(ui: &egui::Ui, rect: Rect, color: Color32, time: f32, amp: f32) {
    let n = 7;
    let gap = rect.width() / n as f32;
    for i in 0..n {
        let x = rect.left() + gap * (i as f32 + 0.5);
        let phase = time * 5.0 + i as f32 * 0.9;
        let v = (phase.sin() * 0.5 + 0.5) * 0.7 + 0.3;
        let h = (rect.height() * 0.5) * v * amp;
        ui.painter().line_segment(
            [
                Pos2::new(x, rect.center().y - h),
                Pos2::new(x, rect.center().y + h),
            ],
            Stroke::new(gap * 0.32, color),
        );
    }
}

pub fn font_family_serif() -> FontFamily {
    FontFamily::Name("serif".into())
}

pub fn center_rect(outer: Rect, size: Vec2) -> Rect {
    Rect::from_center_size(outer.center(), size)
}

pub fn alpha(c: Color32, a: u8) -> Color32 {
    Color32::from_rgba_unmultiplied(c.r(), c.g(), c.b(), a)
}

pub fn lerp_color(a: Color32, b: Color32, t: f32) -> Color32 {
    let t = t.clamp(0.0, 1.0);
    let f = |x: u8, y: u8| (x as f32 + (y as f32 - x as f32) * t) as u8;
    Color32::from_rgb(f(a.r(), b.r()), f(a.g(), b.g()), f(a.b(), b.b()))
}

pub fn label_tag(ui: &egui::Ui, pal: Palette, pos: Pos2, label: &str) -> Rect {
    let galley = ui.fonts(|f| f.layout_no_wrap(label.to_owned(), sans(10.5), pal.text_muted()));
    let rect = Rect::from_min_size(pos, galley.size() + Vec2::new(10.0, 4.0));
    let _ = galley;
    round_rect(
        ui.painter(),
        rect,
        5,
        if pal.dark { VAST_800 } else { SAND_600 },
    );
    text_center(ui, rect.center(), label, sans(10.5), pal.text_muted());
    rect
}

/// Seven bars fed by the microphone's live frequency bands.
pub fn spectrum(ui: &egui::Ui, rect: Rect, color: Color32, bands: &[f32; 7]) {
    let gap = rect.width() / 7.0;
    for (i, v) in bands.iter().enumerate() {
        let x = rect.left() + gap * (i as f32 + 0.5);
        let h = 2.0 + (rect.height() * 0.5) * v.clamp(0.0, 1.0);
        let c = if *v > 0.72 { GLOW_300 } else { color };
        ui.painter().line_segment(
            [
                Pos2::new(x, rect.center().y - h),
                Pos2::new(x, rect.center().y + h),
            ],
            Stroke::new(gap * 0.34, c),
        );
    }
}
