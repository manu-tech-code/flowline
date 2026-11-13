use crate::theme::*;
use crate::ui::*;
use crate::{MAX_DICTATION_SECS, WARN_DICTATION_SECS};
use eframe::egui::{self, Color32, CornerRadius, Pos2, Rect, Stroke, Vec2};
use std::time::Instant;

#[derive(Clone)]
pub enum Phase {
    Idle,
    Listening {
        started: Instant,
        hands_free: bool,
    },
    Transcribing {
        started: Instant,
        raw: String,
        secs: f32,
    },
    /// The transcript is in. `deadline: None` keeps the pill on screen until
    /// the person dismisses it or copies the text.
    Done {
        deadline: Option<Instant>,
        text: String,
        pasted: bool,
        copied: bool,
        /// Heading for the card when it is not a plain transcript (command mode).
        head: Option<String>,
    },
}

impl Phase {
    pub fn is_active(&self) -> bool {
        !matches!(self, Phase::Idle)
    }
    pub fn elapsed(&self) -> f32 {
        match self {
            Phase::Idle => 0.0,
            Phase::Listening { started, .. } => started.elapsed().as_secs_f32(),
            Phase::Transcribing { secs, .. } => *secs,
            Phase::Done { .. } => 0.0,
        }
    }
}

#[derive(Clone, Copy)]
pub enum OverlayAction {
    None,
    Stop,
    Cancel,
    Copy,
    Dismiss,
}

/// The pill floats over other apps without moving focus: on screen it is a
/// non-activating NSPanel (see `native_overlay`) that can never become key or
/// main. This egui version draws the same pill and never calls request_focus.
pub fn overlay(
    ui: &egui::Ui,
    _pal: Palette,
    phase: &Phase,
    time: f32,
    bands: &[f32; 7],
    model: &str,
    draggable: bool,
) -> OverlayAction {
    if matches!(phase, Phase::Idle) {
        return OverlayAction::None;
    }
    let screen = ui.max_rect();
    let pill = Rect::from_center_size(
        Pos2::new(screen.center().x, screen.bottom() - 58.0),
        Vec2::new(306.0, 66.0),
    );
    shadow(ui.painter(), pill, 22);
    ui.painter()
        .rect_filled(pill, CornerRadius::same(22), VAST_950);
    ui.painter().rect_stroke(
        pill,
        CornerRadius::same(22),
        Stroke::new(1.0, Color32::from_white_alpha(22)),
        egui::StrokeKind::Inside,
    );

    if draggable {
        let handle = Rect::from_min_max(
            pill.min,
            Pos2::new(pill.right() - 74.0, pill.bottom()),
        );
        let resp = ui.interact(handle, ui.id().with("overlay-drag"), egui::Sense::drag());
        if resp.drag_started() {
            ui.ctx().send_viewport_cmd(egui::ViewportCommand::StartDrag);
        }
    }

    let mut action = OverlayAction::None;
    match phase {
        Phase::Idle => {}
        Phase::Listening {
            started: _,
            hands_free,
        } => {
            let bars = Rect::from_min_size(
                Pos2::new(pill.left() + 18.0, pill.center().y - 16.0),
                Vec2::new(58.0, 32.0),
            );
            // Driven by the microphone: each bar is a real frequency band.
            let n = 9;
            let gap = bars.width() / n as f32;
            for i in 0..n {
                let band = (i * 6) / (n - 1);
                let v = bands[band].clamp(0.0, 1.0);
                let h = 2.0 + 13.0 * v;
                let x = bars.left() + gap * (i as f32 + 0.5);
                let color = if v > 0.72 { GLOW_300 } else { SAND_100 };
                ui.painter().line_segment(
                    [
                        Pos2::new(x, bars.center().y - h),
                        Pos2::new(x, bars.center().y + h),
                    ],
                    Stroke::new(gap * 0.34, color),
                );
            }
            let _ = time;
            let almost = phase.elapsed() >= WARN_DICTATION_SECS;
            let title = if almost {
                "Almost at the limit"
            } else if *hands_free {
                "Listening · hands-free"
            } else {
                "Listening"
            };
            let g = ui.fonts(|f| f.layout_no_wrap(title.to_owned(), sans(13.0), Color32::WHITE));
            ui.painter().galley(
                Pos2::new(pill.left() + 90.0, pill.top() + 15.0),
                g,
                Color32::WHITE,
            );
            let sub = if almost {
                format!(
                    "{} / {} · Esc to cancel",
                    mmss(phase.elapsed()),
                    mmss(MAX_DICTATION_SECS)
                )
            } else {
                format!("{} · Esc to cancel", mmss(phase.elapsed()))
            };
            let g = ui
                .fonts(|f| f.layout_no_wrap(sub, sans(10.5), Color32::from_rgb(0xa3, 0xa3, 0x9e)));
            ui.painter().galley(
                Pos2::new(pill.left() + 90.0, pill.top() + 36.0),
                g,
                Color32::from_rgb(0xa3, 0xa3, 0x9e),
            );

            let stop = Rect::from_center_size(
                Pos2::new(pill.right() - 28.0, pill.center().y),
                Vec2::splat(30.0),
            );
            let hov = hovered(ui, stop, "overlay-stop");
            ui.painter().circle_filled(
                stop.center(),
                14.0,
                if hov {
                    Color32::from_rgb(0x3a, 0x3a, 0x38)
                } else {
                    Color32::from_rgb(0x30, 0x30, 0x2f)
                },
            );
            ui.painter().rect_filled(
                Rect::from_center_size(stop.center(), Vec2::splat(9.0)),
                CornerRadius::same(2),
                SAND_100,
            );
            if click(ui, stop, "overlay-stop") {
                action = OverlayAction::Stop;
            }
        }
        Phase::Transcribing { started, .. } => {
            let dots = Rect::from_min_size(
                Pos2::new(pill.left() + 18.0, pill.center().y - 8.0),
                Vec2::new(58.0, 16.0),
            );
            for i in 0..3 {
                let ph = (time * 3.2 - i as f32 * 0.35).sin() * 0.5 + 0.5;
                let c = lerp_color(Color32::from_rgb(0x71, 0x71, 0x6e), SAND_100, ph);
                ui.painter().circle_filled(
                    Pos2::new(dots.left() + 10.0 + i as f32 * 16.0, dots.center().y),
                    3.6,
                    c,
                );
            }
            let g = ui
                .fonts(|f| f.layout_no_wrap("Transcribing".to_owned(), sans(13.0), Color32::WHITE));
            ui.painter().galley(
                Pos2::new(pill.left() + 90.0, pill.top() + 15.0),
                g,
                Color32::WHITE,
            );
            let sub = format!("{model} · {:.1} s audio", phase.elapsed());
            let g = ui.fonts(|f| {
                f.layout_no_wrap(
                    sub.to_owned(),
                    sans(10.5),
                    Color32::from_rgb(0xa3, 0xa3, 0x9e),
                )
            });
            ui.painter().galley(
                Pos2::new(pill.left() + 90.0, pill.top() + 36.0),
                g,
                Color32::from_rgb(0xa3, 0xa3, 0x9e),
            );
            let _ = started;
        }
        Phase::Done {
            deadline,
            text,
            pasted,
            copied,
            head: custom_head,
        } => {
            if !*pasted {
                // No text field had focus: keep the transcript here with a
                // Copy button until dismissed.
                let card = Rect::from_center_size(
                    Pos2::new(screen.center().x, screen.bottom() - 74.0),
                    Vec2::new(436.0, 104.0),
                );
                shadow(ui.painter(), card, 22);
                ui.painter().rect_filled(card, CornerRadius::same(18), VAST_950);
                ui.painter().rect_stroke(
                    card,
                    CornerRadius::same(18),
                    Stroke::new(1.0, Color32::from_white_alpha(22)),
                    egui::StrokeKind::Inside,
                );
                let head = if let Some(h) = custom_head {
                    h.as_str()
                } else if *copied {
                    "No text field focused · copied"
                } else {
                    "No text field focused"
                };
                ui.painter().text(
                    Pos2::new(card.left() + 18.0, card.top() + 14.0),
                    egui::Align2::LEFT_TOP,
                    head,
                    sans(11.5),
                    Color32::from_rgb(0xa3, 0xa3, 0x9e),
                );
                let preview: String = if text.chars().count() > 96 {
                    format!("{}…", text.chars().take(95).collect::<String>())
                } else {
                    text.clone()
                };
                let galley = ui.fonts(|f| {
                    f.layout(
                        preview,
                        sans(12.0),
                        Color32::WHITE,
                        card.width() - 150.0,
                    )
                });
                ui.painter().galley(
                    Pos2::new(card.left() + 18.0, card.top() + 36.0),
                    galley,
                    Color32::WHITE,
                );
                let copy = Rect::from_min_size(
                    Pos2::new(card.right() - 108.0, card.bottom() - 46.0),
                    Vec2::new(84.0, 30.0),
                );
                let hot = hovered(ui, copy, "overlay-copy");
                ui.painter().rect_filled(
                    copy,
                    CornerRadius::same(8),
                    if hot {
                        Color32::from_rgb(0x3a, 0x3a, 0x38)
                    } else {
                        Color32::from_rgb(0x30, 0x30, 0x2f)
                    },
                );
                ui.painter().text(
                    copy.center(),
                    egui::Align2::CENTER_CENTER,
                    if *copied { "Copied" } else { "Copy" },
                    sans(11.0),
                    SAND_100,
                );
                if click(ui, copy, "overlay-copy") {
                    action = OverlayAction::Copy;
                }
                let close = Rect::from_center_size(
                    Pos2::new(card.right() - 16.0, card.top() + 16.0),
                    Vec2::splat(20.0),
                );
                ui.painter().text(
                    close.center(),
                    egui::Align2::CENTER_CENTER,
                    "×",
                    sans(15.0),
                    Color32::from_rgb(0x71, 0x71, 0x6e),
                );
                if click(ui, close, "overlay-dismiss") {
                    action = OverlayAction::Dismiss;
                }
                if ui.input(|i| i.key_pressed(egui::Key::Escape)) {
                    action = OverlayAction::Dismiss;
                }
                let _ = deadline;
            } else {
                let c = Pos2::new(pill.left() + 47.0, pill.center().y);
                ui.painter()
                    .circle_stroke(c, 13.0, Stroke::new(1.3, SAND_100));
                ui.painter().line_segment(
                    [c + Vec2::new(-5.5, 0.5), c + Vec2::new(-1.5, 4.5)],
                    Stroke::new(1.8, SAND_100),
                );
                ui.painter().line_segment(
                    [c + Vec2::new(-1.5, 4.5), c + Vec2::new(6.0, -5.0)],
                    Stroke::new(1.8, SAND_100),
                );
                let g = ui.fonts(|f| {
                    f.layout_no_wrap("Pasted".to_owned(), sans(13.0), Color32::WHITE)
                });
                ui.painter().galley(
                    Pos2::new(pill.left() + 90.0, pill.top() + 15.0),
                    g,
                    Color32::WHITE,
                );
                let g = ui.fonts(|f| {
                    f.layout_no_wrap(
                        format!("{model} · {:.1} s audio", phase.elapsed()),
                        sans(10.5),
                        Color32::from_rgb(0xa3, 0xa3, 0x9e),
                    )
                });
                ui.painter().galley(
                    Pos2::new(pill.left() + 90.0, pill.top() + 36.0),
                    g,
                    Color32::from_rgb(0xa3, 0xa3, 0x9e),
                );
                if ui.input(|i| i.key_pressed(egui::Key::Escape)) {
                    action = OverlayAction::Dismiss;
                }
                if deadline.is_none() && click(ui, pill, "overlay-dismiss") {
                    action = OverlayAction::Dismiss;
                }
            }
        }
    }
    action
}

pub fn mmss(secs: f32) -> String {
    let s = secs.max(0.0) as u32;
    format!("{}:{:02}", s / 60, s % 60)
}

pub fn toast(ui: &egui::Ui, pal: Palette, text: &str, alpha: f32) {
    let screen = ui.max_rect();
    let g = ui.fonts(|f| f.layout_no_wrap(text.to_owned(), sans(12.5), Color32::WHITE));
    let rect = Rect::from_center_size(
        Pos2::new(screen.center().x, screen.top() + 72.0),
        Vec2::new(g.size().x + 28.0, 34.0),
    );
    let a = (alpha * 235.0) as u8;
    ui.painter().rect_filled(
        rect,
        CornerRadius::same(17),
        Color32::from_rgba_unmultiplied(0x1a, 0x1a, 0x1a, a),
    );
    ui.painter().galley(
        rect.min + Vec2::new(14.0, 9.0),
        g,
        Color32::from_rgba_unmultiplied(255, 255, 255, a),
    );
    let _ = pal;
}
