//! The Transforms and Notetaker hub pages.

use crate::dictation::mmss;
use crate::features::TransformEdit;
use crate::hub::{header, rounded_card};
use crate::model::*;
use crate::notes::*;
use crate::theme::*;
use crate::ui::*;
use crate::App;
use eframe::egui::{self, CornerRadius, Frame, Margin, Pos2, Rect, RichText, Sense, Stroke, Vec2};

const PAD: f32 = 40.0;

// ---- small widgets ---------------------------------------------------------

fn btn(ui: &mut egui::Ui, pal: Palette, label: &str, w: f32, primary: bool, key: &str) -> bool {
    let (rect, _) = ui.allocate_exact_size(Vec2::new(w, 34.0), Sense::hover());
    if primary {
        primary_button(ui, pal, rect, label, key)
    } else {
        secondary_button(ui, pal, rect, label, key)
    }
}

fn chip(ui: &mut egui::Ui, pal: Palette, label: &str, on: bool, key: &str) -> bool {
    let w = ui.fonts(|f| f.layout_no_wrap(label.to_owned(), sans(12.5), pal.text()).size().x) + 26.0;
    let (rect, _) = ui.allocate_exact_size(Vec2::new(w, 30.0), Sense::hover());
    let hov = hovered(ui, rect, key);
    let fill = if on {
        pal.accent()
    } else if hov {
        pal.hover()
    } else {
        pal.card()
    };
    bordered(ui.painter(), rect, 15, fill, pal.border());
    text_center(
        ui,
        rect.center(),
        label,
        sans(12.5),
        if on { pal.accent_text() } else { pal.text() },
    );
    click(ui, rect, key)
}

fn switch(ui: &mut egui::Ui, pal: Palette, on: bool, key: &str) -> bool {
    let (rect, _) = ui.allocate_exact_size(Vec2::new(44.0, 24.0), Sense::hover());
    toggle(ui, pal, rect, on, key)
}

fn card<R>(ui: &mut egui::Ui, pal: Palette, add: impl FnOnce(&mut egui::Ui) -> R) -> R {
    Frame::new()
        .fill(pal.card())
        .stroke(Stroke::new(1.0, pal.border()))
        .corner_radius(CornerRadius::same(16))
        .inner_margin(Margin::same(20))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            add(ui)
        })
        .inner
}

fn title(ui: &mut egui::Ui, pal: Palette, s: &str) {
    ui.label(RichText::new(s).font(sans(15.0)).color(pal.text()));
}

fn muted(ui: &mut egui::Ui, pal: Palette, s: &str) {
    ui.label(RichText::new(s).font(sans(12.0)).color(pal.text_muted()));
}

fn faint(ui: &mut egui::Ui, pal: Palette, s: &str) {
    ui.label(RichText::new(s).font(sans(11.0)).color(pal.text_faint()));
}

fn problem(ui: &mut egui::Ui, s: &str) {
    ui.label(RichText::new(s).font(sans(12.0)).color(RED_450));
}

fn field(ui: &mut egui::Ui, pal: Palette, value: &mut String, w: f32, hint: &str, id: &str) -> bool {
    let mut changed = false;
    Frame::new()
        .fill(pal.card_alt())
        .stroke(Stroke::new(1.0, pal.border()))
        .corner_radius(CornerRadius::same(9))
        .inner_margin(Margin::symmetric(10, 7))
        .show(ui, |ui| {
            let r = ui.add(
                egui::TextEdit::singleline(value)
                    .hint_text(hint)
                    .font(sans(13.0))
                    .text_color(pal.text())
                    .frame(false)
                    .desired_width(w - 20.0)
                    .id(egui::Id::new(id)),
            );
            changed = r.changed();
        });
    changed
}

fn area(ui: &mut egui::Ui, pal: Palette, value: &mut String, rows: usize, hint: &str, id: &str) -> bool {
    let mut changed = false;
    Frame::new()
        .fill(pal.card_alt())
        .stroke(Stroke::new(1.0, pal.border()))
        .corner_radius(CornerRadius::same(10))
        .inner_margin(Margin::same(10))
        .show(ui, |ui| {
            let r = ui.add(
                egui::TextEdit::multiline(value)
                    .hint_text(hint)
                    .font(sans(13.0))
                    .text_color(pal.text())
                    .frame(false)
                    .desired_rows(rows)
                    .desired_width(ui.available_width())
                    .id(egui::Id::new(id)),
            );
            changed = r.changed();
        });
    changed
}

fn preview(s: &str, n: usize) -> String {
    let one: String = s.split_whitespace().collect::<Vec<_>>().join(" ");
    if one.chars().count() > n {
        format!("{}…", one.chars().take(n).collect::<String>())
    } else {
        one
    }
}

// ---- Transforms ------------------------------------------------------------

enum Act {
    Edit(usize),
    Delete(usize),
    New,
}

pub fn transforms(ui: &mut egui::Ui, app: &mut App, pal: Palette, content: Rect) {
    let x = content.left() + PAD;
    let right = content.right() - PAD;
    let top = header(
        ui,
        pal,
        content,
        "Transforms",
        "Rewrite text with a prompt you choose, using a text model you control.",
    );
    label_tag(ui, pal, Pos2::new(x + 148.0, content.top() + 52.0), "Beta");

    let rect = Rect::from_min_max(Pos2::new(x, top), Pos2::new(right, content.bottom() - 12.0));
    let mut child = ui.new_child(egui::UiBuilder::new().max_rect(rect));
    egui::ScrollArea::vertical()
        .auto_shrink([false, false])
        .id_salt("transforms-scroll")
        .show(&mut child, |ui| {
            ui.set_width(rect.width() - 14.0);

            // Text model
            card(ui, pal, |ui| {
                title(ui, pal, "Text model");
                muted(
                    ui,
                    pal,
                    "Rewrites and note summaries come from a chat model on a server you choose \
                     (Ollama, LM Studio, llama.cpp). Text goes only to the endpoint below.",
                );
                ui.add_space(6.0);
                ui.horizontal(|ui| {
                    faint(ui, pal, "Endpoint");
                    let mut ep = app.state.text.endpoint.clone();
                    if field(ui, pal, &mut ep, 300.0, "http://127.0.0.1:11434/v1", "text-endpoint") {
                        app.state.text.endpoint = ep;
                        app.save();
                    }
                    faint(ui, pal, "Model");
                    let mut m = app.state.text.model.clone();
                    if field(ui, pal, &mut m, 220.0, "model id", "text-model") {
                        app.state.text.model = m;
                        app.save();
                    }
                    let label = if app.ext.models_busy { "Checking…" } else { "Detect models" };
                    if btn(ui, pal, label, 130.0, false, "tf-detect") {
                        app.detect_text_models();
                    }
                });
                if !app.ext.models_status.is_empty() {
                    if app.ext.models_status.starts_with("Could not") {
                        problem(ui, &app.ext.models_status.clone());
                    } else {
                        faint(ui, pal, &app.ext.models_status.clone());
                    }
                }
                if !app.ext.models.is_empty() {
                    ui.horizontal_wrapped(|ui| {
                        for (i, m) in app.ext.models.clone().iter().enumerate() {
                            let on = *m == app.state.text.model;
                            if chip(ui, pal, m, on, &format!("text-model-{i}")) {
                                app.state.text.model = m.clone();
                                app.save();
                            }
                        }
                    });
                }
                if !app.state.text_model_ready() {
                    ui.add_space(2.0);
                    muted(ui, pal, "Press Detect models, or type a model id, to turn this on.");
                }
            });
            ui.add_space(14.0);

            // Opt in
            card(ui, pal, |ui| {
                ui.horizontal(|ui| {
                    ui.vertical(|ui| {
                        title(ui, pal, "Use Transforms");
                        ui.set_max_width(ui.available_width() - 70.0);
                        muted(
                            ui,
                            pal,
                            &format!(
                                "Highlight text in any app, then press {} for Polish, {} for Prompt \
                                 Engineer, {} for your own. The rewrite replaces the selection.",
                                crate::keys::transform_key(1),
                                crate::keys::transform_key(2),
                                crate::keys::transform_custom_range(),
                            ),
                        );
                    });
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if switch(ui, pal, app.state.transforms.enabled, "tf-enabled") {
                            app.state.transforms.enabled = !app.state.transforms.enabled;
                            app.save();
                        }
                    });
                });
            });
            ui.add_space(14.0);
            if !app.state.transforms.enabled {
                faint(ui, pal, "Turn Transforms on to edit prompts and try them.");
                return;
            }

            // Command mode
            card(ui, pal, |ui| {
                ui.horizontal(|ui| {
                    ui.vertical(|ui| {
                        title(ui, pal, "Command mode");
                        ui.set_max_width(ui.available_width() - 70.0);
                        muted(
                            ui,
                            pal,
                            &format!(
                                "Select text in any app, hold {} and say what to do, such as \
                                 \"make this shorter\" or \"translate to French\", then release. The result \
                                 replaces the selection.",
                                crate::keys::command_key()
                            ),
                        );
                    });
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if switch(ui, pal, app.state.transforms.command, "tf-command") {
                            app.state.transforms.command = !app.state.transforms.command;
                            app.save();
                        }
                    });
                });
                if app.state.transforms.command && !app.state.text_model_ready() {
                    problem(ui, "Choose a text model above, or command mode stays off.");
                }
            });
            ui.add_space(14.0);

            // Auto apply
            card(ui, pal, |ui| {
                ui.horizontal(|ui| {
                    ui.vertical(|ui| {
                        title(ui, pal, "Auto apply after dictation");
                        ui.set_max_width(ui.available_width() - 70.0);
                        muted(
                            ui,
                            pal,
                            "Run one transform on every dictation before it is pasted. If it \
                             fails or times out, the text is pasted as dictated.",
                        );
                    });
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if switch(ui, pal, app.state.transforms.auto, "tf-auto") {
                            app.state.transforms.auto = !app.state.transforms.auto;
                            if app.state.transforms.auto_id.is_empty() {
                                app.state.transforms.auto_id = "polish".into();
                            }
                            app.save();
                        }
                    });
                });
                if app.state.transforms.auto {
                    ui.add_space(6.0);
                    ui.horizontal_wrapped(|ui| {
                        for (i, t) in app.state.all_transforms().iter().enumerate() {
                            let on = app.state.transforms.auto_id == t.id;
                            if chip(ui, pal, &t.name, on, &format!("tf-auto-{i}")) {
                                app.state.transforms.auto_id = t.id.clone();
                                app.save();
                            }
                        }
                    });
                    if !app.state.text_model_ready() {
                        problem(ui, "Choose a text model above, or auto apply is skipped.");
                    }
                }
            });
            ui.add_space(14.0);

            // My transforms
            let mut act: Option<Act> = None;
            card(ui, pal, |ui| {
                ui.horizontal(|ui| {
                    title(ui, pal, "My transforms");
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        let used = app.state.transforms.custom.len();
                        if used >= MAX_CUSTOM_TRANSFORMS {
                            faint(ui, pal, &format!("{used} of {MAX_CUSTOM_TRANSFORMS} custom used"));
                        } else if btn(ui, pal, "New transform", 130.0, false, "tf-new") {
                            act = Some(Act::New);
                        }
                    });
                });
                ui.add_space(4.0);
                let builtin_count = builtin_transforms().len();
                for (i, t) in app.state.all_transforms().iter().enumerate() {
                    ui.separator();
                    ui.horizontal(|ui| {
                        ui.vertical(|ui| {
                            ui.horizontal(|ui| {
                                ui.label(RichText::new(&t.name).font(sans(14.0)).color(pal.text()));
                                if t.builtin {
                                    faint(ui, pal, "Built-in");
                                }
                                if i < 9 {
                                    faint(ui, pal, &crate::keys::transform_key(i + 1));
                                }
                            });
                            ui.set_max_width(ui.available_width() - 190.0);
                            faint(ui, pal, &preview(&t.prompt, 150));
                        });
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if !t.builtin {
                                let ci = i - builtin_count;
                                if btn(ui, pal, "Delete", 80.0, false, &format!("tf-del-{i}")) {
                                    act = Some(Act::Delete(ci));
                                }
                                if btn(ui, pal, "Edit", 70.0, false, &format!("tf-edit-{i}")) {
                                    act = Some(Act::Edit(ci));
                                }
                            }
                        });
                    });
                }
            });
            match act {
                Some(Act::New) => {
                    app.ext.tf_edit = Some(TransformEdit {
                        index: None,
                        name: String::new(),
                        prompt: String::new(),
                    });
                }
                Some(Act::Edit(ci)) => {
                    if let Some(t) = app.state.transforms.custom.get(ci) {
                        app.ext.tf_edit = Some(TransformEdit {
                            index: Some(ci),
                            name: t.name.clone(),
                            prompt: t.prompt.clone(),
                        });
                    }
                }
                Some(Act::Delete(ci)) => {
                    if ci < app.state.transforms.custom.len() {
                        let gone = app.state.transforms.custom.remove(ci);
                        if app.state.transforms.auto_id == gone.id {
                            app.state.transforms.auto_id = "polish".into();
                        }
                        if app.ext.tf_sel == gone.id {
                            app.ext.tf_sel = "polish".into();
                        }
                        app.ext.tf_edit = None;
                        app.save();
                    }
                }
                None => {}
            }
            ui.add_space(14.0);

            // Editor
            if app.ext.tf_edit.is_some() {
                let mut save = false;
                let mut cancel = false;
                let mut edit = app.ext.tf_edit.take().unwrap();
                card(ui, pal, |ui| {
                    title(
                        ui,
                        pal,
                        if edit.index.is_some() { "Edit transform" } else { "New transform" },
                    );
                    ui.add_space(4.0);
                    field(ui, pal, &mut edit.name, 320.0, "Name, for example Friendly reply", "tf-edit-name");
                    ui.add_space(6.0);
                    area(
                        ui,
                        pal,
                        &mut edit.prompt,
                        5,
                        "Tell the model how to rewrite the text. It receives the text as the message.",
                        "tf-edit-prompt",
                    );
                    ui.add_space(6.0);
                    ui.horizontal(|ui| {
                        let ok = !edit.name.trim().is_empty() && !edit.prompt.trim().is_empty();
                        if btn(ui, pal, "Save", 90.0, ok, "tf-edit-save") && ok {
                            save = true;
                        }
                        if btn(ui, pal, "Cancel", 90.0, false, "tf-edit-cancel") {
                            cancel = true;
                        }
                        if !ok {
                            faint(ui, pal, "A name and a prompt are both needed.");
                        }
                    });
                });
                ui.add_space(14.0);
                if save {
                    let name = edit.name.trim().to_string();
                    let prompt = edit.prompt.trim().to_string();
                    match edit.index {
                        Some(ci) => {
                            if let Some(t) = app.state.transforms.custom.get_mut(ci) {
                                t.name = name;
                                t.prompt = prompt;
                            }
                        }
                        None => {
                            let id = format!("custom-{}", chrono::Local::now().timestamp_millis());
                            app.ext.tf_sel = id.clone();
                            app.state.transforms.custom.push(Transform {
                                id,
                                name,
                                prompt,
                                builtin: false,
                            });
                        }
                    }
                    app.save();
                } else if !cancel {
                    app.ext.tf_edit = Some(edit);
                }
            }

            // Try it
            card(ui, pal, |ui| {
                title(ui, pal, "Try it");
                muted(ui, pal, "Paste text, pick a transform, and see the rewrite.");
                ui.add_space(4.0);
                ui.horizontal_wrapped(|ui| {
                    for (i, t) in app.state.all_transforms().iter().enumerate() {
                        let on = app.ext.tf_sel == t.id;
                        if chip(ui, pal, &t.name, on, &format!("tf-pick-{i}")) {
                            app.ext.tf_sel = t.id.clone();
                        }
                    }
                });
                ui.add_space(6.0);
                let mut input = std::mem::take(&mut app.ext.tf_input);
                area(ui, pal, &mut input, 5, "Text to rewrite", "tf-input");
                app.ext.tf_input = input;
                ui.add_space(6.0);
                ui.horizontal(|ui| {
                    let label = if app.ext.tf_busy { "Working…" } else { "Apply transform" };
                    if btn(ui, pal, label, 150.0, true, "tf-apply") {
                        app.run_transform();
                    }
                    if !app.state.text_model_ready() {
                        faint(ui, pal, "Choose a text model first.");
                    }
                });
                if let Some(e) = app.ext.tf_error.clone() {
                    problem(ui, &e);
                }
                if !app.ext.tf_output.is_empty() {
                    ui.add_space(6.0);
                    let mut out = std::mem::take(&mut app.ext.tf_output);
                    area(ui, pal, &mut out, 5, "", "tf-output");
                    app.ext.tf_output = out;
                    ui.add_space(6.0);
                    ui.horizontal(|ui| {
                        if btn(ui, pal, "Copy result", 120.0, false, "tf-copy") {
                            let text = app.ext.tf_output.clone();
                            app.copy_to_clipboard(&text, "Result");
                        }
                        if btn(ui, pal, "Use as input", 120.0, false, "tf-reuse") {
                            app.ext.tf_input = app.ext.tf_output.clone();
                            app.ext.tf_output.clear();
                        }
                    });
                }
            });
            ui.add_space(24.0);
        });
}

// ---- Notetaker -------------------------------------------------------------

fn when(ts: i64) -> String {
    use chrono::TimeZone;
    chrono::Local
        .timestamp_opt(ts, 0)
        .single()
        .map(|d| d.format("%b %-d, %-I:%M %p").to_string())
        .unwrap_or_default()
}

fn status_label(n: &Note) -> String {
    match n.status {
        NoteStatus::Recording => "Recording".into(),
        NoteStatus::Transcribing => "Transcribing…".into(),
        NoteStatus::Summarizing => "Summarizing…".into(),
        NoteStatus::Done => format!("{} · {}", when(n.created), mmss(n.seconds)),
    }
}

pub fn notetaker(ui: &mut egui::Ui, app: &mut App, pal: Palette, content: Rect) {
    let x = content.left() + PAD;
    let right = content.right() - PAD;
    header(
        ui,
        pal,
        content,
        "Notetaker",
        "Record a conversation, then get a transcript and notes.",
    );

    // New / stop
    let recording = app.ext.nt_rec.is_some();
    let action = Rect::from_min_size(
        Pos2::new(right - 150.0, content.top() + 46.0),
        Vec2::new(150.0, 34.0),
    );
    if recording {
        if primary_button(ui, pal, action, "Stop recording", "nt-stop") {
            app.stop_note();
        }
    } else if primary_button(ui, pal, action, "New note", "nt-new") {
        app.start_note();
    }

    // Meeting auto-detect: opt-in, local only.
    let tg = Rect::from_min_size(
        Pos2::new(action.left() - 16.0 - 44.0, content.top() + 51.0),
        Vec2::new(44.0, 24.0),
    );
    if toggle(ui, pal, tg, app.state.notetaker.auto_detect, "nt-auto") {
        app.state.notetaker.auto_detect = !app.state.notetaker.auto_detect;
        app.save();
    }
    text_right(
        ui,
        Pos2::new(tg.left() - 12.0, content.top() + 56.0),
        "Offer a note when a meeting starts",
        sans(12.0),
        pal.text_muted(),
    );
    let ignored = app.state.notetaker.ignored.len();
    if app.state.notetaker.auto_detect && ignored > 0 {
        let r = Rect::from_min_size(
            Pos2::new(tg.right() - 190.0, content.top() + 88.0),
            Vec2::new(190.0, 26.0),
        );
        if secondary_button(ui, pal, r, &format!("Reset ignored apps ({ignored})"), "nt-reset-ignored") {
            app.state.notetaker.ignored.clear();
            app.ext.prompted = None;
            app.save();
        }
    }

    let list_rect = Rect::from_min_max(
        Pos2::new(x, content.top() + 128.0),
        Pos2::new(x + 290.0, content.bottom() - 24.0),
    );
    let detail = Rect::from_min_max(
        Pos2::new(list_rect.right() + 16.0, list_rect.top()),
        Pos2::new(right, list_rect.bottom()),
    );
    rounded_card(ui, pal, list_rect, 16);
    rounded_card(ui, pal, detail, 16);

    // Keep a valid selection.
    if app.ext.nt_sel.map(|id| app.notes.get(id).is_none()).unwrap_or(false) {
        app.ext.nt_sel = None;
    }
    if app.ext.nt_sel.is_none() {
        app.ext.nt_sel = app.notes.notes.first().map(|n| n.id);
    }

    // List
    let inner = list_rect.shrink(8.0);
    let mut child = ui.new_child(egui::UiBuilder::new().max_rect(inner));
    if app.notes.notes.is_empty() {
        child.add_space(12.0);
        child.set_width(inner.width());
        child.vertical_centered(|ui| {
            faint(ui, pal, "No notes yet");
        });
    } else {
        egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .id_salt("nt-list")
            .show(&mut child, |ui| {
                ui.set_width(inner.width() - 8.0);
                let rows: Vec<(u64, String, String)> = app
                    .notes
                    .notes
                    .iter()
                    .map(|n| (n.id, n.title.clone(), status_label(n)))
                    .collect();
                for (id, name, sub) in rows {
                    let (r, resp) =
                        ui.allocate_exact_size(Vec2::new(ui.available_width(), 56.0), Sense::click());
                    let on = app.ext.nt_sel == Some(id);
                    if on {
                        ui.painter().rect_filled(r, CornerRadius::same(10), pal.selection());
                    } else if resp.hovered() {
                        ui.painter().rect_filled(r, CornerRadius::same(10), pal.hover());
                    }
                    let clipped = ui.painter().with_clip_rect(r.shrink2(Vec2::new(10.0, 0.0)));
                    clipped.text(
                        Pos2::new(r.left() + 12.0, r.top() + 10.0),
                        egui::Align2::LEFT_TOP,
                        &name,
                        sans(13.0),
                        pal.text(),
                    );
                    clipped.text(
                        Pos2::new(r.left() + 12.0, r.top() + 32.0),
                        egui::Align2::LEFT_TOP,
                        &sub,
                        sans(10.5),
                        if sub.starts_with("Recording") { RED_450 } else { pal.text_faint() },
                    );
                    if resp.clicked() {
                        app.ext.nt_sel = Some(id);
                        app.ext.nt_confirm_delete = None;
                    }
                }
            });
    }

    // Detail
    let Some(id) = app.ext.nt_sel else {
        let c = detail.center();
        text_center(ui, Pos2::new(c.x, c.y - 40.0), "Record your first note", sans(18.0), pal.text());
        let wrap = Rect::from_center_size(Pos2::new(c.x, c.y + 4.0), Vec2::new(440.0, 50.0));
        wrapped(
            ui,
            wrap,
            "Flowline listens through your microphone, transcribes on this Mac, and writes \
             the notes with the text model you chose on the Transforms page.",
            sans(12.5),
            pal.text_muted(),
        );
        return;
    };
    let Some(note) = app.notes.get(id).cloned() else { return };

    let dx = detail.left() + 24.0;
    let dr = detail.right() - 24.0;

    // title + actions
    let mut title_text = note.title.clone();
    let title_rect = Rect::from_min_size(
        Pos2::new(dx, detail.top() + 16.0),
        Vec2::new((dr - dx) - 190.0, 30.0),
    );
    let tr = ui.put(
        title_rect,
        egui::TextEdit::singleline(&mut title_text)
            .font(sans(18.0))
            .text_color(pal.text())
            .frame(false)
            .id(egui::Id::new(("nt-title", id))),
    );
    if tr.changed() {
        if let Some(n) = app.notes.get_mut(id) {
            n.title = title_text;
        }
        app.notes.save();
    }
    let copy_rect = Rect::from_min_size(Pos2::new(dr - 180.0, detail.top() + 14.0), Vec2::new(84.0, 32.0));
    let del_rect = Rect::from_min_size(Pos2::new(dr - 90.0, detail.top() + 14.0), Vec2::new(90.0, 32.0));
    let mut copy_text: Option<String> = None;
    if secondary_button(ui, pal, copy_rect, "Copy", "nt-copy") {
        let mut all = format!("{}\n\n", note.title);
        if !note.summary.is_empty() {
            all.push_str(&note.summary);
            all.push_str("\n\n");
        }
        if !note.notes.trim().is_empty() {
            all.push_str(&format!("Notes:\n{}\n\n", note.notes.trim()));
        }
        if note.has_transcript() {
            all.push_str(&format!("Transcript:\n{}\n", note.transcript()));
        }
        copy_text = Some(all);
    }
    if !note.is_busy() {
        if app.ext.nt_confirm_delete == Some(id) {
            if primary_button(ui, pal, del_rect, "Delete?", "nt-del-yes") {
                app.delete_note(id);
                app.ext.nt_confirm_delete = None;
                return;
            }
        } else if secondary_button(ui, pal, del_rect, "Delete", "nt-del") {
            app.ext.nt_confirm_delete = Some(id);
        }
    }
    if let Some(t) = copy_text {
        app.copy_to_clipboard(&t, "Note");
    }

    // status
    let live = app
        .ext
        .nt_rec
        .as_ref()
        .filter(|r| r.id == id)
        .map(|r| r.started.elapsed().as_secs_f32());
    let status = match live {
        Some(s) => format!("● Recording {} · transcript updates every {} s", mmss(s), SEGMENT_SECS as u32),
        None => status_label(&note),
    };
    ui.painter().text(
        Pos2::new(dx, detail.top() + 54.0),
        egui::Align2::LEFT_TOP,
        status,
        sans(11.5),
        if live.is_some() { RED_450 } else { pal.text_faint() },
    );
    let mut body_top = detail.top() + 84.0;
    if !note.error.is_empty() {
        let r = Rect::from_min_size(Pos2::new(dx, body_top - 4.0), Vec2::new(dr - dx, 30.0));
        wrapped(ui, r, &note.error, sans(11.5), RED_450);
        body_top += 30.0;
    }

    // tabs
    let tabs = ["Summary", "Notes", "Transcript"];
    let mut tx = dx;
    for (i, t) in tabs.iter().enumerate() {
        let w = ui.fonts(|f| f.layout_no_wrap((*t).to_owned(), sans(12.5), pal.text()).size().x) + 26.0;
        let r = Rect::from_min_size(Pos2::new(tx, body_top), Vec2::new(w, 30.0));
        let on = app.ext.nt_tab == i;
        let hov = hovered(ui, r, &format!("nt-tab-{i}"));
        bordered(
            ui.painter(),
            r,
            15,
            if on { pal.accent() } else if hov { pal.hover() } else { pal.card() },
            pal.border(),
        );
        text_center(
            ui,
            r.center(),
            t,
            sans(12.5),
            if on { pal.accent_text() } else { pal.text() },
        );
        if click(ui, r, &format!("nt-tab-{i}")) {
            app.ext.nt_tab = i;
        }
        tx += w + 8.0;
    }

    let body = Rect::from_min_max(
        Pos2::new(dx, body_top + 42.0),
        Pos2::new(dr, detail.bottom() - 20.0),
    );
    let tab = app.ext.nt_tab;

    // Summary action row
    let mut body = body;
    if tab == 0 {
        let busy = note.is_busy();
        let r = Rect::from_min_size(Pos2::new(dr - 150.0, body_top - 1.0), Vec2::new(150.0, 32.0));
        let label = if note.status == NoteStatus::Summarizing {
            "Summarizing…"
        } else if note.summary.is_empty() {
            "Summarize"
        } else {
            "Regenerate"
        };
        if secondary_button(ui, pal, r, label, "nt-sum") && !busy {
            app.summarize_note(id);
        }
    }
    body.set_bottom(body.bottom().max(body.top() + 40.0));

    let mut child = ui.new_child(egui::UiBuilder::new().max_rect(body));
    egui::ScrollArea::vertical()
        .auto_shrink([false, false])
        .id_salt(("nt-body", id, tab))
        .show(&mut child, |ui| {
            ui.set_width(body.width() - 10.0);
            match tab {
                0 => {
                    let mut s = note.summary.clone();
                    if s.is_empty() && !note.is_busy() {
                        let hint = if app.state.text_model_ready() {
                            "No summary yet. Press Summarize to write one from the transcript."
                        } else {
                            "Choose a text model on the Transforms page, then press Summarize."
                        };
                        muted(ui, pal, hint);
                    } else if note.status == NoteStatus::Summarizing && s.is_empty() {
                        muted(ui, pal, "Writing the summary…");
                    } else if area(ui, pal, &mut s, 14, "", &format!("nt-summary-{id}")) {
                        if let Some(n) = app.notes.get_mut(id) {
                            n.summary = s;
                        }
                        app.notes.save();
                    }
                }
                1 => {
                    let mut s = note.notes.clone();
                    if area(
                        ui,
                        pal,
                        &mut s,
                        14,
                        "Type your own notes here. They are included when the summary is written.",
                        &format!("nt-notes-{id}"),
                    ) {
                        if let Some(n) = app.notes.get_mut(id) {
                            n.notes = s;
                        }
                        app.notes.save();
                    }
                }
                _ => {
                    let t = note.transcript();
                    if t.is_empty() {
                        let msg = match note.status {
                            NoteStatus::Recording => "Listening… the first lines appear after a few seconds.",
                            NoteStatus::Transcribing => "Transcribing…",
                            _ => "No transcript.",
                        };
                        muted(ui, pal, msg);
                    } else {
                        ui.add(
                            egui::Label::new(RichText::new(t).font(sans(13.0)).color(pal.text()))
                                .selectable(true)
                                .wrap(),
                        );
                    }
                }
            }
        });
}

// ---- My Voice --------------------------------------------------------------

pub fn my_voice(ui: &mut egui::Ui, app: &mut App, pal: Palette, content: Rect) {
    let x = content.left() + PAD;
    let right = content.right() - PAD;
    let top = header(
        ui,
        pal,
        content,
        "My Voice",
        "Flowline learns the words it mishears and shows how you dictate.",
    );
    let rect = Rect::from_min_max(Pos2::new(x, top), Pos2::new(right, content.bottom() - 12.0));
    let mut child = ui.new_child(egui::UiBuilder::new().max_rect(rect));
    egui::ScrollArea::vertical()
        .auto_shrink([false, false])
        .id_salt("voice-scroll")
        .show(&mut child, |ui| {
            ui.set_width(rect.width() - 14.0);

            // Learn from corrections
            if cfg!(not(target_os = "macos")) {
                card(ui, pal, |ui| {
                    title(ui, pal, "Learn from my corrections");
                    muted(
                        ui,
                        pal,
                        "Not available on Linux yet. It needs to read the text field you paste into, \
                         which Linux only offers through the AT-SPI accessibility bus. Your \
                         Dictionary, and the numbers below, work as usual.",
                    );
                });
                ui.add_space(14.0);
            } else {
                card(ui, pal, |ui| {
                    ui.horizontal(|ui| {
                        ui.vertical(|ui| {
                            title(ui, pal, "Learn from my corrections");
                            ui.set_max_width(ui.available_width() - 70.0);
                            muted(
                                ui,
                                pal,
                                "After Flowline pastes a dictation it watches that one text field for \
                                 20 seconds. If you fix a misheard word, the fix is added to your \
                                 Dictionary so it comes out right next time.",
                            );
                        });
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if switch(ui, pal, app.state.voice.learn, "voice-learn") {
                                app.state.voice.learn = !app.state.voice.learn;
                                if app.state.voice.learn && !crate::platform::accessibility_granted() {
                                    crate::platform::request_accessibility();
                                }
                                app.save();
                            }
                        });
                    });
                    ui.add_space(4.0);
                    faint(
                        ui,
                        pal,
                        "Reads only the field you just pasted into, never password fields. Keeps just \
                         the corrected word, not the text around it. Only small spelling fixes are \
                         learned: rewrites and everyday words are ignored.",
                    );
                    if app.state.voice.learn && !crate::platform::accessibility_granted() {
                        problem(ui, "Allow Accessibility for Flowline so it can read the field.");
                    }
                });
                ui.add_space(14.0);
            }

            // Learned list
            let mut remove: Option<usize> = None;
            card(ui, pal, |ui| {
                title(ui, pal, "What Flowline has learned");
                let learned: Vec<(usize, String, String)> = app
                    .state
                    .dictionary
                    .iter()
                    .enumerate()
                    .filter(|(_, d)| d.learned)
                    .map(|(i, d)| (i, d.word.clone(), d.replacement.clone()))
                    .collect();
                if learned.is_empty() {
                    muted(
                        ui,
                        pal,
                        if app.state.voice.learn {
                            "Nothing yet. Fix a misheard word right after a dictation and it will show up here."
                        } else {
                            "Nothing yet. Turn on learning above to start."
                        },
                    );
                }
                for (i, heard, fixed) in learned {
                    ui.separator();
                    ui.horizontal(|ui| {
                        ui.label(RichText::new(&heard).font(sans(14.0)).color(pal.text_muted()));
                        ui.label(RichText::new("→").font(sans(14.0)).color(pal.text_faint()));
                        ui.label(RichText::new(&fixed).font(sans(14.0)).color(pal.text()));
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if btn(ui, pal, "Remove", 84.0, false, &format!("voice-rm-{i}")) {
                                remove = Some(i);
                            }
                        });
                    });
                }
            });
            if let Some(i) = remove {
                if i < app.state.dictionary.len() {
                    app.state.dictionary.remove(i);
                    app.save();
                }
            }
            ui.add_space(14.0);

            // Numbers
            let s = crate::voice::stats(&app.state.history);
            card(ui, pal, |ui| {
                title(ui, pal, "Your voice in numbers");
                if s.dictations == 0 {
                    muted(ui, pal, "Dictate something and your pace and favourite words show up here.");
                    return;
                }
                ui.add_space(4.0);
                ui.horizontal_wrapped(|ui| {
                    for (label, value) in [
                        ("Dictations", s.dictations.to_string()),
                        ("Words", s.words.to_string()),
                        ("Pace", format!("{:.0} wpm", s.wpm)),
                        ("Words cleaned up", s.cleaned_words.to_string()),
                    ] {
                        Frame::new()
                            .fill(pal.card_alt())
                            .stroke(Stroke::new(1.0, pal.border()))
                            .corner_radius(CornerRadius::same(12))
                            .inner_margin(Margin::symmetric(16, 12))
                            .show(ui, |ui| {
                                ui.vertical(|ui| {
                                    ui.label(RichText::new(value).font(sans(20.0)).color(pal.text()));
                                    faint(ui, pal, label);
                                });
                            });
                    }
                });
                if !s.top_words.is_empty() {
                    ui.add_space(8.0);
                    faint(ui, pal, "Words you use most");
                    ui.horizontal_wrapped(|ui| {
                        for (w, n) in &s.top_words {
                            chip(ui, pal, &format!("{w} · {n}"), false, &format!("voice-top-{w}"));
                        }
                    });
                }
            });
            ui.add_space(24.0);
        });
}

// ---- Style: apps ----------------------------------------------------------

fn tab_name(tab: usize) -> &'static str {
    TONE_TABS.iter().find(|(t, _)| *t == tab).map(|(_, n)| *n).unwrap_or("Other")
}

pub fn style_apps(ui: &mut egui::Ui, app: &mut App, pal: Palette, rect: Rect) {
    let mut child = ui.new_child(egui::UiBuilder::new().max_rect(rect));
    egui::ScrollArea::vertical()
        .auto_shrink([false, false])
        .id_salt("style-apps-scroll")
        .show(&mut child, |ui| {
            ui.set_width(rect.width() - 14.0);

            card(ui, pal, |ui| {
                ui.horizontal(|ui| {
                    ui.vertical(|ui| {
                        title(ui, pal, "Match tone to the app");
                        ui.set_max_width(ui.available_width() - 70.0);
                        muted(
                            ui,
                            pal,
                            "Flowline picks Work, Email, Personal or Other from the app you dictate \
                             into and uses that tab's style. In a browser it reads the window title \
                             (such as \"Inbox - Gmail\") only to choose, and keeps nothing. Turn this \
                             off to use the Email style everywhere.",
                        );
                    });
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if switch(ui, pal, app.state.tone_by_app, "tone-by-app") {
                            app.state.tone_by_app = !app.state.tone_by_app;
                            app.save();
                        }
                    });
                });
            });
            ui.add_space(14.0);

            // Apps you have dictated into, newest first, plus any you set by hand.
            let mut apps: Vec<String> = Vec::new();
            let names = app
                .state
                .app_tones
                .iter()
                .map(|r| r.app.clone())
                .chain(app.state.history.iter().rev().map(|d| d.app.clone()));
            for name in names {
                let n = name.trim();
                if !n.is_empty()
                    && n != "Scratchpad"
                    && !apps.iter().any(|a| a.eq_ignore_ascii_case(n))
                {
                    apps.push(n.to_string());
                }
                if apps.len() >= 30 {
                    break;
                }
            }

            let mut change: Option<(String, Option<usize>)> = None;
            card(ui, pal, |ui| {
                title(ui, pal, "Apps");
                muted(
                    ui,
                    pal,
                    "Apps you have dictated into appear here. Pick a category to override the automatic choice.",
                );
                if apps.is_empty() {
                    ui.add_space(6.0);
                    faint(ui, pal, "No apps yet. Dictate into an app and it will be listed.");
                }
                for (i, name) in apps.iter().enumerate() {
                    ui.separator();
                    let chosen = app
                        .state
                        .app_tones
                        .iter()
                        .find(|r| r.app.eq_ignore_ascii_case(name))
                        .map(|r| r.tab);
                    let auto = builtin_tab_for_app(name, None);
                    let effective = chosen.unwrap_or(auto);
                    ui.horizontal_wrapped(|ui| {
                        ui.vertical(|ui| {
                            ui.set_min_width(330.0);
                            ui.set_max_width(330.0);
                            ui.label(RichText::new(name).font(sans(14.0)).color(pal.text()));
                            let how = match chosen {
                                Some(_) => "Set by you".to_string(),
                                None if is_browser(name) => "Chosen by page title".to_string(),
                                None => "Automatic".to_string(),
                            };
                            faint(
                                ui,
                                pal,
                                &format!(
                                    "{} · {}",
                                    how,
                                    if chosen.is_none() && is_browser(name) {
                                        "Other unless the page says otherwise".to_string()
                                    } else {
                                        format!(
                                            "{} · {}",
                                            tab_name(effective),
                                            app.style_for_tab(effective).label()
                                        )
                                    }
                                ),
                            );
                        });
                        if chip(ui, pal, "Auto", chosen.is_none(), &format!("tone-auto-{i}")) {
                            change = Some((name.clone(), None));
                        }
                        for (tab, label) in TONE_TABS {
                            if chip(ui, pal, label, chosen == Some(tab), &format!("tone-{i}-{tab}")) {
                                change = Some((name.clone(), Some(tab)));
                            }
                        }
                    });
                }
            });
            if let Some((name, tab)) = change {
                app.state.app_tones.retain(|r| !r.app.eq_ignore_ascii_case(&name));
                if let Some(tab) = tab {
                    app.state.app_tones.push(AppTone { app: name, tab });
                }
                app.save();
            }
            ui.add_space(24.0);
        });
}
