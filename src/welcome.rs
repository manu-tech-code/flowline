use crate::theme::*;
use crate::ui::*;
use eframe::egui::{self, Align2, Color32, CornerRadius, FontId, Pos2, Rect, Stroke, Vec2};

pub fn show(ui: &mut egui::Ui, pal: Palette, time: f32) -> bool {
    let full = ui.max_rect();
    let w = full.width();
    let h = full.height();
    let split = (w * 0.52).clamp(560.0, 760.0);
    let p = ui.painter().clone();

    // left panel
    p.rect_filled(
        Rect::from_min_max(full.min, Pos2::new(split, full.bottom())),
        0.0,
        SAND_100,
    );

    // top strip
    p.rect_filled(
        Rect::from_min_max(full.min, Pos2::new(full.right(), full.top() + 44.0)),
        0.0,
        SAND_50,
    );
    p.line_segment(
        [
            Pos2::new(full.left(), full.top() + 44.5),
            Pos2::new(full.right(), full.top() + 44.5),
        ],
        Stroke::new(1.0, SAND_600),
    );

    // right gradient
    let right = Rect::from_min_max(Pos2::new(split, full.top() + 44.0), full.max);
    paint_gradient(&p, right);

    // brand
    let brand_y = full.top() + 44.0 + (h - 44.0) * 0.335;
    let mark = Rect::from_center_size(
        Pos2::new(split / 2.0 - 57.0, brand_y + 10.0),
        Vec2::new(26.0, 20.0),
    );
    draw_brand_mark(&p, mark, pal.text());
    let word = "Flowline";
    let galley = ui.fonts(|f| f.layout_no_wrap(word.to_owned(), sans(21.0), pal.text()));
    let word_x = mark.right() + 10.0;
    p.galley(Pos2::new(word_x, brand_y), galley, pal.text());

    let cx = split / 2.0;
    let title_y = full.top() + 44.0 + (h - 44.0) * 0.405;
    let title = "Let's get you started";
    let g = ui.fonts(|f| f.layout_no_wrap(title.to_owned(), sans(33.0), pal.text()));
    let tw = g.size().x;
    p.galley(Pos2::new(cx - tw / 2.0, title_y), g, pal.text());

    let sub_y = title_y + 46.0;
    let sub = "Accurate voice-to-text, everywhere you work";
    let g = ui.fonts(|f| f.layout_no_wrap(sub.to_owned(), sans(15.5), pal.text_muted()));
    p.galley(Pos2::new(cx - g.size().x / 2.0, sub_y), g, pal.text_muted());

    // continue button
    let bw = (split - 120.0).min(420.0);
    let brect = Rect::from_center_size(Pos2::new(cx, sub_y + 78.0), Vec2::new(bw, 46.0));
    let hov = hovered(ui, brect, "continue-demo");
    p.rect_filled(
        brect,
        CornerRadius::same(10),
        if hov {
            if pal.dark {
                SAND_100
            } else {
                VAST_800
            }
        } else {
            pal.accent()
        },
    );
    let label = "Get started";
    let fg = if pal.dark { VAST_950 } else { Color32::WHITE };
    let g = ui.fonts(|f| f.layout_no_wrap(label.to_owned(), sans(14.5), fg));
    let arrow = Rect::from_center_size(
        Pos2::new(brect.right() - 26.0, brect.center().y),
        Vec2::splat(12.0),
    );
    let start_x = brect.center().x - (g.size().x + 22.0) / 2.0;
    p.galley(
        Pos2::new(start_x, brect.center().y - g.size().y / 2.0),
        g,
        fg,
    );
    icon(&p, "arrow", arrow, fg, 1.5);

    // legal
    let legal_y = brect.bottom() + 26.0;
    let l1 = "Everything stays on this Mac.";
    let g = ui.fonts(|f| f.layout_no_wrap(l1.to_owned(), sans(12.5), pal.text_muted()));
    p.galley(
        Pos2::new(cx - g.size().x / 2.0, legal_y),
        g,
        pal.text_muted(),
    );
    let l2 = "Audio goes only to the engine you choose.";
    let g = ui.fonts(|f| f.layout_no_wrap(l2.to_owned(), sans(12.5), pal.text()));
    p.galley(
        Pos2::new(cx - g.size().x / 2.0, legal_y + 21.0),
        g,
        pal.text(),
    );

    // help pill
    let help = Rect::from_min_size(
        Pos2::new(full.left() + 20.0, full.bottom() - 52.0),
        Vec2::new(76.0, 32.0),
    );
    p.rect_filled(help, CornerRadius::same(16), SAND_600);
    icon(
        &p,
        "help",
        Rect::from_center_size(
            Pos2::new(help.left() + 18.0, help.center().y),
            Vec2::splat(13.0),
        ),
        pal.text_muted(),
        1.2,
    );
    let g = ui.fonts(|f| f.layout_no_wrap("Help".to_owned(), sans(12.5), pal.text_muted()));
    p.galley(
        Pos2::new(help.left() + 32.0, help.center().y - g.size().y / 2.0),
        g,
        pal.text_muted(),
    );

    // right: headline
    let rcx = split + (w - split) / 2.0;
    let hy = full.top() + 44.0 + 102.0;
    let f1 = "Works ";
    let f2 = "wherever";
    let f3 = " you do.";
    let g1 = ui.fonts(|f| f.layout_no_wrap(f1.to_owned(), serif(35.0), pal.text()));
    let g2 = ui.fonts(|f| f.layout_no_wrap(f2.to_owned(), serif_italic(35.0), pal.text()));
    let g3 = ui.fonts(|f| f.layout_no_wrap(f3.to_owned(), serif(35.0), pal.text()));
    let total = g1.size().x + g2.size().x + g3.size().x;
    let mut x = rcx - total / 2.0;
    let w1 = g1.size().x;
    let w2 = g2.size().x;
    p.galley(Pos2::new(x, hy), g1, pal.text());
    x += w1;
    p.galley(Pos2::new(x, hy), g2, pal.text());
    x += w2;
    p.galley(Pos2::new(x, hy), g3, pal.text());

    // mock app card
    let card_w = (w - split - 120.0).min(490.0);
    let card = Rect::from_center_size(Pos2::new(rcx, hy + 238.0), Vec2::new(card_w, 300.0));
    shadow(&p, card, 22);
    p.rect(
        card,
        CornerRadius::same(22),
        Color32::from_rgba_unmultiplied(255, 255, 255, 242),
        Stroke::new(1.0, Color32::from_white_alpha(60)),
        egui::StrokeKind::Inside,
    );
    // sidebar glyph rail
    let rail_x = card.left() + 24.0;
    for (i, name) in ["style", "bell", "search", "image", "clock", "grid"]
        .iter()
        .enumerate()
    {
        let r = Rect::from_center_size(
            Pos2::new(rail_x, card.top() + 32.0 + i as f32 * 32.0),
            Vec2::splat(13.0),
        );
        if *name == "bell" {
            p.add(egui::Shape::line(
                vec![
                    Pos2::new(r.center().x - 6.0, r.center().y + 4.0),
                    Pos2::new(r.center().x - 5.0, r.center().y - 3.0),
                    Pos2::new(r.center().x, r.center().y - 7.0),
                    Pos2::new(r.center().x + 5.0, r.center().y - 3.0),
                    Pos2::new(r.center().x + 6.0, r.center().y + 4.0),
                    Pos2::new(r.center().x - 6.0, r.center().y + 4.0),
                ],
                Stroke::new(1.2, SAND_800),
            ));
        } else if *name == "image" {
            p.rect_stroke(
                r,
                CornerRadius::same(2),
                Stroke::new(1.2, SAND_800),
                egui::StrokeKind::Middle,
            );
            p.circle_filled(Pos2::new(r.left() + 4.0, r.top() + 4.0), 1.4, SAND_800);
        } else if *name == "clock" {
            p.circle_stroke(r.center(), 6.0, Stroke::new(1.2, SAND_800));
            p.line_segment(
                [r.center(), Pos2::new(r.center().x, r.center().y - 4.0)],
                Stroke::new(1.2, SAND_800),
            );
        } else if *name == "grid" {
            for (dx, dy) in [(-4.0, -4.0), (1.0, -4.0), (-4.0, 1.0), (1.0, 1.0)] {
                p.rect_filled(
                    Rect::from_min_size(
                        Pos2::new(r.center().x + dx, r.center().y + dy),
                        Vec2::splat(4.0),
                    ),
                    1,
                    SAND_800,
                );
            }
        } else {
            icon(&p, name, r, SAND_800, 1.2);
        }
    }
    // main mock content
    let mc = Pos2::new((rail_x + 20.0 + card.right()) * 0.5, card.top() + 120.0);
    let g =
        ui.fonts(|f| f.layout_no_wrap("What can I help with ?".to_owned(), sans(13.5), pal.text()));
    p.galley(Pos2::new(mc.x - g.size().x / 2.0, mc.y), g, pal.text());
    let input =
        Rect::from_center_size(Pos2::new(mc.x, mc.y + 56.0), Vec2::new(card_w * 0.58, 44.0));
    p.rect(
        input,
        CornerRadius::same(8),
        Color32::WHITE,
        Stroke::new(1.0, SAND_600),
        egui::StrokeKind::Inside,
    );
    let g = ui.fonts(|f| f.layout_no_wrap("Ask anything".to_owned(), sans(10.5), SAND_800));
    p.galley(
        Pos2::new(input.left() + 10.0, input.top() + 8.0),
        g,
        SAND_800,
    );
    let g = ui.fonts(|f| f.layout_no_wrap("+   Tools".to_owned(), sans(9.5), SAND_900));
    p.galley(
        Pos2::new(input.left() + 10.0, input.bottom() - 15.0),
        g,
        SAND_900,
    );
    icon(
        &p,
        "mic",
        Rect::from_center_size(
            Pos2::new(input.right() - 34.0, input.bottom() - 12.0),
            Vec2::splat(11.0),
        ),
        SAND_900,
        1.0,
    );
    p.circle_filled(
        Pos2::new(input.right() - 15.0, input.bottom() - 12.0),
        6.0,
        VAST_950,
    );
    p.line_segment(
        [
            Pos2::new(input.right() - 18.0, input.bottom() - 12.0),
            Pos2::new(input.right() - 12.0, input.bottom() - 12.0),
        ],
        Stroke::new(1.0, Color32::WHITE),
    );

    // mic pill below card
    let pill = Rect::from_center_size(Pos2::new(rcx, card.bottom() + 38.0), Vec2::new(56.0, 26.0));
    p.rect_filled(pill, CornerRadius::same(13), VAST_950);
    waveform(
        ui,
        pill.shrink2(Vec2::new(17.0, 8.0)),
        Color32::WHITE,
        time * 0.6,
        1.0,
    );

    // dock
    if card.bottom() + 120.0 < full.bottom() {
        let dock_w = (w - split - 160.0).min(516.0);
        let dock = Rect::from_center_size(
            Pos2::new(rcx, pill.bottom() + 58.0),
            Vec2::new(dock_w, 66.0),
        );
        shadow(&p, dock, 18);
        p.rect_filled(
            dock,
            CornerRadius::same(18),
            Color32::from_rgba_unmultiplied(231, 226, 215, 235),
        );
        let n = 7;
        let gap = 11.0;
        let tile = 44.0;
        let total = n as f32 * tile + (n - 1) as f32 * gap;
        let mut x = dock.center().x - total / 2.0;
        for i in 0..n {
            let tr = Rect::from_min_size(
                Pos2::new(x, dock.center().y - tile / 2.0),
                Vec2::splat(tile),
            );
            app_tile(&p, i, tr);
            x += tile + gap;
        }
        // dots
        for i in 0..3 {
            let dx = rcx - 18.0 + i as f32 * 18.0;
            let dy = dock.bottom() + 22.0;
            p.circle_filled(
                Pos2::new(dx, dy),
                2.8,
                if i == 0 { SAND_900 } else { SAND_700 },
            );
        }
    }

    click(ui, brect, "continue-demo")
}

fn paint_gradient(p: &egui::Painter, rect: Rect) {
    let cols = 48;
    let rows = 32;
    let mut mesh = egui::Mesh::default();
    let top = [255.0f32, 251.0, 244.0];
    let bottom = [250.0, 231.0, 210.0];
    let blobs: [(f32, f32, f32, [f32; 3], f32); 3] = [
        (0.80, 0.06, 0.58, [255.0, 199.0, 140.0], 0.85),
        (0.08, 0.98, 0.52, [228.0, 205.0, 242.0], 0.70),
        (0.50, 0.50, 0.55, [255.0, 240.0, 205.0], 0.55),
    ];
    for r in 0..=rows {
        for c in 0..=cols {
            let u = c as f32 / cols as f32;
            let v = r as f32 / rows as f32;
            let mut col = [0.0f32; 3];
            for k in 0..3 {
                col[k] = top[k] + (bottom[k] - top[k]) * v;
            }
            for (bx, by, br, bc, bs) in blobs {
                let dx = u - bx;
                let dy = v - by;
                let d2 = (dx * dx + dy * dy) / (br * br);
                let w = (-d2 * 2.2).exp() * bs;
                for k in 0..3 {
                    col[k] += (bc[k] - col[k]) * w * 0.55;
                }
            }
            let color = Color32::from_rgb(
                col[0].clamp(0.0, 255.0) as u8,
                col[1].clamp(0.0, 255.0) as u8,
                col[2].clamp(0.0, 255.0) as u8,
            );
            mesh.colored_vertex(
                Pos2::new(
                    rect.left() + rect.width() * u,
                    rect.top() + rect.height() * v,
                ),
                color,
            );
        }
    }
    for r in 0..rows {
        for c in 0..cols {
            let i = (r * (cols + 1) + c) as u32;
            let i2 = i + 1;
            let i3 = i + cols as u32 + 1;
            let i4 = i3 + 1;
            mesh.add_triangle(i, i2, i3);
            mesh.add_triangle(i2, i4, i3);
        }
    }
    p.add(egui::Shape::mesh(mesh));
}

fn draw_brand_mark(p: &egui::Painter, rect: Rect, color: Color32) {
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

fn app_tile(p: &egui::Painter, i: usize, r: Rect) {
    let white = Color32::WHITE;
    match i {
        0 => {
            p.rect_filled(r, CornerRadius::same(12), Color32::from_rgb(74, 214, 92));
            p.add(egui::Shape::closed_line(
                vec![
                    Pos2::new(r.left() + 12.0, r.top() + 15.0),
                    Pos2::new(r.right() - 12.0, r.top() + 15.0),
                    Pos2::new(r.right() - 12.0, r.bottom() - 15.0),
                    Pos2::new(r.left() + 17.0, r.bottom() - 15.0),
                    Pos2::new(r.left() + 10.0, r.bottom() - 9.0),
                    Pos2::new(r.left() + 12.0, r.bottom() - 15.0),
                ],
                Stroke::new(2.0, white),
            ));
        }
        1 => {
            p.rect_filled(r, CornerRadius::same(12), white);
            let c = r.center();
            p.add(egui::Shape::line(
                vec![
                    Pos2::new(c.x - 13.0, c.y - 8.0),
                    Pos2::new(c.x - 13.0, c.y + 9.0),
                    Pos2::new(c.x + 13.0, c.y + 9.0),
                    Pos2::new(c.x + 13.0, c.y - 8.0),
                ],
                Stroke::new(2.0, Color32::from_rgb(220, 68, 58)),
            ));
            p.add(egui::Shape::line(
                vec![
                    Pos2::new(c.x - 13.0, c.y - 8.0),
                    Pos2::new(c.x, c.y + 2.0),
                    Pos2::new(c.x + 13.0, c.y - 8.0),
                ],
                Stroke::new(2.0, Color32::from_rgb(66, 133, 244)),
            ));
        }
        2 => {
            p.rect_filled(r, CornerRadius::same(12), white);
            let c = r.center();
            p.rect_stroke(
                Rect::from_center_size(c, Vec2::new(24.0, 26.0)),
                CornerRadius::same(3),
                Stroke::new(2.0, VAST_950),
                egui::StrokeKind::Middle,
            );
            p.text(
                c,
                Align2::CENTER_CENTER,
                "N",
                FontId::proportional(16.0),
                VAST_950,
            );
        }
        3 => {
            p.rect_filled(r, CornerRadius::same(12), VAST_950);
            let wr = Rect::from_center_size(r.center(), Vec2::new(28.0, 22.0));
            for k in 0..7 {
                let x = wr.left() + wr.width() / 7.0 * (k as f32 + 0.5);
                let h = 10.0 * (0.3 + 0.7 * ((k as f32 * 1.1).sin().abs()));
                p.line_segment(
                    [
                        Pos2::new(x, wr.center().y - h),
                        Pos2::new(x, wr.center().y + h),
                    ],
                    Stroke::new(2.0, white),
                );
            }
        }
        4 => {
            p.rect_filled(r, CornerRadius::same(12), white);
            let c = r.center();
            let dots = [
                (-8.0, -2.0, Color32::from_rgb(230, 80, 60)),
                (0.0, -8.0, Color32::from_rgb(80, 170, 120)),
                (8.0, -2.0, Color32::from_rgb(80, 110, 230)),
                (0.0, 8.0, Color32::from_rgb(240, 190, 60)),
            ];
            for (dx, dy, col) in dots {
                p.circle_filled(Pos2::new(c.x + dx, c.y + dy), 6.0, col);
            }
        }
        5 => {
            p.rect_filled(r, CornerRadius::same(12), Color32::from_rgb(250, 210, 80));
            for i in 0..4 {
                let y = r.top() + 12.0 + i as f32 * 7.0;
                p.line_segment(
                    [Pos2::new(r.left() + 9.0, y), Pos2::new(r.right() - 9.0, y)],
                    Stroke::new(1.4, VAST_950),
                );
            }
        }
        _ => {
            p.rect_filled(r, CornerRadius::same(12), white);
            let c = r.center();
            for i in 1..4 {
                p.circle_stroke(c, i as f32 * 4.5, Stroke::new(1.4, VAST_950));
            }
        }
    }
}
