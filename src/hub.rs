use crate::dictation::mmss;
use crate::model::*;
use crate::theme::*;
use crate::ui::*;
use crate::{App, Modal, Page, SettingsTab};
use eframe::egui::{self, Color32, CornerRadius, Pos2, Rect, Stroke, Vec2};

const SIDEBAR: f32 = 236.0;
const PAD: f32 = 40.0;

pub fn show(ui: &mut egui::Ui, app: &mut App, pal: Palette, time: f32) {
    let full = ui.max_rect();
    sidebar(ui, app, pal, full);
    let content = Rect::from_min_max(Pos2::new(full.left() + SIDEBAR, full.top()), full.max);
    ui.painter().rect_filled(content, 0.0, pal.canvas());

    match app.page {
        Page::Home => home(ui, app, pal, content, time),
        Page::Dictionary => dictionary(ui, app, pal, content),
        Page::Snippets => snippets(ui, app, pal, content),
        Page::Style => style_page(ui, app, pal, content),
        Page::Transforms => crate::pages::transforms(ui, app, pal, content),
        Page::Notetaker => crate::pages::notetaker(ui, app, pal, content),
        Page::Scratchpad => scratchpad(ui, app, pal, content),
        Page::Insights => insights(ui, app, pal, content),
        Page::MyVoice => crate::pages::my_voice(ui, app, pal, content),
        Page::Settings => settings(ui, app, pal, content),
    }
    modal(ui, app, pal, full);
}

fn sidebar(ui: &egui::Ui, app: &mut App, pal: Palette, full: Rect) {
    let p = ui.painter().clone();
    let side = Rect::from_min_max(full.min, Pos2::new(full.left() + SIDEBAR, full.bottom()));
    p.rect_filled(side, 0.0, pal.sidebar());
    p.line_segment(
        [
            Pos2::new(side.right() - 0.5, side.top()),
            Pos2::new(side.right() - 0.5, side.bottom()),
        ],
        Stroke::new(1.0, pal.border()),
    );

    let mark = Rect::from_center_size(
        Pos2::new(side.left() + 32.0, side.top() + 38.0),
        Vec2::new(22.0, 18.0),
    );
    brand_mark(&p, mark, pal.text());
    let g = ui.fonts(|f| f.layout_no_wrap("Flowline".to_owned(), sans(16.0), pal.text()));
    p.galley(
        Pos2::new(mark.right() + 8.0, side.top() + 28.0),
        g,
        pal.text(),
    );

    // account
    let acc = Rect::from_min_size(
        Pos2::new(side.left() + 12.0, side.top() + 66.0),
        Vec2::new(side.width() - 24.0, 46.0),
    );
    p.rect_filled(
        acc,
        CornerRadius::same(10),
        if pal.dark {
            Color32::from_rgb(0x26, 0x26, 0x25)
        } else {
            SAND_200
        },
    );
    p.circle_filled(
        Pos2::new(acc.left() + 23.0, acc.center().y),
        15.0,
        if pal.dark { VAST_800 } else { SAND_700 },
    );
    p.text(
        Pos2::new(acc.left() + 23.0, acc.center().y),
        egui::Align2::CENTER_CENTER,
        "L",
        sans(12.0),
        if pal.dark { SAND_100 } else { Color32::WHITE },
    );
    let g = ui.fonts(|f| f.layout_no_wrap("Local".to_owned(), sans(12.5), pal.text()));
    p.galley(Pos2::new(acc.left() + 46.0, acc.top() + 9.0), g, pal.text());
    let g = ui.fonts(|f| f.layout_no_wrap("On this Mac".to_owned(), sans(10.0), pal.text_faint()));
    p.galley(
        Pos2::new(acc.left() + 46.0, acc.top() + 26.0),
        g,
        pal.text_faint(),
    );

    let items: [(Page, &str, &str); 10] = [
        (Page::Home, "Home", "home"),
        (Page::Dictionary, "Dictionary", "book"),
        (Page::Snippets, "Snippets", "snippet"),
        (Page::Style, "Style", "style"),
        (Page::Transforms, "Transforms", "wave"),
        (Page::Scratchpad, "Scratchpad", "pad"),
        (Page::Notetaker, "Notetaker", "mic"),
        (Page::Insights, "Insights", "chart"),
        (Page::MyVoice, "My Voice", "star"),
        (Page::Settings, "Settings", "gear"),
    ];
    let mut y = side.top() + 130.0;
    for (idx, (page, label, icon_name)) in items.iter().enumerate() {
        if *page == Page::Settings {
            y += 14.0;
        }
        let row = Rect::from_min_size(
            Pos2::new(side.left() + 12.0, y),
            Vec2::new(side.width() - 24.0, 34.0),
        );
        let active = app.page == *page;
        let hov = hovered(ui, row, &format!("nav-{idx}"));
        if active {
            p.rect_filled(row, CornerRadius::same(8), pal.selection());
        } else if hov {
            p.rect_filled(row, CornerRadius::same(8), pal.hover());
        }
        let ic = Rect::from_center_size(
            Pos2::new(row.left() + 18.0, row.center().y),
            Vec2::splat(15.0),
        );
        icon(
            &p,
            icon_name,
            ic,
            if active { pal.text() } else { pal.text_muted() },
            1.4,
        );
        p.text(
            Pos2::new(row.left() + 34.0, row.center().y),
            egui::Align2::LEFT_CENTER,
            *label,
            sans(13.0),
            if active { pal.text() } else { pal.text_muted() },
        );
        if click(ui, row, &format!("nav-{idx}")) {
            if std::env::var_os("FLOWLINE_DEBUG").is_some() {
                eprintln!("nav click -> {label}");
            }
            app.page = *page;
        }
        y += 36.0;
    }

    // tip card
    let tip = Rect::from_min_size(
        Pos2::new(side.left() + 14.0, y + 18.0),
        Vec2::new(side.width() - 28.0, 86.0),
    );
    p.rect_filled(
        tip,
        CornerRadius::same(10),
        if pal.dark {
            Color32::from_rgb(0x26, 0x26, 0x25)
        } else {
            SAND_200
        },
    );
    p.text(
        Pos2::new(tip.left() + 14.0, tip.top() + 12.0),
        egui::Align2::LEFT_TOP,
        "Private by design",
        sans(11.0),
        pal.text(),
    );
    wrapped(
        ui,
        Rect::from_min_size(
            Pos2::new(tip.left() + 14.0, tip.top() + 32.0),
            Vec2::new(tip.width() - 28.0, 44.0),
        ),
        "Transcripts and notes stay on this Mac. Audio goes only to the engine you choose.",
        sans(9.5),
        pal.text_muted(),
    );

    // bottom help
    let help = Rect::from_min_size(
        Pos2::new(side.left() + 12.0, side.bottom() - 88.0),
        Vec2::new(side.width() - 24.0, 32.0),
    );
    let hov = hovered(ui, help, "side-help");
    if hov {
        p.rect_filled(help, CornerRadius::same(8), pal.hover());
    }
    icon(
        &p,
        "help",
        Rect::from_center_size(
            Pos2::new(help.left() + 18.0, help.center().y),
            Vec2::splat(14.0),
        ),
        pal.text_muted(),
        1.2,
    );
    p.text(
        Pos2::new(help.left() + 34.0, help.center().y),
        egui::Align2::LEFT_CENTER,
        "Help & support",
        sans(12.0),
        pal.text_muted(),
    );
    let _ = click(ui, help, "side-help");

    p.line_segment(
        [
            Pos2::new(side.left() + 16.0, side.bottom() - 46.0),
            Pos2::new(side.right() - 16.0, side.bottom() - 46.0),
        ],
        Stroke::new(1.0, pal.border()),
    );
    let g = ui.fonts(|f| {
        f.layout_no_wrap(
            format!("Flowline {}", env!("CARGO_PKG_VERSION")),
            sans(10.0),
            pal.text_faint(),
        )
    });
    p.galley(
        Pos2::new(side.left() + 18.0, side.bottom() - 28.0),
        g,
        pal.text_faint(),
    );
}

pub(crate) fn header(ui: &egui::Ui, pal: Palette, content: Rect, title: &str, subtitle: &str) -> f32 {
    let x = content.left() + PAD;
    ui.painter().text(
        Pos2::new(x, content.top() + 44.0),
        egui::Align2::LEFT_TOP,
        title,
        sans(26.0),
        pal.text(),
    );
    ui.painter().text(
        Pos2::new(x, content.top() + 82.0),
        egui::Align2::LEFT_TOP,
        subtitle,
        sans(13.0),
        pal.text_muted(),
    );
    content.top() + 128.0
}

fn home(ui: &mut egui::Ui, app: &mut App, pal: Palette, content: Rect, time: f32) {
    let x = content.left() + PAD;
    let right = content.right() - PAD;
    header(
        ui,
        pal,
        content,
        "Home",
        "Your voice, turned into clean text.",
    );

    let p = ui.painter().clone();
    // hero
    let hero = Rect::from_min_max(
        Pos2::new(x, content.top() + 128.0),
        Pos2::new(right, content.top() + 282.0),
    );
    rounded_card(ui, pal, hero, 18);
    p.text(
        Pos2::new(hero.left() + 24.0, hero.top() + 22.0),
        egui::Align2::LEFT_TOP,
        "Ready when you are",
        sans(21.0),
        pal.text(),
    );
    let (badge, badge_color) = engine_badge(app, pal);
    p.circle_filled(
        Pos2::new(hero.left() + 232.0, hero.top() + 34.0),
        3.0,
        badge_color,
    );
    p.text(
        Pos2::new(hero.left() + 242.0, hero.top() + 27.0),
        egui::Align2::LEFT_TOP,
        &badge,
        sans(10.0),
        badge_color,
    );
    // The badge is a shortcut to the engine picker.
    let badge_hit = Rect::from_min_size(
        Pos2::new(hero.left() + 224.0, hero.top() + 22.0),
        Vec2::new(measure(ui, &badge, 10.0) + 34.0, 24.0),
    );
    if hovered(ui, badge_hit, "hero-engine") {
        p.line_segment(
            [
                Pos2::new(badge_hit.left() + 18.0, badge_hit.bottom() - 4.0),
                Pos2::new(badge_hit.right() - 4.0, badge_hit.bottom() - 4.0),
            ],
            Stroke::new(1.0, badge_color),
        );
    }
    if click(ui, badge_hit, "hero-engine") {
        app.page = Page::Settings;
        app.settings_tab = SettingsTab::General;
        app.model_menu = false;
    }
    p.text(
        Pos2::new(hero.left() + 24.0, hero.top() + 54.0),
        egui::Align2::LEFT_TOP,
        "Hold the shortcut and speak. Everything you say lands here, cleaned up.",
        sans(12.5),
        pal.text_muted(),
    );

    let key1 = Rect::from_min_size(
        Pos2::new(hero.left() + 24.0, hero.top() + 84.0),
        Vec2::new(46.0, 26.0),
    );
    key_cap(ui, pal, key1, "Fn");
    p.text(
        Pos2::new(key1.right() + 8.0, key1.center().y),
        egui::Align2::LEFT_CENTER,
        "or",
        sans(11.0),
        pal.text_faint(),
    );
    let key2 = Rect::from_min_size(
        Pos2::new(key1.right() + 30.0, hero.top() + 84.0),
        Vec2::new(86.0, 26.0),
    );
    key_cap(ui, pal, key2, "Ctrl Space");
    p.text(
        Pos2::new(key2.right() + 8.0, key2.center().y),
        egui::Align2::LEFT_CENTER,
        "to dictate",
        sans(11.0),
        pal.text_faint(),
    );

    let start = Rect::from_min_size(
        Pos2::new(hero.left() + 24.0, hero.top() + 122.0),
        Vec2::new(146.0, 36.0),
    );
    if primary_button(ui, pal, start, "Start dictation", "home-start") {
        app.start_dictation();
    }
    let open_note = Rect::from_min_size(
        Pos2::new(start.right() + 10.0, start.top()),
        Vec2::new(136.0, 36.0),
    );
    if secondary_button(ui, pal, open_note, "Open Scratchpad", "home-note") {
        app.page = Page::Scratchpad;
    }
    let import = Rect::from_min_size(
        Pos2::new(open_note.right() + 10.0, start.top()),
        Vec2::new(152.0, 36.0),
    );
    if secondary_button(ui, pal, import, "Transcribe audio…", "home-import") {
        if let Some(path) = pick_wav() {
            app.transcribe_path(&path);
        }
    }

    // hero waveform
    let circle_c = Pos2::new(hero.right() - 92.0, hero.center().y - 5.0);
    p.circle_filled(
        circle_c,
        54.0,
        if pal.dark {
            Color32::from_rgb(0x2a, 0x2a, 0x29)
        } else {
            SAND_100
        },
    );
    p.circle_stroke(circle_c, 54.0, Stroke::new(1.0, pal.border()));
    let listening = matches!(app.dict, crate::dictation::Phase::Listening { .. });
    let wave_rect = Rect::from_center_size(circle_c, Vec2::new(62.0, 40.0));
    let wave_color = if pal.dark { SAND_100 } else { VAST_950 };
    if listening {
        spectrum(ui, wave_rect, wave_color, &app.display_bands());
    } else {
        waveform(ui, wave_rect, wave_color, time, 1.0);
    }
    let status = if listening { "Listening" } else { "Ready" };
    let g = ui.fonts(|f| f.layout_no_wrap(status.to_owned(), sans(10.0), pal.text_faint()));
    p.galley(
        Pos2::new(circle_c.x - g.size().x / 2.0, circle_c.y + 62.0),
        g,
        pal.text_faint(),
    );

    // stats: all from the real history, never placeholder numbers
    let stats = week_stats(&app.state.history);
    let streak = day_streak(&app.state.history);
    let cards = [
        (
            "WORDS DICTATED",
            format!("{}", stats.words),
            "last 7 days".to_string(),
        ),
        (
            "TIME SAVED",
            format!("{} min", stats.minutes_saved()),
            "vs. typing".to_string(),
        ),
        (
            "DAY STREAK",
            format!("{}", streak),
            if streak == 0 { "speak to start" } else { "keep it going" }.to_string(),
        ),
        (
            "AVG. SPEED",
            format!("{} wpm", stats.wpm()),
            "last 7 days".to_string(),
        ),
    ];
    let gap = 12.0;
    let cw = (right - x - gap * 3.0) / 4.0;
    for (i, (label, value, note)) in cards.iter().enumerate() {
        let card = Rect::from_min_size(
            Pos2::new(x + i as f32 * (cw + gap), hero.bottom() + 16.0),
            Vec2::new(cw, 92.0),
        );
        rounded_card(ui, pal, card, 14);
        p.text(
            Pos2::new(card.left() + 16.0, card.top() + 14.0),
            egui::Align2::LEFT_TOP,
            *label,
            sans(9.5),
            pal.text_faint(),
        );
        p.text(
            Pos2::new(card.left() + 16.0, card.top() + 34.0),
            egui::Align2::LEFT_TOP,
            value,
            sans(24.0),
            pal.text(),
        );
        p.text(
            Pos2::new(card.left() + 16.0, card.top() + 68.0),
            egui::Align2::LEFT_TOP,
            note,
            sans(10.0),
            pal.text_muted(),
        );
    }

    // dictations list + right rail
    let list_w = (right - x) * 0.655;
    let list = Rect::from_min_max(
        Pos2::new(x, hero.bottom() + 124.0),
        Pos2::new(x + list_w, content.bottom() - 24.0),
    );
    rounded_card(ui, pal, list, 16);
    p.text(
        Pos2::new(list.left() + 20.0, list.top() + 18.0),
        egui::Align2::LEFT_TOP,
        "Recent dictations",
        sans(14.5),
        pal.text(),
    );
    let g = ui.fonts(|f| {
        f.layout_no_wrap(
            format!("{}", app.state.history.len()),
            sans(11.0),
            pal.text_muted(),
        )
    });
    p.galley(
        Pos2::new(list.right() - 20.0 - g.size().x, list.top() + 22.0),
        g,
        pal.text_muted(),
    );

    if app.state.history.is_empty() {
        p.text(
            Pos2::new(list.center().x, list.center().y - 18.0),
            egui::Align2::CENTER_CENTER,
            "No dictations yet",
            sans(14.0),
            pal.text(),
        );
        p.text(
            Pos2::new(list.center().x, list.center().y + 4.0),
            egui::Align2::CENTER_CENTER,
            "Don't type, just speak.",
            sans(11.5),
            pal.text_muted(),
        );
    } else {
        let mut y = list.top() + 48.0;
        let shown: Vec<Dictation> = app.state.history.iter().rev().take(4).cloned().collect();
        for (i, d) in shown.iter().enumerate() {
            let row = Rect::from_min_size(
                Pos2::new(list.left() + 8.0, y),
                Vec2::new(list.width() - 16.0, 56.0),
            );
            let hov = hovered(ui, row, &format!("hist-{i}"));
            if hov {
                p.rect_filled(
                    row,
                    CornerRadius::same(9),
                    if pal.dark {
                        Color32::from_rgb(0x26, 0x26, 0x25)
                    } else {
                        SAND_100
                    },
                );
            }
            p.text(
                Pos2::new(row.left() + 12.0, row.center().y),
                egui::Align2::LEFT_CENTER,
                &d.at,
                sans(10.5),
                pal.text_faint(),
            );
            let tag = Rect::from_min_size(
                Pos2::new(row.left() + 70.0, row.center().y - 9.0),
                Vec2::new(62.0, 18.0),
            );
            p.rect_filled(
                tag,
                CornerRadius::same(5),
                if pal.dark {
                    Color32::from_rgb(0x30, 0x30, 0x2f)
                } else {
                    SAND_600
                },
            );
            p.text(
                tag.center(),
                egui::Align2::CENTER_CENTER,
                &d.app,
                sans(8.5),
                pal.text_muted(),
            );
            let max_chars = ((row.width() - 170.0) / 6.6).max(24.0) as usize;
            let text = if d.text.chars().count() > max_chars {
                format!("{}…", d.text.chars().take(max_chars).collect::<String>())
            } else {
                d.text.clone()
            };
            p.text(
                Pos2::new(row.left() + 144.0, row.center().y),
                egui::Align2::LEFT_CENTER,
                text,
                sans(11.5),
                pal.text(),
            );
            let copy = Rect::from_center_size(
                Pos2::new(row.right() - 22.0, row.center().y),
                Vec2::splat(26.0),
            );
            icon(
                &p,
                "copy",
                Rect::from_center_size(copy.center(), Vec2::splat(12.0)),
                pal.text_faint(),
                1.2,
            );
            if click(ui, copy, &format!("copy-{i}")) {
                ui.ctx().copy_text(d.text.clone());
                app.toast("Copied to clipboard".into());
            }
            y += 56.0;
        }
    }

    // right rail
    let rail = Rect::from_min_max(
        Pos2::new(x + list_w + 16.0, hero.bottom() + 124.0),
        Pos2::new(right, content.bottom() - 24.0),
    );
    let week = Rect::from_min_size(rail.min, Vec2::new(rail.width(), 190.0));
    rounded_card(ui, pal, week, 16);
    p.text(
        Pos2::new(week.left() + 18.0, week.top() + 16.0),
        egui::Align2::LEFT_TOP,
        "Your week",
        sans(13.5),
        pal.text(),
    );
    p.text(
        Pos2::new(week.left() + 18.0, week.top() + 38.0),
        egui::Align2::LEFT_TOP,
        format!("{} words · 7 days", stats.words),
        sans(10.0),
        pal.text_muted(),
    );
    let base = week.bottom() - 26.0;
    let top = week.top() + 66.0;
    let days = daily_words(&app.state.history);
    let max = days.iter().map(|(_, w)| *w).max().unwrap_or(0).max(1) as f32;
    let bw = (week.width() - 40.0) / 7.0;
    for (i, (_, words)) in days.iter().enumerate() {
        let bh = (*words as f32 / max) * (base - top);
        let r = Rect::from_min_size(
            Pos2::new(week.left() + 20.0 + i as f32 * bw + 3.0, base - bh),
            Vec2::new(bw - 6.0, bh),
        );
        p.rect_filled(
            r,
            CornerRadius::same(3),
            if i == 6 {
                pal.accent()
            } else if pal.dark {
                VAST_800
            } else {
                SAND_600
            },
        );
    }

    let tips = Rect::from_min_max(Pos2::new(rail.left(), week.bottom() + 16.0), rail.max);
    rounded_card(ui, pal, tips, 16);
    p.text(
        Pos2::new(tips.left() + 18.0, tips.top() + 16.0),
        egui::Align2::LEFT_TOP,
        "Try saying",
        sans(13.5),
        pal.text(),
    );
    let examples = [
        "Send a follow up to the design team",
        "Remind me to book the offsite",
        "Add a note about the pricing page",
    ];
    let mut yy = tips.top() + 48.0;
    for e in examples {
        p.text(
            Pos2::new(tips.left() + 18.0, yy),
            egui::Align2::LEFT_TOP,
            "“",
            sans(12.0),
            pal.text_faint(),
        );
        let h = wrapped(
            ui,
            Rect::from_min_size(
                Pos2::new(tips.left() + 27.0, yy),
                Vec2::new(tips.width() - 45.0, 36.0),
            ),
            e,
            sans(11.0),
            pal.text_muted(),
        );
        yy += h + 14.0;
    }
    let k = Rect::from_min_size(
        Pos2::new(tips.left() + 18.0, tips.bottom() - 40.0),
        Vec2::new(64.0, 24.0),
    );
    key_cap(ui, pal, k, "Space");
    p.text(
        Pos2::new(k.right() + 8.0, k.center().y),
        egui::Align2::LEFT_CENTER,
        "hold to speak",
        sans(9.5),
        pal.text_faint(),
    );
}

fn dictionary(ui: &mut egui::Ui, app: &mut App, pal: Palette, content: Rect) {
    let x = content.left() + PAD;
    let right = content.right() - PAD;
    header(
        ui,
        pal,
        content,
        "Dictionary",
        "Words Flowline should always spell your way.",
    );
    let p = ui.painter().clone();

    let search = Rect::from_min_size(Pos2::new(x, content.top() + 128.0), Vec2::new(280.0, 36.0));
    search_field(
        ui,
        pal,
        search,
        &mut app.search,
        "Search your vocabulary",
        "dict-search",
    );
    let add = Rect::from_min_size(
        Pos2::new(right - 106.0, search.top()),
        Vec2::new(106.0, 36.0),
    );
    if primary_button(ui, pal, add, "Add new", "dict-add") {
        app.form1.clear();
        app.form2.clear();
        app.modal = Some(Modal::AddWord);
    }

    let card = Rect::from_min_max(
        Pos2::new(x, search.bottom() + 16.0),
        Pos2::new(right, content.bottom() - 24.0),
    );
    rounded_card(ui, pal, card, 16);
    p.text(
        Pos2::new(card.left() + 20.0, card.top() + 16.0),
        egui::Align2::LEFT_TOP,
        "WORD",
        sans(9.5),
        pal.text_faint(),
    );
    p.text(
        Pos2::new(card.left() + 320.0, card.top() + 16.0),
        egui::Align2::LEFT_TOP,
        "ALWAYS WRITE IT AS",
        sans(9.5),
        pal.text_faint(),
    );
    p.line_segment(
        [
            Pos2::new(card.left() + 12.0, card.top() + 36.0),
            Pos2::new(card.right() - 12.0, card.top() + 36.0),
        ],
        Stroke::new(1.0, pal.border()),
    );

    let q = app.search.to_lowercase();
    let rows: Vec<usize> = app
        .state
        .dictionary
        .iter()
        .enumerate()
        .filter(|(_, d)| {
            q.is_empty()
                || d.word.to_lowercase().contains(&q)
                || d.replacement.to_lowercase().contains(&q)
        })
        .map(|(i, _)| i)
        .collect();

    if rows.is_empty() {
        p.text(
            card.center(),
            egui::Align2::CENTER_CENTER,
            "No words found matching your search",
            sans(12.5),
            pal.text_muted(),
        );
    }
    let mut y = card.top() + 44.0;
    for (n, i) in rows.iter().enumerate() {
        let d = app.state.dictionary[*i].clone();
        let row = Rect::from_min_size(
            Pos2::new(card.left() + 8.0, y),
            Vec2::new(card.width() - 16.0, 52.0),
        );
        let hov = hovered(ui, row, &format!("drow-{n}"));
        if hov {
            p.rect_filled(
                row,
                CornerRadius::same(9),
                if pal.dark {
                    Color32::from_rgb(0x26, 0x26, 0x25)
                } else {
                    SAND_100
                },
            );
        }
        let star = Rect::from_center_size(
            Pos2::new(row.left() + 22.0, row.center().y),
            Vec2::splat(26.0),
        );
        icon(
            &p,
            "star",
            Rect::from_center_size(star.center(), Vec2::splat(13.0)),
            if d.starred {
                GLOW_300
            } else {
                pal.text_faint()
            },
            1.2,
        );
        if click(ui, star, &format!("dstar-{n}")) {
            app.state.dictionary[*i].starred = !d.starred;
            app.save();
        }
        p.text(
            Pos2::new(row.left() + 52.0, row.center().y),
            egui::Align2::LEFT_CENTER,
            &d.word,
            sans(13.0),
            pal.text(),
        );
        if d.learned {
            let tag = Rect::from_min_size(
                Pos2::new(
                    row.left() + 52.0 + measure(ui, &d.word, 13.0) + 8.0,
                    row.center().y - 8.0,
                ),
                Vec2::new(52.0, 16.0),
            );
            p.rect_filled(
                tag,
                CornerRadius::same(4),
                if pal.dark { VAST_800 } else { SAND_600 },
            );
            p.text(
                tag.center(),
                egui::Align2::CENTER_CENTER,
                "learned",
                sans(8.5),
                pal.text_muted(),
            );
        }
        icon(
            &p,
            "arrow",
            Rect::from_center_size(
                Pos2::new(row.left() + 290.0, row.center().y),
                Vec2::splat(11.0),
            ),
            pal.text_faint(),
            1.1,
        );
        p.text(
            Pos2::new(row.left() + 320.0, row.center().y),
            egui::Align2::LEFT_CENTER,
            &d.replacement,
            sans(12.5),
            pal.text_muted(),
        );

        let edit = Rect::from_center_size(
            Pos2::new(row.right() - 52.0, row.center().y),
            Vec2::splat(26.0),
        );
        let trash = Rect::from_center_size(
            Pos2::new(row.right() - 24.0, row.center().y),
            Vec2::splat(26.0),
        );
        icon(
            &p,
            "edit",
            Rect::from_center_size(edit.center(), Vec2::splat(12.0)),
            pal.text_faint(),
            1.2,
        );
        icon(
            &p,
            "trash",
            Rect::from_center_size(trash.center(), Vec2::splat(12.0)),
            pal.text_faint(),
            1.2,
        );
        if click(ui, edit, &format!("dedit-{n}")) {
            app.form1 = d.word.clone();
            app.form2 = d.replacement.clone();
            app.modal = Some(Modal::EditWord(*i));
        }
        if click(ui, trash, &format!("ddel-{n}")) {
            app.modal = Some(Modal::DeleteWord(*i));
        }
        y += 52.0;
    }
    let g = ui.fonts(|f| {
        f.layout_no_wrap(
            format!("{} words", app.state.dictionary.len()),
            sans(10.5),
            pal.text_faint(),
        )
    });
    p.galley(
        Pos2::new(card.left() + 20.0, card.bottom() - 24.0),
        g,
        pal.text_faint(),
    );
}

fn snippets(ui: &mut egui::Ui, app: &mut App, pal: Palette, content: Rect) {
    let x = content.left() + PAD;
    let right = content.right() - PAD;
    header(
        ui,
        pal,
        content,
        "Snippets",
        "Say a short phrase, get a longer one.",
    );
    let p = ui.painter().clone();

    let search = Rect::from_min_size(Pos2::new(x, content.top() + 128.0), Vec2::new(280.0, 36.0));
    search_field(
        ui,
        pal,
        search,
        &mut app.search,
        "Search your snippets",
        "snip-search",
    );
    let add = Rect::from_min_size(
        Pos2::new(right - 106.0, search.top()),
        Vec2::new(106.0, 36.0),
    );
    if primary_button(ui, pal, add, "Add new", "snip-add") {
        app.form1.clear();
        app.form2.clear();
        app.modal = Some(Modal::AddSnippet);
    }

    let card = Rect::from_min_max(
        Pos2::new(x, search.bottom() + 16.0),
        Pos2::new(right, content.bottom() - 24.0),
    );
    rounded_card(ui, pal, card, 16);
    p.text(
        Pos2::new(card.left() + 20.0, card.top() + 16.0),
        egui::Align2::LEFT_TOP,
        "WHEN YOU SAY",
        sans(9.5),
        pal.text_faint(),
    );
    p.text(
        Pos2::new(card.left() + 320.0, card.top() + 16.0),
        egui::Align2::LEFT_TOP,
        "FLOWLINE WRITES",
        sans(9.5),
        pal.text_faint(),
    );
    p.line_segment(
        [
            Pos2::new(card.left() + 12.0, card.top() + 36.0),
            Pos2::new(card.right() - 12.0, card.top() + 36.0),
        ],
        Stroke::new(1.0, pal.border()),
    );

    let q = app.search.to_lowercase();
    let rows: Vec<usize> = app
        .state
        .snippets
        .iter()
        .enumerate()
        .filter(|(_, s)| {
            q.is_empty()
                || s.trigger.to_lowercase().contains(&q)
                || s.expansion.to_lowercase().contains(&q)
        })
        .map(|(i, _)| i)
        .collect();
    if rows.is_empty() {
        p.text(
            card.center(),
            egui::Align2::CENTER_CENTER,
            "No snippets were found",
            sans(12.5),
            pal.text_muted(),
        );
    }
    let mut y = card.top() + 44.0;
    for (n, i) in rows.iter().enumerate() {
        let s = app.state.snippets[*i].clone();
        let row = Rect::from_min_size(
            Pos2::new(card.left() + 8.0, y),
            Vec2::new(card.width() - 16.0, 56.0),
        );
        let _ = hovered(ui, row, &format!("srow-{n}"));
        p.text(
            Pos2::new(row.left() + 20.0, row.center().y),
            egui::Align2::LEFT_CENTER,
            format!("“{}”", s.trigger),
            sans(13.0),
            pal.text(),
        );
        icon(
            &p,
            "arrow",
            Rect::from_center_size(
                Pos2::new(row.left() + 290.0, row.center().y),
                Vec2::splat(11.0),
            ),
            pal.text_faint(),
            1.1,
        );
        let disp = if s.expansion.chars().count() > 64 {
            format!("{}…", s.expansion.chars().take(64).collect::<String>())
        } else {
            s.expansion.clone()
        };
        p.text(
            Pos2::new(row.left() + 320.0, row.center().y),
            egui::Align2::LEFT_CENTER,
            disp,
            sans(12.5),
            pal.text_muted(),
        );
        let edit = Rect::from_center_size(
            Pos2::new(row.right() - 52.0, row.center().y),
            Vec2::splat(26.0),
        );
        let trash = Rect::from_center_size(
            Pos2::new(row.right() - 24.0, row.center().y),
            Vec2::splat(26.0),
        );
        icon(
            &p,
            "edit",
            Rect::from_center_size(edit.center(), Vec2::splat(12.0)),
            pal.text_faint(),
            1.2,
        );
        icon(
            &p,
            "trash",
            Rect::from_center_size(trash.center(), Vec2::splat(12.0)),
            pal.text_faint(),
            1.2,
        );
        if click(ui, edit, &format!("sedit-{n}")) {
            app.form1 = s.trigger.clone();
            app.form2 = s.expansion.clone();
            app.modal = Some(Modal::EditSnippet(*i));
        }
        if click(ui, trash, &format!("sdel-{n}")) {
            app.modal = Some(Modal::DeleteSnippet(*i));
        }
        y += 56.0;
    }
    let g = ui.fonts(|f| {
        f.layout_no_wrap(
            format!("{} snippets", app.state.snippets.len()),
            sans(10.5),
            pal.text_faint(),
        )
    });
    p.galley(
        Pos2::new(card.left() + 20.0, card.bottom() - 24.0),
        g,
        pal.text_faint(),
    );
}

fn style_page(ui: &mut egui::Ui, app: &mut App, pal: Palette, content: Rect) {
    let x = content.left() + PAD;
    let right = content.right() - PAD;
    header(
        ui,
        pal,
        content,
        "Style",
        "How your words look when they come out.",
    );
    let p = ui.painter().clone();

    let tabs = [
        "Auto cleanup",
        "Work messages",
        "Email",
        "Personal messages",
        "Other",
        "Apps",
    ];
    let mut tx = x;
    for (i, t) in tabs.iter().enumerate() {
        let w = measure(ui, t, 12.5) + 26.0;
        let rect = Rect::from_min_size(Pos2::new(tx, content.top() + 124.0), Vec2::new(w, 30.0));
        let on = app.style_tab == i;
        if on {
            p.rect_filled(rect, CornerRadius::same(7), pal.accent());
        } else if hovered(ui, rect, &format!("styletab-{i}")) {
            p.rect_filled(rect, CornerRadius::same(7), pal.hover());
        }
        p.text(
            rect.center(),
            egui::Align2::CENTER_CENTER,
            *t,
            sans(12.0),
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
        if click(ui, rect, &format!("styletab-{i}")) {
            app.style_tab = i;
        }
        tx += w + 4.0;
    }

    if app.style_tab == 5 {
        let area = Rect::from_min_max(
            Pos2::new(x, content.top() + 172.0),
            Pos2::new(right, content.bottom() - 12.0),
        );
        crate::pages::style_apps(ui, app, pal, area);
        return;
    }

    if app.style_tab == 0 {
        let note = Rect::from_min_size(
            Pos2::new(x, content.top() + 176.0),
            Vec2::new(right - x, 56.0),
        );
        p.rect_filled(
            note,
            CornerRadius::same(10),
            if pal.dark {
                Color32::from_rgb(0x26, 0x26, 0x25)
            } else {
                SAND_200
            },
        );
        p.text(
            Pos2::new(note.left() + 16.0, note.top() + 12.0),
            egui::Align2::LEFT_TOP,
            "Auto Cleanup applies to all your dictations",
            sans(12.5),
            pal.text(),
        );
        p.text(
            Pos2::new(note.left() + 16.0, note.top() + 32.0),
            egui::Align2::LEFT_TOP,
            "Choose the level of cleanup that's applied every time, across all apps.",
            sans(10.5),
            pal.text_muted(),
        );

        let levels = [
            Cleanup::None,
            Cleanup::Light,
            Cleanup::Medium,
            Cleanup::High,
        ];
        let gap = 12.0;
        let cw = (right - x - gap * 3.0) / 4.0;
        for (i, level) in levels.iter().enumerate() {
            let card = Rect::from_min_size(
                Pos2::new(x + i as f32 * (cw + gap), note.bottom() + 16.0),
                Vec2::new(cw, 118.0),
            );
            let on = app.state.cleanup == *level;
            if on {
                ui.painter().rect(
                    card,
                    CornerRadius::same(14),
                    pal.card(),
                    Stroke::new(1.6, pal.accent()),
                    egui::StrokeKind::Inside,
                );
            } else {
                rounded_card(ui, pal, card, 14);
            }
            let radio = Rect::from_center_size(
                Pos2::new(card.left() + 22.0, card.top() + 24.0),
                Vec2::splat(16.0),
            );
            p.circle_stroke(
                radio.center(),
                7.0,
                Stroke::new(
                    1.2,
                    if on {
                        pal.accent()
                    } else {
                        pal.border_strong()
                    },
                ),
            );
            if on {
                p.circle_filled(radio.center(), 3.8, pal.accent());
            }
            p.text(
                Pos2::new(card.left() + 38.0, card.top() + 16.0),
                egui::Align2::LEFT_TOP,
                level.label(),
                sans(14.0),
                pal.text(),
            );
            wrapped(
                ui,
                Rect::from_min_size(
                    Pos2::new(card.left() + 20.0, card.top() + 52.0),
                    Vec2::new(card.width() - 40.0, 80.0),
                ),
                level.description(),
                sans(11.5),
                pal.text_muted(),
            );
            if click(ui, card, &format!("cleanup-{i}")) {
                app.state.cleanup = *level;
                app.save();
            }
        }

        let preview = Rect::from_min_max(
            Pos2::new(x, note.bottom() + 150.0),
            Pos2::new(right, note.bottom() + 290.0),
        );
        rounded_card(ui, pal, preview, 14);
        p.text(
            Pos2::new(preview.left() + 20.0, preview.top() + 16.0),
            egui::Align2::LEFT_TOP,
            "Preview",
            sans(12.5),
            pal.text(),
        );
        let raw = "um so I think we should ship the new onboarding flow on Friday";
        let cleaned = cleanup_text(raw, app.state.cleanup, Style::Formal);
        p.text(
            Pos2::new(preview.left() + 20.0, preview.top() + 46.0),
            egui::Align2::LEFT_TOP,
            "YOU SAID",
            sans(9.0),
            pal.text_faint(),
        );
        wrapped(
            ui,
            Rect::from_min_size(
                Pos2::new(preview.left() + 88.0, preview.top() + 44.0),
                Vec2::new(preview.width() - 108.0, 30.0),
            ),
            raw,
            sans(11.5),
            pal.text_muted(),
        );
        p.text(
            Pos2::new(preview.left() + 20.0, preview.top() + 86.0),
            egui::Align2::LEFT_TOP,
            "FLOWLINE WRITES",
            sans(9.0),
            pal.text_faint(),
        );
        wrapped(
            ui,
            Rect::from_min_size(
                Pos2::new(preview.left() + 122.0, preview.top() + 82.0),
                Vec2::new(preview.width() - 142.0, 40.0),
            ),
            &cleaned,
            sans(13.0),
            pal.text(),
        );
    } else {
        let styles = [
            Style::Formal,
            Style::Casual,
            Style::VeryCasual,
            Style::Excited,
        ];
        let current = app.style_for_tab(app.style_tab);
        let gap = 12.0;
        let cw = (right - x - gap) / 2.0;
        for (i, st) in styles.iter().enumerate() {
            let col = i % 2;
            let rowi = i / 2;
            let card = Rect::from_min_size(
                Pos2::new(
                    x + col as f32 * (cw + gap),
                    content.top() + 176.0 + rowi as f32 * 176.0,
                ),
                Vec2::new(cw, 164.0),
            );
            let on = current == *st;
            if on {
                ui.painter().rect(
                    card,
                    CornerRadius::same(14),
                    pal.card(),
                    Stroke::new(1.6, pal.accent()),
                    egui::StrokeKind::Inside,
                );
            } else {
                rounded_card(ui, pal, card, 14);
            }
            p.text(
                Pos2::new(card.left() + 20.0, card.top() + 16.0),
                egui::Align2::LEFT_TOP,
                st.label(),
                sans(14.5),
                pal.text(),
            );
            p.text(
                Pos2::new(card.left() + 20.0, card.top() + 38.0),
                egui::Align2::LEFT_TOP,
                st.subtitle(),
                sans(10.0),
                pal.text_faint(),
            );
            if on {
                p.circle_filled(
                    Pos2::new(card.right() - 22.0, card.top() + 24.0),
                    9.0,
                    pal.accent(),
                );
                icon(
                    &p,
                    "check",
                    Rect::from_center_size(
                        Pos2::new(card.right() - 22.0, card.top() + 24.0),
                        Vec2::splat(11.0),
                    ),
                    if pal.dark { VAST_950 } else { Color32::WHITE },
                    1.6,
                );
            }
            wrapped(
                ui,
                Rect::from_min_size(
                    Pos2::new(card.left() + 20.0, card.top() + 62.0),
                    Vec2::new(card.width() - 40.0, 90.0),
                ),
                &style_sample(*st, app.style_tab),
                sans(13.0),
                pal.text(),
            );
            if click(ui, card, &format!("style-{i}")) {
                app.set_style_for_tab(app.style_tab, *st);
                app.save();
            }
        }
    }
}

fn scratchpad(ui: &mut egui::Ui, app: &mut App, pal: Palette, content: Rect) {
    let x = content.left() + PAD;
    let right = content.right() - PAD;
    header(
        ui,
        pal,
        content,
        "Scratchpad",
        "A quiet place to dictate, edit and keep notes.",
    );
    let p = ui.painter().clone();

    let new_note = Rect::from_min_size(
        Pos2::new(right - 108.0, content.top() + 46.0),
        Vec2::new(108.0, 34.0),
    );
    if secondary_button(ui, pal, new_note, "New note", "note-new") {
        app.state.note.clear();
        app.save();
    }

    let editor = Rect::from_min_max(
        Pos2::new(x, content.top() + 128.0),
        Pos2::new(right - 300.0, content.bottom() - 24.0),
    );
    rounded_card(ui, pal, editor, 16);
    p.text(
        Pos2::new(editor.left() + 24.0, editor.top() + 20.0),
        egui::Align2::LEFT_TOP,
        "Weekly planning",
        sans(17.0),
        pal.text(),
    );
    p.text(
        Pos2::new(editor.left() + 24.0, editor.top() + 46.0),
        egui::Align2::LEFT_TOP,
        "Edited just now · saved locally",
        sans(10.0),
        pal.text_faint(),
    );
    p.line_segment(
        [
            Pos2::new(editor.left() + 16.0, editor.top() + 70.0),
            Pos2::new(editor.right() - 16.0, editor.top() + 70.0),
        ],
        Stroke::new(1.0, pal.border()),
    );
    let text_rect = Rect::from_min_max(
        Pos2::new(editor.left() + 24.0, editor.top() + 84.0),
        Pos2::new(editor.right() - 24.0, editor.bottom() - 20.0),
    );
    let note_resp = ui.put(
        text_rect,
        egui::TextEdit::multiline(&mut app.state.note)
            .frame(false)
            .font(sans(13.5))
            .text_color(pal.text())
            .desired_width(text_rect.width())
            .margin(Vec2::new(0.0, 4.0))
            .id(egui::Id::new("scratch-editor")),
    );
    if note_resp.changed() {
        app.save();
    }

    let side = Rect::from_min_max(
        Pos2::new(right - 280.0, content.top() + 128.0),
        Pos2::new(right, content.bottom() - 24.0),
    );
    rounded_card(ui, pal, side, 16);
    p.text(
        Pos2::new(side.left() + 20.0, side.top() + 18.0),
        egui::Align2::LEFT_TOP,
        "Dictate into this note",
        sans(13.5),
        pal.text(),
    );
    let mut yy = side.top() + 48.0;
    key_cap(
        ui,
        pal,
        Rect::from_min_size(Pos2::new(side.left() + 20.0, yy), Vec2::new(90.0, 26.0)),
        "Hold Space",
    );
    p.text(
        Pos2::new(side.left() + 20.0, yy + 34.0),
        egui::Align2::LEFT_TOP,
        "Speak, then release. Your words",
        sans(10.5),
        pal.text_muted(),
    );
    p.text(
        Pos2::new(side.left() + 20.0, yy + 48.0),
        egui::Align2::LEFT_TOP,
        "appear at the end of the note.",
        sans(10.5),
        pal.text_muted(),
    );
    yy += 84.0;
    p.line_segment(
        [
            Pos2::new(side.left() + 16.0, yy),
            Pos2::new(side.right() - 16.0, yy),
        ],
        Stroke::new(1.0, pal.border()),
    );
    yy += 14.0;
    p.text(
        Pos2::new(side.left() + 20.0, yy),
        egui::Align2::LEFT_TOP,
        "RECENT NOTES",
        sans(9.5),
        pal.text_faint(),
    );
    yy += 24.0;
    for (title, when) in [
        ("Product notes", "Yesterday"),
        ("Interview questions", "Mon"),
    ] {
        p.text(
            Pos2::new(side.left() + 20.0, yy),
            egui::Align2::LEFT_TOP,
            title,
            sans(12.0),
            pal.text(),
        );
        p.text(
            Pos2::new(side.right() - 20.0, yy + 1.0),
            egui::Align2::RIGHT_TOP,
            when,
            sans(9.5),
            pal.text_faint(),
        );
        yy += 28.0;
    }
    yy += 8.0;
    p.line_segment(
        [
            Pos2::new(side.left() + 16.0, yy),
            Pos2::new(side.right() - 16.0, yy),
        ],
        Stroke::new(1.0, pal.border()),
    );
    yy += 14.0;
    p.text(
        Pos2::new(side.left() + 20.0, yy),
        egui::Align2::LEFT_TOP,
        "RECENT DICTATIONS",
        sans(9.5),
        pal.text_faint(),
    );
    yy += 24.0;
    for d in app.state.history.iter().rev().take(3) {
        let t = if d.text.chars().count() > 34 {
            format!("{}…", d.text.chars().take(34).collect::<String>())
        } else {
            d.text.clone()
        };
        p.text(
            Pos2::new(side.left() + 20.0, yy),
            egui::Align2::LEFT_TOP,
            t,
            sans(10.5),
            pal.text_muted(),
        );
        yy += 20.0;
    }
    let _ = mmss(0.0);
}

fn insights(ui: &mut egui::Ui, app: &mut App, pal: Palette, content: Rect) {
    let x = content.left() + PAD;
    let right = content.right() - PAD;
    header(
        ui,
        pal,
        content,
        "Insights",
        "A little proof that speaking beats typing.",
    );
    let p = ui.painter().clone();
    let total_words: usize = app.state.history.iter().map(|d| d.words).sum();
    let mins = (total_words as f32 / 40.0 * 60.0) as i64;
    let cards = [
        ("TOTAL WORDS", format!("{total_words}"), "all time"),
        (
            "DICTATIONS",
            format!("{}", app.state.history.len()),
            "all time",
        ),
        ("MINUTES SAVED", format!("{mins}"), "at 40 wpm typing"),
        (
            "AVG. WORDS",
            format!(
                "{}",
                if app.state.history.is_empty() {
                    0
                } else {
                    total_words / app.state.history.len()
                }
            ),
            "per dictation",
        ),
    ];
    let gap = 12.0;
    let cw = (right - x - gap * 3.0) / 4.0;
    for (i, (label, value, note)) in cards.iter().enumerate() {
        let card = Rect::from_min_size(
            Pos2::new(x + i as f32 * (cw + gap), content.top() + 128.0),
            Vec2::new(cw, 92.0),
        );
        rounded_card(ui, pal, card, 14);
        p.text(
            Pos2::new(card.left() + 16.0, card.top() + 14.0),
            egui::Align2::LEFT_TOP,
            *label,
            sans(9.5),
            pal.text_faint(),
        );
        p.text(
            Pos2::new(card.left() + 16.0, card.top() + 34.0),
            egui::Align2::LEFT_TOP,
            value,
            sans(24.0),
            pal.text(),
        );
        p.text(
            Pos2::new(card.left() + 16.0, card.top() + 68.0),
            egui::Align2::LEFT_TOP,
            note,
            sans(9.5),
            pal.text_muted(),
        );
    }

    let chart = Rect::from_min_max(
        Pos2::new(x, content.top() + 236.0),
        Pos2::new(right - 320.0, content.bottom() - 24.0),
    );
    rounded_card(ui, pal, chart, 16);
    p.text(
        Pos2::new(chart.left() + 20.0, chart.top() + 18.0),
        egui::Align2::LEFT_TOP,
        "Words per day",
        sans(13.5),
        pal.text(),
    );
    let base = chart.bottom() - 40.0;
    let top = chart.top() + 60.0;
    let days = daily_words(&app.state.history);
    let max = days.iter().map(|(_, w)| *w).max().unwrap_or(0).max(1) as f32;
    let bw = (chart.width() - 80.0) / 7.0;
    for (i, (label, words)) in days.iter().enumerate() {
        let bh = (*words as f32 / max) * (base - top);
        let r = Rect::from_min_size(
            Pos2::new(chart.left() + 40.0 + i as f32 * bw + 8.0, base - bh),
            Vec2::new(bw - 16.0, bh),
        );
        p.rect_filled(
            r,
            CornerRadius::same(4),
            if i == 6 {
                pal.accent()
            } else if pal.dark {
                VAST_800
            } else {
                SAND_600
            },
        );
        p.text(
            Pos2::new(r.center().x, base + 8.0),
            egui::Align2::CENTER_TOP,
            label,
            sans(9.5),
            pal.text_faint(),
        );
    }

    let top_apps = Rect::from_min_max(
        Pos2::new(right - 300.0, content.top() + 236.0),
        Pos2::new(right, content.bottom() - 24.0),
    );
    rounded_card(ui, pal, top_apps, 16);
    p.text(
        Pos2::new(top_apps.left() + 20.0, top_apps.top() + 18.0),
        egui::Align2::LEFT_TOP,
        "Top apps",
        sans(13.5),
        pal.text(),
    );
    let mut yy = top_apps.top() + 56.0;
    let mut counts: Vec<(String, usize)> = Vec::new();
    for d in &app.state.history {
        if let Some(entry) = counts.iter_mut().find(|(name, _)| *name == d.app) {
            entry.1 += 1;
        } else {
            counts.push((d.app.clone(), 1));
        }
    }
    counts.sort_by(|a, b| b.1.cmp(&a.1));
    let total_dictations = app.state.history.len().max(1);
    if counts.is_empty() {
        p.text(
            Pos2::new(top_apps.left() + 20.0, yy),
            egui::Align2::LEFT_TOP,
            "No dictations yet",
            sans(11.0),
            pal.text_muted(),
        );
    }
    for (name, count) in counts.iter().take(3) {
        let pct = *count as f32 / total_dictations as f32;
        p.text(
            Pos2::new(top_apps.left() + 20.0, yy),
            egui::Align2::LEFT_TOP,
            name,
            sans(11.5),
            pal.text(),
        );
        p.text(
            Pos2::new(top_apps.right() - 20.0, yy),
            egui::Align2::RIGHT_TOP,
            format!("{}%", (pct * 100.0) as i32),
            sans(10.0),
            pal.text_muted(),
        );
        let bar = Rect::from_min_size(
            Pos2::new(top_apps.left() + 20.0, yy + 20.0),
            Vec2::new(top_apps.width() - 40.0, 5.0),
        );
        p.rect_filled(
            bar,
            CornerRadius::same(2),
            if pal.dark { VAST_800 } else { SAND_600 },
        );
        p.rect_filled(
            Rect::from_min_size(bar.min, Vec2::new(bar.width() * pct, 5.0)),
            CornerRadius::same(2),
            pal.accent(),
        );
        yy += 44.0;
    }
}

fn settings(ui: &mut egui::Ui, app: &mut App, pal: Palette, content: Rect) {
    let x = content.left() + PAD;
    let right = content.right() - PAD;
    header(
        ui,
        pal,
        content,
        "Settings",
        "Make Flowline feel like yours.",
    );
    let p = ui.painter().clone();

    let tabs = [
        (SettingsTab::General, "General"),
        (SettingsTab::System, "System"),
        (SettingsTab::Popup, "Popup"),
        (SettingsTab::Data, "Data and privacy"),
        (SettingsTab::Account, "Account"),
        (SettingsTab::About, "About"),
    ];
    let mut y = content.top() + 128.0;
    for (i, (tab, label)) in tabs.iter().enumerate() {
        let rect = Rect::from_min_size(Pos2::new(x, y), Vec2::new(168.0, 32.0));
        let on = app.settings_tab == *tab;
        if on {
            p.rect_filled(rect, CornerRadius::same(7), pal.selection());
        } else if hovered(ui, rect, &format!("settab-{i}")) {
            p.rect_filled(rect, CornerRadius::same(7), pal.hover());
        }
        p.text(
            Pos2::new(rect.left() + 12.0, rect.center().y),
            egui::Align2::LEFT_CENTER,
            *label,
            sans(12.0),
            if on { pal.text() } else { pal.text_muted() },
        );
        if click(ui, rect, &format!("settab-{i}")) {
            app.settings_tab = *tab;
        }
        y += 34.0;
    }

    let panel = Rect::from_min_max(
        Pos2::new(x + 200.0, content.top() + 128.0),
        Pos2::new((x + 960.0).min(right), content.bottom() - 24.0),
    );
    rounded_card(ui, pal, panel, 16);
    match app.settings_tab {
        SettingsTab::General => {
            let mut y = panel.top() + 20.0;
            setting_row(
                ui,
                pal,
                panel,
                &mut y,
                "Theme",
                "Light, dark or follow the system.",
            );
            let seg = Rect::from_min_size(
                Pos2::new(panel.right() - 152.0, y - 36.0),
                Vec2::new(132.0, 28.0),
            );
            p.rect_filled(
                seg,
                CornerRadius::same(7),
                if pal.dark { VAST_800 } else { SAND_500 },
            );
            let half = seg.width() / 2.0;
            for (i, label) in ["Light", "Dark"].iter().enumerate() {
                let r = Rect::from_min_size(
                    Pos2::new(seg.left() + i as f32 * half, seg.top()),
                    Vec2::new(half, seg.height()),
                );
                let on = (i == 1) == app.state.settings.dark;
                if on {
                    p.rect_filled(r.shrink(2.0), CornerRadius::same(5), pal.card());
                }
                p.text(
                    r.center(),
                    egui::Align2::CENTER_CENTER,
                    *label,
                    sans(11.0),
                    if on { pal.text() } else { pal.text_muted() },
                );
                if click(ui, r, &format!("theme-{i}")) {
                    app.state.settings.dark = i == 1;
                    app.save();
                }
            }
            setting_row(ui, pal, panel, &mut y, "Microphone", "Used for dictation.");
            if chevron_value(
                ui,
                pal,
                panel,
                y,
                &app.state.settings.microphone,
                "mic-cycle",
            ) {
                let mut devices = crate::audio::input_names();
                devices.insert(0, "System default".into());
                let i = devices
                    .iter()
                    .position(|d| *d == app.state.settings.microphone)
                    .unwrap_or(0);
                app.state.settings.microphone = devices[(i + 1) % devices.len()].to_string();
                app.save();
            }
            setting_row(
                ui,
                pal,
                panel,
                &mut y,
                "Languages",
                "Select the languages you speak.",
            );
            if chevron_value(
                ui,
                pal,
                panel,
                y,
                &app.state.settings.language,
                "lang-cycle",
            ) {
                let langs = [
                    "English (US)",
                    "English (UK)",
                    "Spanish",
                    "French",
                    "German",
                ];
                let i = langs
                    .iter()
                    .position(|d| *d == app.state.settings.language)
                    .unwrap_or(0);
                app.state.settings.language = langs[(i + 1) % langs.len()].to_string();
                app.save();
            }
            setting_row(
                ui,
                pal,
                panel,
                &mut y,
                "Shortcuts",
                "Hold Fn anywhere, even in another app. Ctrl Space also works.",
            );
            let k = Rect::from_min_size(
                Pos2::new(panel.right() - 84.0, y - 34.0),
                Vec2::new(64.0, 26.0),
            );
            key_cap(ui, pal, k, &app.state.settings.ptt);
            if click(ui, k, "ptt-cycle") {
                app.state.settings.ptt = if app.state.settings.ptt == "Fn" {
                    "Ctrl Space".into()
                } else {
                    "Fn".into()
                };
                crate::platform::set_hotkey_mode(&app.state.settings.ptt);
                crate::platform::ensure_hotkey_monitor(ui.ctx().clone());
                app.save();
            }
            setting_row(
                ui,
                pal,
                panel,
                &mut y,
                "Hands-free",
                "Double-tap the shortcut in the hub to keep listening.",
            );
            let on = app.state.settings.hands_free;
            let tr = Rect::from_min_size(
                Pos2::new(panel.right() - 54.0, y - 38.0),
                Vec2::new(36.0, 20.0),
            );
            if toggle(ui, pal, tr, on, "set-handsfree") {
                app.state.settings.hands_free = !on;
                app.save();
            }
            setting_row(
                ui,
                pal,
                panel,
                &mut y,
                "Quiet speech",
                "Boosts soft or whispered speech so it is heard. Best in a quiet room.",
            );
            let on = app.state.settings.quiet_mode;
            let tr = Rect::from_min_size(
                Pos2::new(panel.right() - 54.0, y - 38.0),
                Vec2::new(36.0, 20.0),
            );
            if toggle(ui, pal, tr, on, "set-quiet") {
                app.state.settings.quiet_mode = !on;
                app.save();
            }

            y += 4.0;
            p.line_segment(
                [
                    Pos2::new(panel.left() + 20.0, y),
                    Pos2::new(panel.right() - 20.0, y),
                ],
                Stroke::new(1.0, pal.border()),
            );
            y += 18.0;
            p.text(
                Pos2::new(panel.left() + 24.0, y),
                egui::Align2::LEFT_TOP,
                "Transcription engine",
                sans(13.5),
                pal.text(),
            );
            p.text(
                Pos2::new(panel.left() + 24.0, y + 20.0),
                egui::Align2::LEFT_TOP,
                if app.uses_local_engine() {
                    "Whisper runs on this Mac. Real transcription, no server needed."
                } else {
                    "Your own OpenAI-compatible server — pick an audio model or type an id."
                },
                sans(10.0),
                pal.text_muted(),
            );
            let (state_label, state_color) = engine_badge(app, pal);
            let pill_w = 142.0;
            status_pill(
                ui,
                pal,
                Rect::from_min_size(
                    Pos2::new(panel.right() - 24.0 - pill_w, y + 1.0),
                    Vec2::new(pill_w, 22.0),
                ),
                &state_label,
            );
            p.circle_filled(
                Pos2::new(panel.right() - 24.0 - pill_w + 10.0, y + 12.0),
                3.2,
                state_color,
            );
            y += 46.0;

            // Engine mode: on-device Whisper, or an OpenAI-compatible server.
            let local = app.uses_local_engine();
            let mut mc = panel.left() + 96.0;
            for (label, is_local) in [("On this Mac", true), ("Server", false)] {
                let w = measure(ui, label, 10.5) + 28.0;
                let chip = Rect::from_min_size(Pos2::new(mc, y), Vec2::new(w, 32.0));
                let selected = local == is_local;
                let hot = hovered(ui, chip, &format!("engine-mode-{is_local}"));
                if selected {
                    p.rect_filled(chip, CornerRadius::same(7), pal.accent());
                } else {
                    p.rect(
                        chip,
                        CornerRadius::same(7),
                        if hot { pal.hover() } else { pal.card_alt() },
                        Stroke::new(1.0, pal.border()),
                        egui::StrokeKind::Inside,
                    );
                }
                p.text(
                    chip.center(),
                    egui::Align2::CENTER_CENTER,
                    label,
                    sans(10.5),
                    if selected {
                        if pal.dark { VAST_950 } else { Color32::WHITE }
                    } else {
                        pal.text_muted()
                    },
                );
                if click(ui, chip, &format!("engine-mode-{is_local}")) {
                    app.state.settings.engine =
                        if is_local { "local".into() } else { "server".into() };
                    app.model_menu = false;
                    app.save();
                    app.probe_engine();
                }
                mc += w + 8.0;
            }
            y += 44.0;

            if local {
                let models = crate::engine::local_models();
                let (ready, detail) = crate::engine::local_ready(&app.state.settings.local_model);
                p.text(
                    Pos2::new(panel.left() + 24.0, y + 8.0),
                    egui::Align2::LEFT_TOP,
                    "Model",
                    sans(11.0),
                    pal.text_muted(),
                );
                let file = Rect::from_min_size(
                    Pos2::new(panel.left() + 96.0, y - 2.0),
                    Vec2::new(panel.width() - 96.0 - 24.0 - 84.0, 32.0),
                );
                p.rect(
                    file,
                    CornerRadius::same(7),
                    pal.card_alt(),
                    Stroke::new(1.0, pal.border()),
                    egui::StrokeKind::Inside,
                );
                p.text(
                    Pos2::new(file.left() + 12.0, file.center().y),
                    egui::Align2::LEFT_CENTER,
                    if ready { &detail } else { "No Whisper model installed" },
                    sans(11.0),
                    if ready { pal.text() } else { pal.text_muted() },
                );
                let cycle = Rect::from_min_size(
                    Pos2::new(panel.right() - 104.0, y - 2.0),
                    Vec2::new(80.0, 32.0),
                );
                if secondary_button(ui, pal, cycle, "Change", "local-model-cycle") && models.len() > 1 {
                    let current = crate::engine::local_model_file(&app.state.settings.local_model)
                        .ok()
                        .and_then(|p| p.file_name().map(|n| n.to_string_lossy().into_owned()));
                    let idx = models.iter().position(|m| Some(m) == current.as_ref()).unwrap_or(0);
                    app.state.settings.local_model = models[(idx + 1) % models.len()].clone();
                    app.save();
                    app.probe_engine();
                }
                y += 44.0;
                if ready {
                    p.text(
                        Pos2::new(panel.left() + 96.0, y - 10.0),
                        egui::Align2::LEFT_TOP,
                        "Real speech-to-text on this Mac. Audio never leaves it.",
                        sans(9.5),
                        pal.text_faint(),
                    );
                    y += 22.0;
                } else {
                    wrapped(
                        ui,
                        Rect::from_min_size(
                            Pos2::new(panel.left() + 96.0, y - 10.0),
                            Vec2::new(panel.width() - 120.0, 48.0),
                        ),
                        &detail,
                        sans(9.5),
                        GLOW_300,
                    );
                    y += 54.0;
                    // One click gets a model: downloaded on request from
                    // huggingface.co, nothing else is contacted.
                    if let Some(dl) = app.ext.dl {
                        let got = app.ext.dl_progress.load(std::sync::atomic::Ordering::Relaxed);
                        let total = dl.mb as u64 * 1_000_000;
                        let frac = (got as f32 / total as f32).clamp(0.0, 1.0);
                        p.text(
                            Pos2::new(panel.left() + 96.0, y),
                            egui::Align2::LEFT_TOP,
                            format!(
                                "Downloading {} · {} of ~{} MB",
                                dl.label,
                                got / 1_000_000,
                                dl.mb
                            ),
                            sans(11.0),
                            pal.text(),
                        );
                        let bar = Rect::from_min_size(
                            Pos2::new(panel.left() + 96.0, y + 22.0),
                            Vec2::new(panel.width() - 120.0, 8.0),
                        );
                        p.rect_filled(bar, CornerRadius::same(4), pal.border());
                        p.rect_filled(
                            Rect::from_min_size(bar.min, Vec2::new(bar.width() * frac, 8.0)),
                            CornerRadius::same(4),
                            pal.accent(),
                        );
                        y += 44.0;
                    } else {
                        let mut bx = panel.left() + 96.0;
                        for d in crate::engine::DOWNLOADS.iter() {
                            let label = format!("{} · ~{} MB", d.label, d.mb);
                            let w = measure(ui, &label, 14.5) + 32.0;
                            let r = Rect::from_min_size(Pos2::new(bx, y - 8.0), Vec2::new(w, 32.0));
                            let key = format!("dl-{}", d.id);
                            let clicked = if d.id == "small" {
                                primary_button(ui, pal, r, &label, &key)
                            } else {
                                secondary_button(ui, pal, r, &label, &key)
                            };
                            if clicked {
                                app.start_model_download(d.id);
                            }
                            bx += w + 8.0;
                        }
                        y += 36.0;
                        p.text(
                            Pos2::new(panel.left() + 96.0, y - 4.0),
                            egui::Align2::LEFT_TOP,
                            "Downloads from huggingface.co once. Small is recommended.",
                            sans(9.5),
                            pal.text_faint(),
                        );
                        y += 20.0;
                        if let Some(err) = app.ext.dl_error.clone() {
                            wrapped(
                                ui,
                                Rect::from_min_size(
                                    Pos2::new(panel.left() + 96.0, y - 4.0),
                                    Vec2::new(panel.width() - 120.0, 30.0),
                                ),
                                &format!("Download failed: {err}"),
                                sans(10.0),
                                RED_450,
                            );
                            y += 32.0;
                        }
                    }
                }
                let folder = Rect::from_min_size(
                    Pos2::new(panel.left() + 96.0, y),
                    Vec2::new(150.0, 30.0),
                );
                if secondary_button(ui, pal, folder, "Show models folder", "local-folder") {
                    if let Some(dir) = crate::engine::local_dir() {
                        let _ = std::fs::create_dir_all(&dir);
                        let _ = std::process::Command::new("open").arg(dir).spawn();
                    }
                }
                let _ = models;
            } else {
            // Server preset chips. Picking one fills the endpoint and a model
            // that server commonly serves; both stay editable.
            p.text(
                Pos2::new(panel.left() + 24.0, y + 8.0),
                egui::Align2::LEFT_TOP,
                "Server",
                sans(11.0),
                pal.text_muted(),
            );
            let chips_left = panel.left() + 96.0;
            let chips_right = panel.right() - 20.0;
            let mut cx = chips_left;
            let mut cy = y;
            let mut lines = 1usize;
            let current =
                crate::engine::preset_for(&app.state.settings.engine_endpoint).map(|p| p.name);
            for (i, preset) in crate::engine::PRESETS.iter().enumerate() {
                let w = measure(ui, preset.name, 10.5) + 24.0;
                if cx > chips_left && cx + w > chips_right {
                    cx = chips_left;
                    cy += 34.0;
                    lines += 1;
                }
                let r = Rect::from_min_size(Pos2::new(cx, cy), Vec2::new(w, 28.0));
                let on = current == Some(preset.name);
                let hot = hovered(ui, r, &format!("preset-{i}"));
                if on {
                    p.rect_filled(r, CornerRadius::same(7), pal.accent());
                    p.text(
                        r.center(),
                        egui::Align2::CENTER_CENTER,
                        preset.name,
                        sans(10.5),
                        if pal.dark { VAST_950 } else { Color32::WHITE },
                    );
                } else {
                    p.rect(
                        r,
                        CornerRadius::same(7),
                        if hot { pal.hover() } else { pal.card_alt() },
                        Stroke::new(1.0, pal.border()),
                        egui::StrokeKind::Inside,
                    );
                    p.text(
                        r.center(),
                        egui::Align2::CENTER_CENTER,
                        preset.name,
                        sans(10.5),
                        pal.text_muted(),
                    );
                }
                if click(ui, r, &format!("preset-{i}")) {
                    app.state.settings.engine_endpoint = preset.base.to_string();
                    app.state.settings.model = preset.model.to_string();
                    app.model_menu = false;
                    app.save();
                    app.probe_engine();
                }
                cx += w + 6.0;
            }
            if current.is_none() {
                let note_w = measure(ui, "Custom endpoint", 10.0);
                if cx + note_w < chips_right {
                    p.text(
                        Pos2::new(cx, cy + 8.0),
                        egui::Align2::LEFT_TOP,
                        "Custom endpoint",
                        sans(10.0),
                        pal.text_faint(),
                    );
                }
            }
            y += if lines > 1 {
                34.0 * lines as f32 + 8.0
            } else {
                42.0
            };
            p.text(
                Pos2::new(panel.left() + 24.0, y + 8.0),
                egui::Align2::LEFT_TOP,
                "Model",
                sans(11.0),
                pal.text_muted(),
            );
            let model_rect = Rect::from_min_size(
                Pos2::new(panel.left() + 96.0, y - 2.0),
                Vec2::new(panel.width() - 96.0 - 66.0, 32.0),
            );
            let before_model = app.state.settings.model.clone();
            let model_resp = field(
                ui,
                pal,
                model_rect,
                &mut app.state.settings.model,
                "type any model id",
                "engine-model",
            );
            if before_model != app.state.settings.model {
                app.save();
            }
            if model_resp.lost_focus() && app.state.settings.model != before_model {
                app.probe_engine();
            }
            let chev = Rect::from_min_size(
                Pos2::new(model_rect.right() + 6.0, y - 2.0),
                Vec2::new(34.0, 32.0),
            );
            let chev_hot = hovered(ui, chev, "engine-model-menu");
            p.rect(
                chev,
                CornerRadius::same(7),
                if chev_hot {
                    pal.hover()
                } else {
                    pal.card_alt()
                },
                Stroke::new(1.0, pal.border()),
                egui::StrokeKind::Inside,
            );
            let cc = chev.center();
            p.add(egui::Shape::line(
                vec![
                    Pos2::new(cc.x - 4.0, cc.y - 2.0),
                    Pos2::new(cc.x, cc.y + 2.0),
                    Pos2::new(cc.x + 4.0, cc.y - 2.0),
                ],
                Stroke::new(1.2, pal.text_muted()),
            ));
            let mut opened_now = false;
            if click(ui, chev, "engine-model-menu") {
                app.model_menu = !app.model_menu;
                opened_now = app.model_menu;
            }

            y += 42.0;
            p.text(
                Pos2::new(panel.left() + 24.0, y + 8.0),
                egui::Align2::LEFT_TOP,
                "Endpoint",
                sans(11.0),
                pal.text_muted(),
            );
            let retry_w = if app.last_audio.is_some() { 96.0 } else { 0.0 };
            let check_w = 70.0;
            let actions_w = check_w + if retry_w > 0.0 { retry_w + 6.0 } else { 0.0 };
            let endpoint_rect = Rect::from_min_size(
                Pos2::new(panel.left() + 96.0, y - 2.0),
                Vec2::new(panel.width() - 96.0 - 24.0 - actions_w - 8.0, 32.0),
            );
            let before_base = app.state.settings.engine_endpoint.clone();
            let endpoint_resp = field(
                ui,
                pal,
                endpoint_rect,
                &mut app.state.settings.engine_endpoint,
                crate::engine::DEFAULT_BASE,
                "engine-endpoint",
            );
            if before_base != app.state.settings.engine_endpoint {
                app.state.settings.engine_endpoint =
                    app.state.settings.engine_endpoint.trim().to_string();
                app.save();
            }
            if endpoint_resp.lost_focus() && app.state.settings.engine_endpoint != before_base {
                app.probe_engine();
            }
            let mut bx = panel.right() - 24.0 - check_w;
            let check = Rect::from_min_size(Pos2::new(bx, y - 2.0), Vec2::new(check_w, 32.0));
            if secondary_button(ui, pal, check, "Check", "engine-check") {
                app.model_menu = false;
                app.probe_engine();
            }
            if app.last_audio.is_some() {
                bx -= retry_w + 6.0;
                let retry = Rect::from_min_size(Pos2::new(bx, y - 2.0), Vec2::new(retry_w, 32.0));
                if secondary_button(ui, pal, retry, "Retry audio", "engine-retry") {
                    if let Some((wav, secs)) = app.last_audio.clone() {
                        app.start_transcribing(wav, secs);
                    }
                }
            }

            y += 44.0;
            if crate::engine::is_remote(&app.state.settings.engine_endpoint) {
                let host = crate::engine::host_of(&app.state.settings.engine_endpoint)
                    .unwrap_or_else(|| "a remote host".into());
                p.text(
                    Pos2::new(panel.left() + 96.0, y - 12.0),
                    egui::Align2::LEFT_TOP,
                    format!("Remote endpoint — audio is sent to {host}. Use only a server you trust."),
                    sans(9.5),
                    GLOW_300,
                );
                y += 16.0;
            }
            let note = if app.engine_checking {
                "Checking the server…".to_string()
            } else if app.engine.reachable
                && !app.engine.models.is_empty()
                && !crate::engine::has_speech_model(&app.engine.models)
                && crate::engine::has_tts_model(&app.engine.models)
            {
                "This is a text-to-speech server (Kokoro/tts-1): it turns text into audio. Dictation needs a speech-to-text model such as Whisper (e.g. speaches on /v1).".to_string()
            } else if app.engine.reachable
                && !app.engine.models.is_empty()
                && !crate::engine::has_speech_model(&app.engine.models)
            {
                "This server has no speech-to-text model. Ollama cannot transcribe audio; run a Whisper server such as speaches and use its /v1 endpoint.".to_string()
            } else if app.engine.ready() {
                let model = crate::engine::requested_model(&app.state.settings.model);
                {
                    let host = crate::engine::host_of(&app.state.settings.engine_endpoint)
                        .unwrap_or_else(|| "the server".into());
                    format!("{model} is connected on {host}. Audio is sent there; only speech-to-text models return text.")
                }
            } else if app.engine.reachable {
                app.engine.problem.clone().unwrap_or_else(|| {
                    format!(
                        "{} is not in the server's list; requests still use it.",
                        crate::engine::requested_model(&app.state.settings.model)
                    )
                })
            } else {
                app.engine.problem.clone().unwrap_or_else(|| {
                    "The engine is not reachable. Start the server or change the endpoint."
                        .into()
                })
            };
            wrapped(
                ui,
                Rect::from_min_size(
                    Pos2::new(panel.left() + 24.0, y),
                    Vec2::new(panel.width() - 48.0, 44.0),
                ),
                &note,
                sans(9.5),
                pal.text_faint(),
            );

            // The detected-model menu is drawn last so it floats over the form.
            if app.model_menu {
                let mut choices: Vec<String> = Vec::new();
                // Speech-looking ids first, then the rest, keeping the
                // server's order inside each group.
                let mut detected: Vec<&String> = app.engine.models.iter().collect();
                detected.sort_by_key(|m| !crate::engine::looks_like_speech(m));
                for m in detected {
                    if !choices.iter().any(|c| c.eq_ignore_ascii_case(m)) {
                        choices.push(m.clone());
                    }
                }
                for m in crate::engine::SUGGESTED_MODELS {
                    if !choices.iter().any(|c| c.eq_ignore_ascii_case(m)) {
                        choices.push(m.to_string());
                    }
                }
                let shown = choices.len().min(5);
                let rows = shown + 1;
                let h = 46.0 + rows as f32 * 28.0 + 6.0;
                let width = model_rect.width().max(320.0);
                let mut menu = Rect::from_min_size(
                    Pos2::new(model_rect.left(), model_rect.bottom() + 6.0),
                    Vec2::new(width, h),
                );
                if menu.bottom() > panel.bottom() - 10.0 {
                    let top = (model_rect.top() - h - 6.0).max(panel.top() + 10.0);
                    menu =
                        Rect::from_min_size(Pos2::new(model_rect.left(), top), Vec2::new(width, h));
                }
                shadow(&p, menu, 12);
                rounded_card(ui, pal, menu, 12);
                let head = if app.engine.models.is_empty() {
                    "Suggested models"
                } else {
                    "Models on this server"
                };
                p.text(
                    Pos2::new(menu.left() + 14.0, menu.top() + 9.0),
                    egui::Align2::LEFT_TOP,
                    head,
                    sans(9.5),
                    pal.text_faint(),
                );
                let no_speech = !app.engine.models.is_empty()
                    && !app
                        .engine
                        .models
                        .iter()
                        .any(|m| crate::engine::looks_like_speech(m));
                let hint = if no_speech {
                    "No speech-to-text model found — type one below."
                } else {
                    "Pick a speech model, or type any id in the field."
                };
                p.text(
                    Pos2::new(menu.left() + 14.0, menu.top() + 24.0),
                    egui::Align2::LEFT_TOP,
                    hint,
                    sans(9.0),
                    if no_speech {
                        GLOW_300
                    } else {
                        pal.text_faint()
                    },
                );
                let mut my = menu.top() + 44.0;
                let current_model = crate::engine::requested_model(&app.state.settings.model);
                for (i, m) in choices.iter().take(shown).enumerate() {
                    let r = Rect::from_min_size(
                        Pos2::new(menu.left() + 6.0, my),
                        Vec2::new(menu.width() - 12.0, 26.0),
                    );
                    let hot = hovered(ui, r, &format!("model-pick-{i}"));
                    if hot {
                        p.rect_filled(r, CornerRadius::same(6), pal.hover());
                    }
                    let is = m.eq_ignore_ascii_case(&current_model);
                    let label: String = if m.chars().count() > 36 {
                        format!("{}…", m.chars().take(35).collect::<String>())
                    } else {
                        m.clone()
                    };
                    p.text(
                        Pos2::new(r.left() + 10.0, r.center().y),
                        egui::Align2::LEFT_CENTER,
                        &label,
                        sans(11.0),
                        if is { pal.text() } else { pal.text_muted() },
                    );
                    if is {
                        icon(
                            &p,
                            "check",
                            Rect::from_center_size(
                                Pos2::new(r.right() - 12.0, r.center().y),
                                Vec2::splat(12.0),
                            ),
                            pal.accent(),
                            1.6,
                        );
                    }
                    if click(ui, r, &format!("model-pick-{i}")) {
                        app.state.settings.model = m.clone();
                        app.model_menu = false;
                        app.save();
                        app.probe_engine();
                    }
                    my += 28.0;
                }
                let scan = Rect::from_min_size(
                    Pos2::new(menu.left() + 6.0, my),
                    Vec2::new(menu.width() - 12.0, 26.0),
                );
                let scan_hot = hovered(ui, scan, "model-rescan");
                if scan_hot {
                    p.rect_filled(scan, CornerRadius::same(6), pal.hover());
                }
                p.line_segment(
                    [
                        Pos2::new(menu.left() + 12.0, my - 3.0),
                        Pos2::new(menu.right() - 12.0, my - 3.0),
                    ],
                    Stroke::new(1.0, pal.border()),
                );
                p.text(
                    Pos2::new(scan.left() + 10.0, scan.center().y),
                    egui::Align2::LEFT_CENTER,
                    "Scan the server again",
                    sans(10.5),
                    pal.text_faint(),
                );
                if click(ui, scan, "model-rescan") {
                    app.model_menu = false;
                    app.probe_engine();
                }
                if !opened_now && ui.input(|i| i.pointer.any_click()) {
                    let outside = ui
                        .input(|i| i.pointer.interact_pos())
                        .map(|q| !menu.contains(q))
                        .unwrap_or(false);
                    if outside {
                        app.model_menu = false;
                    }
                }
            }
            }
        }
        SettingsTab::System => {
            let mut y = panel.top() + 20.0;
            setting_row(
                ui,
                pal,
                panel,
                &mut y,
                "Dictation sounds",
                "A soft cue when listening starts and stops.",
            );
            let tr = Rect::from_min_size(
                Pos2::new(panel.right() - 54.0, y - 38.0),
                Vec2::new(36.0, 20.0),
            );
            let on = app.state.settings.sounds;
            if toggle(ui, pal, tr, on, "set-sounds") {
                app.state.settings.sounds = !on;
                app.save();
            }
            setting_row(
                ui,
                pal,
                panel,
                &mut y,
                "Mute other audio",
                "Mute music and other sound while dictating. Off by default, as shipped.",
            );
            let tr = Rect::from_min_size(
                Pos2::new(panel.right() - 54.0, y - 38.0),
                Vec2::new(36.0, 20.0),
            );
            let on = app.state.settings.mute_media;
            if toggle(ui, pal, tr, on, "set-mute-media") {
                app.state.settings.mute_media = !on;
                app.save();
            }
            setting_row(
                ui,
                pal,
                panel,
                &mut y,
                "Launch app at login",
                "Flowline is ready when your Mac starts.",
            );
            let tr = Rect::from_min_size(
                Pos2::new(panel.right() - 54.0, y - 38.0),
                Vec2::new(36.0, 20.0),
            );
            let on = app.state.settings.launch_at_login;
            if toggle(ui, pal, tr, on, "set-login") {
                match crate::platform::set_launch_at_login(!on) {
                    Ok(()) => {
                        app.state.settings.launch_at_login = !on;
                        app.save();
                        app.toast(if on {
                            "Flowline will not start at login".into()
                        } else {
                            "Flowline will start at login".into()
                        });
                    }
                    Err(e) => app.toast(format!("Login item could not be changed: {e}")),
                }
            }
            setting_row(
                ui,
                pal,
                panel,
                &mut y,
                "Show app in dock",
                "Hide Flowline and use only the shortcut.",
            );
            let tr = Rect::from_min_size(
                Pos2::new(panel.right() - 54.0, y - 38.0),
                Vec2::new(36.0, 20.0),
            );
            let on = app.state.settings.show_in_dock;
            if toggle(ui, pal, tr, on, "set-dock") {
                app.state.settings.show_in_dock = !on;
                crate::platform::set_dock_visible(!on);
                app.save();
            }
            setting_row(
                ui,
                pal,
                panel,
                &mut y,
                "Scratchpad",
                "Open in a new tab when dictating.",
            );
            let _ = chevron_value(ui, pal, panel, y, "New tab", "scratch-cycle");

            y += 10.0;
            p.line_segment(
                [
                    Pos2::new(panel.left() + 20.0, y),
                    Pos2::new(panel.right() - 20.0, y),
                ],
                Stroke::new(1.0, pal.border()),
            );
            y += 18.0;
            p.text(
                Pos2::new(panel.left() + 24.0, y),
                egui::Align2::LEFT_TOP,
                "Permissions",
                sans(13.0),
                pal.text(),
            );
            y += 28.0;
            let permissions = [
                (
                    "Input Monitoring",
                    "Optional. Only a fallback when Accessibility is off; Flowline never asks for it.",
                    crate::platform::input_monitoring_granted(),
                    "perm-input",
                    "ListenEvent",
                ),
                (
                    "Accessibility",
                    "Lets Flowline paste the transcript into the app you were in.",
                    crate::platform::accessibility_granted(),
                    "perm-ax",
                    "Accessibility",
                ),
                (
                    "Microphone",
                    "Asked the first time you record. Enable opens the pane.",
                    crate::platform::mic_authorized(),
                    "perm-mic",
                    "Microphone",
                ),
            ];
            for (title, sub, ok, key, anchor) in permissions {
                setting_row(ui, pal, panel, &mut y, title, sub);
                let label = if ok {
                    "Allowed"
                } else if anchor == "ListenEvent" {
                    "Optional"
                } else {
                    "Enable"
                };
                let pill = Rect::from_min_size(
                    Pos2::new(panel.right() - 104.0, y - 49.0),
                    Vec2::new(84.0, 26.0),
                );
                let hot = hovered(ui, pill, key);
                ui.painter().rect(
                    pill,
                    CornerRadius::same(7),
                    if hot { pal.hover() } else { pal.card() },
                    Stroke::new(1.0, if ok { pal.border() } else { pal.border_strong() }),
                    egui::StrokeKind::Inside,
                );
                ui.painter().text(
                    pill.center(),
                    egui::Align2::CENTER_CENTER,
                    label,
                    sans(10.5),
                    if ok { pal.text_faint() } else { pal.text() },
                );
                if click(ui, pill, key) {
                    match anchor {
                        "ListenEvent" => {
                            crate::platform::request_input_monitoring();
                            crate::platform::ensure_hotkey_monitor(ui.ctx().clone());
                            crate::platform::open_privacy_pane("ListenEvent");
                            app.toast("Allow Flowline, then reopen the app".into());
                        }
                        "Microphone" => {
                            crate::platform::open_privacy_pane("Microphone");
                        }
                        _ => {
                            crate::platform::request_accessibility();
                            crate::platform::open_privacy_pane("Accessibility");
                        }
                    }
                }
            }
        }
        SettingsTab::Popup => {
            let mut y = panel.top() + 24.0;
            setting_row(
                ui,
                pal,
                panel,
                &mut y,
                "Position",
                "Where the floating pill and transcript card appear.",
            );
            let current = app.state.settings.popup_position.clone();
            let label = match current.as_str() {
                "bottom-right" => "Bottom right",
                "top-center" => "Top centre",
                "custom" => "Custom (drag)",
                _ => "Bottom centre",
            };
            if chevron_value(ui, pal, panel, y, label, "popup-position") {
                app.state.settings.popup_position = match current.as_str() {
                    "bottom-center" => "bottom-right".into(),
                    "bottom-right" => "top-center".into(),
                    "top-center" => "custom".into(),
                    _ => "bottom-center".into(),
                };
                app.save();
            }
            setting_row(
                ui,
                pal,
                panel,
                &mut y,
                "Pasted popup",
                "How long the confirmation stays; Until dismissed keeps it on screen.",
            );
            let ms = app.state.settings.popup_timeout_ms;
            let tlabel = match ms {
                0 => "Until dismissed".to_string(),
                1000 => "1 second".to_string(),
                2000 => "2 seconds".to_string(),
                3000 => "3 seconds".to_string(),
                5000 => "5 seconds".to_string(),
                _ => "10 seconds".to_string(),
            };
            if chevron_value(ui, pal, panel, y, &tlabel, "popup-timeout") {
                app.state.settings.popup_timeout_ms = match ms {
                    0 => 1000,
                    1000 => 2000,
                    2000 => 3000,
                    3000 => 5000,
                    5000 => 10000,
                    _ => 0,
                };
                app.save();
            }
            setting_row(
                ui,
                pal,
                panel,
                &mut y,
                "No text field focused",
                "The transcript waits in the popup with a Copy button until you dismiss it.",
            );
            setting_row(
                ui,
                pal,
                panel,
                &mut y,
                "Fn opens macOS Search?",
                "macOS owns the Globe action; turn it off in System Settings → Keyboard so Fn reaches Flowline.",
            );
            let kb = Rect::from_min_size(
                Pos2::new(panel.right() - 196.0, y - 46.0),
                Vec2::new(172.0, 30.0),
            );
            if secondary_button(ui, pal, kb, "Open Keyboard settings", "open-keyboard") {
                let _ = std::process::Command::new("open")
                    .arg("x-apple.systempreferences:com.apple.Keyboard-Settings.extension")
                    .spawn();
            }
            if app.state.settings.popup_position == "custom" {
                wrapped(
                    ui,
                    Rect::from_min_size(
                        Pos2::new(panel.left() + 24.0, y + 2.0),
                        Vec2::new(panel.width() - 48.0, 40.0),
                    ),
                    "Drag the popup where you want it while dictating; the position is saved when the dictation ends.",
                    sans(10.0),
                    GLOW_300,
                );
            }
        }
        SettingsTab::Data => {
            let mut y = panel.top() + 24.0;
            p.text(
                Pos2::new(panel.left() + 24.0, y),
                egui::Align2::LEFT_TOP,
                "Local data storage",
                sans(13.5),
                pal.text(),
            );
            p.text(
                Pos2::new(panel.left() + 24.0, y + 20.0),
                egui::Align2::LEFT_TOP,
                "Transcripts stay on this Mac. Cloud sync is off.",
                sans(10.5),
                pal.text_muted(),
            );
            y += 56.0;
            for (i, label) in [
                "Store data locally",
                "Auto-delete local data every 24 hours",
                "Never store data locally",
            ]
            .iter()
            .enumerate()
            {
                let row = Rect::from_min_size(
                    Pos2::new(panel.left() + 20.0, y),
                    Vec2::new(panel.width() - 40.0, 34.0),
                );
                let on = app.state.settings.storage as usize == i;
                p.circle_stroke(
                    Pos2::new(row.left() + 10.0, row.center().y),
                    7.0,
                    Stroke::new(
                        1.2,
                        if on {
                            pal.accent()
                        } else {
                            pal.border_strong()
                        },
                    ),
                );
                if on {
                    p.circle_filled(
                        Pos2::new(row.left() + 10.0, row.center().y),
                        3.6,
                        pal.accent(),
                    );
                }
                p.text(
                    Pos2::new(row.left() + 28.0, row.center().y),
                    egui::Align2::LEFT_CENTER,
                    *label,
                    sans(12.0),
                    pal.text(),
                );
                if click(ui, row, &format!("storage-{i}")) {
                    app.state.settings.storage = i as u8;
                    app.save();
                }
                y += 36.0;
            }
            y += 20.0;
            p.line_segment(
                [
                    Pos2::new(panel.left() + 20.0, y),
                    Pos2::new(panel.right() - 20.0, y),
                ],
                Stroke::new(1.0, pal.border()),
            );
            y += 20.0;
            p.text(
                Pos2::new(panel.left() + 24.0, y),
                egui::Align2::LEFT_TOP,
                "Delete your data",
                sans(13.5),
                pal.text(),
            );
            p.text(
                Pos2::new(panel.left() + 24.0, y + 20.0),
                egui::Align2::LEFT_TOP,
                "Permanently delete your dictation history from this device.",
                sans(10.5),
                pal.text_muted(),
            );
            let del = Rect::from_min_size(
                Pos2::new(panel.left() + 24.0, y + 46.0),
                Vec2::new(168.0, 34.0),
            );
            if secondary_button(ui, pal, del, "Delete all dictations", "delete-history") {
                app.modal = Some(Modal::DeleteHistory);
            }
        }
        SettingsTab::Account => {
            let mut y = panel.top() + 24.0;
            p.circle_filled(
                Pos2::new(panel.left() + 44.0, y + 24.0),
                24.0,
                if pal.dark { VAST_800 } else { SAND_600 },
            );
            p.text(
                Pos2::new(panel.left() + 44.0, y + 24.0),
                egui::Align2::CENTER_CENTER,
                "L",
                sans(18.0),
                pal.text(),
            );
            p.text(
                Pos2::new(panel.left() + 82.0, y + 6.0),
                egui::Align2::LEFT_TOP,
                "Local profile",
                sans(14.0),
                pal.text(),
            );
            p.text(
                Pos2::new(panel.left() + 82.0, y + 26.0),
                egui::Align2::LEFT_TOP,
                "No sign-in. Flowline runs on this Mac only.",
                sans(11.0),
                pal.text_muted(),
            );
            y += 72.0;
            p.line_segment(
                [
                    Pos2::new(panel.left() + 20.0, y),
                    Pos2::new(panel.right() - 20.0, y),
                ],
                Stroke::new(1.0, pal.border()),
            );
            y += 16.0;
            let engine_label = if app.uses_local_engine() {
                crate::engine::local_model_file(&app.state.settings.local_model)
                    .ok()
                    .and_then(|p| p.file_stem().map(|s| s.to_string_lossy().into_owned()))
                    .map(|m| format!("{m} on this Mac"))
                    .unwrap_or_else(|| "Whisper on this Mac".into())
            } else {
                crate::engine::host_of(&app.state.settings.engine_endpoint)
                    .map(|h| format!("Server at {h}"))
                    .unwrap_or_else(|| "OpenAI-compatible server".into())
            };
            setting_row(ui, pal, panel, &mut y, "Transcription", &engine_label);
            setting_row(
                ui,
                pal,
                panel,
                &mut y,
                "Storage",
                "~/Library/Application Support/Flowline",
            );
            setting_row(
                ui,
                pal,
                panel,
                &mut y,
                "Version",
                concat!("Flowline ", env!("CARGO_PKG_VERSION")),
            );
            let open_btn = Rect::from_min_size(
                Pos2::new(panel.left() + 24.0, y + 4.0),
                Vec2::new(168.0, 34.0),
            );
            if secondary_button(ui, pal, open_btn, "Show data folder", "open-data") {
                if let Some(dir) = crate::model::State::path().and_then(|p| p.parent().map(|d| d.to_path_buf())) {
                    let _ = std::process::Command::new("open").arg(dir).spawn();
                }
            }
        }
        SettingsTab::About => {
            let mut y = panel.top() + 28.0;
            let mark =
                Rect::from_min_size(Pos2::new(panel.left() + 24.0, y), Vec2::new(26.0, 22.0));
            brand_mark(&p, mark, pal.text());
            p.text(
                Pos2::new(mark.right() + 10.0, y + 2.0),
                egui::Align2::LEFT_TOP,
                "Flowline",
                sans(18.0),
                pal.text(),
            );
            y += 44.0;
            p.text(
                Pos2::new(panel.left() + 24.0, y),
                egui::Align2::LEFT_TOP,
                format!("Version {}", env!("CARGO_PKG_VERSION")),
                sans(11.5),
                pal.text_muted(),
            );
            y += 30.0;
            wrapped(ui, Rect::from_min_size(Pos2::new(panel.left() + 24.0, y), Vec2::new(panel.width() - 48.0, 120.0)), "Hold-to-talk dictation for macOS. Audio is transcribed by Whisper on this Mac, or by an OpenAI-compatible server you choose, then pasted into the app you were using. Dictionary, snippets, styles, scratchpad and history are local.", sans(12.0), pal.text_muted());
            y += 96.0;
            p.text(
                Pos2::new(panel.left() + 24.0, y),
                egui::Align2::LEFT_TOP,
                "Built with Rust and egui",
                sans(11.0),
                pal.text_faint(),
            );
            p.text(
                Pos2::new(panel.left() + 24.0, y + 20.0),
                egui::Align2::LEFT_TOP,
                if app.uses_local_engine() {
                    format!(
                        "Audio pipeline: {} Hz mono · Whisper on this Mac · 20 min max",
                        crate::RECORDING_SAMPLE_RATE
                    )
                } else {
                    format!(
                        "Audio pipeline: {} Hz mono · {} · 20 min max",
                        crate::RECORDING_SAMPLE_RATE,
                        crate::engine::requested_model(&app.state.settings.model)
                    )
                },
                sans(10.0),
                pal.text_faint(),
            );
            p.text(Pos2::new(panel.left() + 24.0, y + 44.0), egui::Align2::LEFT_TOP, "Not included: team billing, cloud sync and the global Transforms shortcut.", sans(10.0), pal.text_faint());
        }
    }
}

fn setting_row(ui: &egui::Ui, pal: Palette, panel: Rect, y: &mut f32, title: &str, sub: &str) {
    ui.painter().text(
        Pos2::new(panel.left() + 24.0, *y),
        egui::Align2::LEFT_TOP,
        title,
        sans(12.5),
        pal.text(),
    );
    ui.painter().text(
        Pos2::new(panel.left() + 24.0, *y + 19.0),
        egui::Align2::LEFT_TOP,
        sub,
        sans(10.0),
        pal.text_muted(),
    );
    *y += 58.0;
}

fn chevron_value(ui: &egui::Ui, pal: Palette, panel: Rect, y: f32, value: &str, key: &str) -> bool {
    let hit = Rect::from_min_size(
        Pos2::new(panel.right() - 260.0, y - 40.0),
        Vec2::new(250.0, 32.0),
    );
    let _ = hovered(ui, hit, key);
    let g = ui.fonts(|f| f.layout_no_wrap(value.to_owned(), sans(11.0), pal.text_muted()));
    let x = panel.right() - 34.0 - g.size().x;
    ui.painter()
        .galley(Pos2::new(x, y - 34.0), g, pal.text_muted());
    let c = Pos2::new(panel.right() - 24.0, y - 26.0);
    ui.painter().add(egui::Shape::line(
        vec![
            Pos2::new(c.x - 4.0, c.y - 2.0),
            Pos2::new(c.x, c.y + 2.0),
            Pos2::new(c.x + 4.0, c.y - 2.0),
        ],
        Stroke::new(1.2, pal.text_faint()),
    ));
    click(ui, hit, key)
}

fn modal(ui: &mut egui::Ui, app: &mut App, pal: Palette, full: Rect) {
    let Some(kind) = app.modal.clone() else {
        return;
    };
    ui.painter()
        .rect_filled(full, 0.0, Color32::from_black_alpha(60));
    let _block = ui.interact(full, ui.id().with("modal-block"), egui::Sense::click());
    let p = ui.painter().clone();
    let (title, sub, add_label) = match kind {
        Modal::AddWord => (
            "Add to vocabulary",
            "Words Flowline should always spell correctly.",
            "Add word",
        ),
        Modal::EditWord(_) => (
            "Edit word",
            "Change the word or how it should be written.",
            "Save changes",
        ),
        Modal::DeleteWord(_) => (
            "Delete word?",
            "This word will be deleted permanently.",
            "Yes, delete it",
        ),
        Modal::AddSnippet => (
            "Add snippet",
            "Say a short phrase, get a longer one.",
            "Add snippet",
        ),
        Modal::EditSnippet(_) => (
            "Edit snippet",
            "Change the phrase or its expansion.",
            "Save changes",
        ),
        Modal::DeleteSnippet(_) => (
            "Delete snippet?",
            "This snippet will be deleted permanently.",
            "Yes, delete it",
        ),
        Modal::DeleteHistory => (
            "Delete all dictations?",
            "This deletes every transcript and note insertion from this Mac.",
            "Delete all",
        ),
    };
    let is_delete = matches!(
        kind,
        Modal::DeleteWord(_) | Modal::DeleteSnippet(_) | Modal::DeleteHistory
    );
    let h = if is_delete { 210.0 } else { 300.0 };
    let card = Rect::from_center_size(full.center(), Vec2::new(460.0, h));
    shadow(&p, card, 18);
    p.rect_filled(card, CornerRadius::same(16), pal.card());
    p.text(
        Pos2::new(card.left() + 24.0, card.top() + 22.0),
        egui::Align2::LEFT_TOP,
        title,
        sans(17.0),
        pal.text(),
    );
    p.text(
        Pos2::new(card.left() + 24.0, card.top() + 48.0),
        egui::Align2::LEFT_TOP,
        sub,
        sans(11.0),
        pal.text_muted(),
    );

    if !is_delete {
        field(
            ui,
            pal,
            Rect::from_min_size(
                Pos2::new(card.left() + 24.0, card.top() + 82.0),
                Vec2::new(card.width() - 48.0, 38.0),
            ),
            &mut app.form1,
            "Word or phrase",
            "modal-f1",
        );
        field(
            ui,
            pal,
            Rect::from_min_size(
                Pos2::new(card.left() + 24.0, card.top() + 148.0),
                Vec2::new(card.width() - 48.0, 38.0),
            ),
            &mut app.form2,
            if matches!(kind, Modal::AddSnippet | Modal::EditSnippet(_)) {
                "Expansion"
            } else {
                "Always write it as (optional)"
            },
            "modal-f2",
        );
    }

    let cancel = Rect::from_min_size(
        Pos2::new(card.left() + 24.0, card.bottom() - 58.0),
        Vec2::new(100.0, 36.0),
    );
    if secondary_button(ui, pal, cancel, "Cancel", "modal-cancel") {
        app.modal = None;
    }
    let confirm = Rect::from_min_size(
        Pos2::new(card.right() - 24.0 - 150.0, card.bottom() - 58.0),
        Vec2::new(150.0, 36.0),
    );
    let confirm_label = if is_delete {
        "Yes, delete it"
    } else {
        add_label
    };
    if primary_button(ui, pal, confirm, confirm_label, "modal-confirm") {
        match kind {
            Modal::AddWord => {
                if !app.form1.trim().is_empty() {
                    let replacement = if app.form2.trim().is_empty() {
                        app.form1.trim().to_string()
                    } else {
                        app.form2.trim().to_string()
                    };
                    app.state.dictionary.push(DictEntry {
                        word: app.form1.trim().to_string(),
                        replacement,
                        starred: false,
                        learned: false,
                    });
                    app.toast("Added to dictionary".into());
                }
            }
            Modal::EditWord(i) => {
                if !app.form1.trim().is_empty() {
                    if let Some(d) = app.state.dictionary.get_mut(i) {
                        d.word = app.form1.trim().to_string();
                        d.replacement = if app.form2.trim().is_empty() {
                            d.word.clone()
                        } else {
                            app.form2.trim().to_string()
                        };
                        app.toast("Updated word".into());
                    }
                }
            }
            Modal::DeleteWord(i) => {
                if i < app.state.dictionary.len() {
                    app.state.dictionary.remove(i);
                    app.toast("Deleted word".into());
                }
            }
            Modal::AddSnippet => {
                if app.form1.chars().count() > crate::SNIPPET_TRIGGER_MAX {
                    app.toast(format!(
                        "Snippet must be {} characters or fewer",
                        crate::SNIPPET_TRIGGER_MAX
                    ));
                } else if app.form2.chars().count() > crate::SNIPPET_EXPANSION_MAX {
                    app.toast(format!(
                        "Expansion must be {} characters or fewer",
                        crate::SNIPPET_EXPANSION_MAX
                    ));
                } else if !app.form1.trim().is_empty() && !app.form2.trim().is_empty() {
                    app.state.snippets.push(Snippet {
                        trigger: app.form1.trim().to_string(),
                        expansion: app.form2.trim().to_string(),
                    });
                    app.toast("Added snippet".into());
                }
            }
            Modal::EditSnippet(i) => {
                if app.form1.chars().count() > crate::SNIPPET_TRIGGER_MAX {
                    app.toast(format!(
                        "Snippet must be {} characters or fewer",
                        crate::SNIPPET_TRIGGER_MAX
                    ));
                } else if app.form2.chars().count() > crate::SNIPPET_EXPANSION_MAX {
                    app.toast(format!(
                        "Expansion must be {} characters or fewer",
                        crate::SNIPPET_EXPANSION_MAX
                    ));
                } else if let Some(s) = app.state.snippets.get_mut(i) {
                    if !app.form1.trim().is_empty() {
                        s.trigger = app.form1.trim().to_string();
                        s.expansion = app.form2.trim().to_string();
                        app.toast("Updated snippet".into());
                    }
                }
            }
            Modal::DeleteSnippet(i) => {
                if i < app.state.snippets.len() {
                    app.state.snippets.remove(i);
                    app.toast("Deleted snippet".into());
                }
            }
            Modal::DeleteHistory => {
                app.state.history.clear();
                app.toast("Dictation history deleted".into());
            }
        }
        app.modal = None;
        app.save();
    }
}

fn field(
    ui: &mut egui::Ui,
    pal: Palette,
    rect: Rect,
    value: &mut String,
    hint: &str,
    key: &str,
) -> egui::Response {
    ui.painter().rect(
        rect,
        CornerRadius::same(8),
        pal.card_alt(),
        Stroke::new(1.0, pal.border()),
        egui::StrokeKind::Inside,
    );
    ui.put(
        rect.shrink2(Vec2::new(10.0, 8.0)),
        egui::TextEdit::singleline(value)
            .hint_text(hint)
            .font(sans(12.5))
            .text_color(pal.text())
            .frame(false)
            .margin(Vec2::new(0.0, 0.0))
            .id(egui::Id::new(key)),
    )
}

/// The engine line shown on Home and in Settings.
fn engine_badge(app: &App, pal: Palette) -> (String, Color32) {
    if app.uses_local_engine() {
        let (ready, detail) = crate::engine::local_ready(&app.state.settings.local_model);
        if app.engine_checking {
            return ("Checking…".to_string(), pal.text_muted());
        }
        return if ready {
            let short = detail
                .trim_end_matches(".bin")
                .trim_start_matches("ggml-")
                .to_string();
            (format!("Whisper {short}"), FATHOM_500)
        } else {
            ("Whisper missing".to_string(), RED_450)
        };
    }
    let model = crate::engine::requested_model(&app.state.settings.model);
    let is_speech =
        crate::engine::looks_like_speech(&model) || model == crate::engine::DEFAULT_MODEL;
    let short: String = if model.chars().count() > 26 {
        format!("{}…", model.chars().take(25).collect::<String>())
    } else {
        model
    };
    if app.engine_checking {
        ("Checking…".to_string(), pal.text_muted())
    } else if app.engine.ready() {
        if !crate::engine::has_speech_model(&app.engine.models) && !app.engine.models.is_empty() {
            let label = if crate::engine::has_tts_model(&app.engine.models) {
                "TTS only"
            } else {
                "No speech model"
            };
            return (label.to_string(), GLOW_300);
        }
        // Only the default model is known to be a speech model here; for any
        // other id, say the server has it without claiming it transcribes.
        if is_speech {
            (format!("{short} ready"), FATHOM_500)
        } else {
            (format!("{short} connected"), pal.text_muted())
        }
    } else if app.engine.reachable {
        (format!("{short} not listed"), GLOW_300)
    } else {
        ("Engine offline".to_string(), RED_450)
    }
}

pub(crate) fn rounded_card(ui: &egui::Ui, pal: Palette, rect: Rect, radius: u8) {
    ui.painter().rect(
        rect,
        CornerRadius::same(radius),
        pal.card(),
        Stroke::new(1.0, pal.border()),
        egui::StrokeKind::Inside,
    );
}

fn key_cap(ui: &egui::Ui, pal: Palette, rect: Rect, label: &str) {
    ui.painter().rect(
        rect,
        CornerRadius::same(6),
        pal.card_alt(),
        Stroke::new(1.0, pal.border()),
        egui::StrokeKind::Inside,
    );
    ui.painter().text(
        rect.center(),
        egui::Align2::CENTER_CENTER,
        label,
        sans(10.5),
        pal.text_muted(),
    );
}

fn brand_mark(p: &egui::Painter, rect: Rect, color: Color32) {
    let n = 7;
    let gap = rect.width() / n as f32;
    let heights = [0.35, 0.65, 0.95, 1.0, 0.8, 0.55, 0.3];
    for i in 0..n {
        let x = rect.left() + gap * (i as f32 + 0.5);
        let h = rect.height() * heights[i] * 0.5;
        p.line_segment(
            [
                Pos2::new(x, rect.center().y - h),
                Pos2::new(x, rect.center().y + h),
            ],
            Stroke::new(gap * 0.34, color),
        );
    }
}

fn measure(ui: &egui::Ui, text: &str, size: f32) -> f32 {
    ui.fonts(|f| {
        f.layout_no_wrap(text.to_owned(), sans(size), Color32::WHITE)
            .size()
            .x
    })
}

/// Native file picker through AppleScript, so no extra crate is needed.
fn pick_wav() -> Option<std::path::PathBuf> {
    let script = r#"POSIX path of (choose file with prompt "Choose a WAV recording to transcribe" of type {"public.wav"})"#;
    let out = std::process::Command::new("osascript")
        .args(["-e", script])
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    let path = String::from_utf8_lossy(&out.stdout).trim().to_string();
    if path.is_empty() {
        None
    } else {
        Some(std::path::PathBuf::from(path))
    }
}

// ---------------------------------------------------------------------------
// Real statistics (no placeholder numbers anywhere)
// ---------------------------------------------------------------------------

use chrono::{Datelike, Local, NaiveDate, TimeZone};
use std::collections::HashSet;

struct WeekStats {
    words: usize,
    seconds: f32,
}

impl WeekStats {
    fn wpm(&self) -> i64 {
        if self.seconds <= 0.0 {
            0
        } else {
            (self.words as f32 / (self.seconds / 60.0)).round() as i64
        }
    }
    /// Speaking at ~40 wpm vs typing.
    fn minutes_saved(&self) -> i64 {
        (((self.words as f32 / 40.0 * 60.0) - self.seconds).max(0.0) / 60.0).round() as i64
    }
}

fn day_start(day: NaiveDate) -> i64 {
    Local
        .from_local_datetime(&day.and_hms_opt(0, 0, 0).unwrap())
        .single()
        .map(|d| d.timestamp())
        .unwrap_or(0)
}

fn local_day(ts: i64) -> Option<NaiveDate> {
    Local.timestamp_opt(ts, 0).single().map(|d| d.date_naive())
}

fn week_stats(history: &[Dictation]) -> WeekStats {
    let today = Local::now().date_naive();
    let start = day_start(today - chrono::Duration::days(6));
    let mut words = 0usize;
    let mut seconds = 0.0f32;
    for d in history {
        if d.ts >= start {
            words += d.words;
            seconds += d.seconds;
        }
    }
    WeekStats { words, seconds }
}

/// Words spoken on each of the last seven local days, oldest first.
fn daily_words(history: &[Dictation]) -> Vec<(String, usize)> {
    let today = Local::now().date_naive();
    (0..7)
        .rev()
        .map(|i| {
            let day = today - chrono::Duration::days(i);
            let start = day_start(day);
            let end = day_start(day + chrono::Duration::days(1));
            let words = history
                .iter()
                .filter(|d| d.ts >= start && d.ts < end)
                .map(|d| d.words)
                .sum();
            let label = match day.weekday() {
                chrono::Weekday::Mon => "Mon",
                chrono::Weekday::Tue => "Tue",
                chrono::Weekday::Wed => "Wed",
                chrono::Weekday::Thu => "Thu",
                chrono::Weekday::Fri => "Fri",
                chrono::Weekday::Sat => "Sat",
                chrono::Weekday::Sun => "Sun",
            };
            (label.to_string(), words)
        })
        .collect()
}

/// Consecutive days with at least one dictation, ending today (or yesterday if
/// today has none yet).
fn day_streak(history: &[Dictation]) -> i64 {
    let days: HashSet<NaiveDate> = history
        .iter()
        .filter(|d| d.ts > 0)
        .filter_map(|d| local_day(d.ts))
        .collect();
    if days.is_empty() {
        return 0;
    }
    let mut day = Local::now().date_naive();
    if !days.contains(&day) {
        day = day - chrono::Duration::days(1);
    }
    let mut streak = 0;
    while days.contains(&day) {
        streak += 1;
        day = day - chrono::Duration::days(1);
    }
    streak
}

/// First-run gate: global dictation and automatic pasting both need macOS
/// Accessibility. Shown until granted, and reopenable from the menu bar.
pub fn permissions_sheet(ui: &mut egui::Ui, app: &mut App, pal: Palette) {
    let full = ui.max_rect();
    let p = ui.painter().clone();
    p.rect_filled(full, 0.0, pal.canvas());
    let card = Rect::from_center_size(full.center(), Vec2::new(620.0, 486.0));
    shadow(&p, card, 28);
    rounded_card(ui, pal, card, 20);

    p.text(
        Pos2::new(card.left() + 32.0, card.top() + 30.0),
        egui::Align2::LEFT_TOP,
        "Let Flowline type for you",
        sans(24.0),
        pal.text(),
    );
    wrapped(
        ui,
        Rect::from_min_size(
            Pos2::new(card.left() + 32.0, card.top() + 68.0),
            Vec2::new(card.width() - 64.0, 54.0),
        ),
        "Hold Fn in any app and speak. Flowline needs Accessibility to see the shortcut and to press ⌘V in the app you are using; Microphone is only open while you hold it.",
        sans(11.5),
        pal.text_muted(),
    );

    let mic = crate::platform::mic_status();
    let rows = [
        (
            "Microphone",
            "Records only while you hold the shortcut.",
            mic == 3,
        ),
        (
            "Accessibility",
            "Catch Fn anywhere and paste the transcript into the frontmost field.",
            crate::platform::accessibility_granted(),
        ),
        (
            "Input Monitoring",
            "Optional. Not needed when Accessibility is allowed.",
            crate::platform::input_monitoring_granted(),
        ),
    ];
    let mut y = card.top() + 136.0;
    for (title, sub, ok) in rows {
        p.circle_filled(
            Pos2::new(card.left() + 40.0, y + 16.0),
            4.0,
            if ok { FATHOM_500 } else { GLOW_300 },
        );
        p.text(
            Pos2::new(card.left() + 56.0, y),
            egui::Align2::LEFT_TOP,
            title,
            sans(13.0),
            pal.text(),
        );
        wrapped(
            ui,
            Rect::from_min_size(
                Pos2::new(card.left() + 56.0, y + 19.0),
                Vec2::new(card.width() - 190.0, 34.0),
            ),
            sub,
            sans(10.0),
            pal.text_muted(),
        );
        p.text(
            Pos2::new(card.right() - 32.0, y + 2.0),
            egui::Align2::RIGHT_TOP,
            if ok {
                "Allowed"
            } else if title == "Microphone" && mic == 0 {
                "Ask on first use"
            } else if title == "Input Monitoring" {
                "Optional"
            } else {
                "Missing"
            },
            sans(10.5),
            if ok { FATHOM_500 } else { GLOW_300 },
        );
        y += 66.0;
    }

    let by = card.bottom() - 66.0;
    let open_ax = Rect::from_min_size(Pos2::new(card.left() + 32.0, by), Vec2::new(196.0, 38.0));
    if primary_button(ui, pal, open_ax, "Open Accessibility", "perm-open-ax") {
        crate::platform::request_accessibility();
        crate::platform::open_privacy_pane("Accessibility");
    }
    let open_im = Rect::from_min_size(Pos2::new(card.left() + 236.0, by), Vec2::new(196.0, 38.0));
    if secondary_button(ui, pal, open_im, "Open Input Monitoring", "perm-open-im") {
        crate::platform::request_input_monitoring();
        crate::platform::open_privacy_pane("ListenEvent");
    }
    let check = Rect::from_min_size(Pos2::new(card.left() + 440.0, by), Vec2::new(148.0, 38.0));
    if secondary_button(ui, pal, check, "Check again", "perm-check") {
        if crate::platform::accessibility_granted() {
            app.show_permissions = false;
            app.toast("Accessibility enabled — dictate anywhere with Fn".into());
        } else {
            app.toast("macOS has not granted Accessibility yet".into());
        }
    }
    let later = Rect::from_min_size(Pos2::new(card.left() + 32.0, card.bottom() - 22.0), Vec2::new(120.0, 18.0));
    if click(ui, later, "perm-later") {
        app.show_permissions = false;
    }
    p.text(
        Pos2::new(card.left() + 32.0, card.bottom() - 24.0),
        egui::Align2::LEFT_TOP,
        "Later — only in-app dictation works.",
        sans(9.5),
        pal.text_faint(),
    );
    let restart = Rect::from_min_size(
        Pos2::new(card.right() - 150.0, card.bottom() - 28.0),
        Vec2::new(118.0, 20.0),
    );
    if click(ui, restart, "perm-restart") {
        crate::platform::restart_bundle();
    }
    p.text(
        Pos2::new(card.right() - 32.0, card.bottom() - 24.0),
        egui::Align2::RIGHT_TOP,
        "Restart Flowline",
        sans(9.5),
        pal.text(),
    );
}
