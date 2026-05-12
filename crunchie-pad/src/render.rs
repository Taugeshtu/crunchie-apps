use eframe::egui;
use std::collections::BTreeMap;
use crunchie_core::model::{Workspace, Diagnostic, Atom};

#[derive(Clone, Copy)]
pub enum HighlightType {
    Function,
    Constant,
    Comment,
}

pub struct Theme {
    pub sticky_note_yellow: egui::Color32,
    pub ghost_gray: egui::Color32,
    pub function_blue: egui::Color32,
    pub constant_purple: egui::Color32,
    pub comment_gray: egui::Color32,
    pub error_red: egui::Color32,
}

impl Default for Theme {
    fn default() -> Self {
        Self {
            sticky_note_yellow: egui::Color32::from_rgb(255, 240, 150),
            ghost_gray: egui::Color32::from_gray(160),
            function_blue: egui::Color32::from_rgb(0, 70, 200),
            constant_purple: egui::Color32::from_rgb(140, 0, 200),
            comment_gray: egui::Color32::from_gray(120),
            error_red: egui::Color32::from_rgba_premultiplied(255, 0, 0, 77), // ~30% opaque
        }
    }
}

pub fn build_highlight_map(
    workspace: &Workspace,
) -> BTreeMap<u32, (usize, HighlightType)> {
    let mut map = BTreeMap::new();

    // 1. Comments
    for comment in &workspace.comments {
        let len = (comment.end.offset - comment.start.offset) as usize;
        map.insert(comment.start.offset, (len, HighlightType::Comment));
    }

    // 2. Functions and Constants
    for container in workspace.containers.values() {
        for entity in &container.contents {
            if let Some(atom) = workspace.atoms.get(&entity.id) {
                match atom {
                    Atom::Function(s) => {
                        map.insert(entity.position.offset, (s.len(), HighlightType::Function));
                    }
                    Atom::Constant(s) => {
                        map.insert(entity.position.offset, (s.len(), HighlightType::Constant));
                    }
                    _ => {}
                }
            }
        }
    }
    map
}

pub fn draw_ghost_text(
    ui: &egui::Ui,
    rect: egui::Rect,
    text: &str,
    theme: &Theme,
) {
    let mut job = egui::text::LayoutJob::default();
    job.append(
        text,
        0.0,
        egui::TextFormat {
            font_id: egui::FontId::monospace(18.0),
            color: theme.ghost_gray,
            italics: true,
            ..Default::default()
        },
    );
    let galley = ui.fonts(|f| f.layout_job(job));
    ui.painter().galley(
        rect.min + egui::vec2(10.0, 10.0),
        galley,
        theme.ghost_gray,
    );
}

pub fn draw_diagnostics(
    ui: &egui::Ui,
    galley_pos: egui::Pos2,
    galley: &egui::Galley,
    diagnostics: &[Diagnostic],
    full_text: &str,
    theme: &Theme,
) {
    let painter = ui.painter();
    
    for diagnostic in diagnostics {
        let start_char = full_text
            .char_indices()
            .take_while(|(i, _)| *i < diagnostic.span.start.offset as usize)
            .count();
        let end_char = full_text
            .char_indices()
            .take_while(|(i, _)| *i < diagnostic.span.end.offset as usize)
            .count();

        let range = if start_char == end_char {
            start_char..start_char + 1
        } else {
            start_char..end_char
        };

        for i in range {
            let ccursor = egui::text::CCursor::new(i);
            let cursor = galley.from_ccursor(ccursor);
            let rect = galley.pos_from_cursor(&cursor);
            
            let mut squiggle_rect = rect;
            let row_height = if let Some(row) = galley.rows.iter().find(|r| r.rect.y_range().contains(rect.center().y)) {
                row.height()
            } else {
                18.0
            };
            
            //squiggle_rect.min.y = rect.min.y + ((1.0 - 0.05) * row_height);
            squiggle_rect.min.y = rect.min.y + 0.85 *row_height;
            squiggle_rect.max.y = squiggle_rect.min.y + (0.25 * row_height);
            
            if i + 1 <= full_text.chars().count() {
                let next_ccursor = egui::text::CCursor::new(i + 1);
                let next_cursor = galley.from_ccursor(next_ccursor);
                let next_rect = galley.pos_from_cursor(&next_cursor);
                if (next_rect.min.y - rect.min.y).abs() < 1.0 {
                    squiggle_rect.max.x = next_rect.min.x;
                } else {
                    squiggle_rect.max.x += 10.0;
                }
            } else {
                squiggle_rect.max.x += 10.0;
            }
            
            let final_rect = squiggle_rect.translate(galley_pos.to_vec2());
            painter.rect_filled(final_rect, egui::Rounding::ZERO, theme.error_red);
        }
    }
}

pub fn syntax_layouter(
    ui: &egui::Ui,
    string: &str,
    highlight_map: &BTreeMap<u32, (usize, HighlightType)>,
    theme: &Theme,
) -> std::sync::Arc<egui::Galley> {
    let mut job = egui::text::LayoutJob::default();
    let mut current_byte = 0;
    let mut chars = string.chars().peekable();

    while let Some(c) = chars.next() {
        let byte_len = c.len_utf8();
        let start_byte = current_byte;

        if let Some((len, h_type)) = highlight_map.get(&(start_byte as u32)) {
            let mut text = String::from(c);
            let mut consumed = byte_len;
            while consumed < *len {
                if let Some(next_c) = chars.next() {
                    text.push(next_c);
                    consumed += next_c.len_utf8();
                } else {
                    break;
                }
            }

            let format = match h_type {
                HighlightType::Function => egui::TextFormat {
                    font_id: egui::FontId::monospace(18.0),
                    color: theme.function_blue,
                    italics: true,
                    ..Default::default()
                },
                HighlightType::Constant => egui::TextFormat {
                    font_id: egui::FontId::monospace(18.0),
                    color: theme.constant_purple,
                    ..Default::default()
                },
                HighlightType::Comment => egui::TextFormat {
                    font_id: egui::FontId::monospace(18.0),
                    color: theme.comment_gray,
                    italics: true,
                    ..Default::default()
                },
            };
            job.append(&text, 0.0, format);
            current_byte += consumed;
            continue;
        }

        job.append(
            &c.to_string(),
            0.0,
            egui::TextFormat {
                font_id: egui::FontId::monospace(18.0),
                color: ui.visuals().text_color(),
                ..Default::default()
            },
        );
        current_byte += byte_len;
    }
    ui.fonts(|f| f.layout_job(job))
}
