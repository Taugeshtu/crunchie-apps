use eframe::egui;
use crunchie_core::config::Config;
use std::time::{Duration, Instant};

fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([300.0, 400.0])
            .with_always_on_top()
            .with_app_id("crunchie-pad"),
        ..Default::default()
    };
    eframe::run_native(
        "crunchie-pad",
        options,
        Box::new(|_cc| Ok(Box::<CrunchiePad>::default())),
    )
}

struct CrunchiePad {
    text: String,
    last_edit: Instant,
    config: Config,
    needs_update: bool,
    pending_edits: Vec<crunchie_core::model::TextEdit>,
    is_first_frame: bool,
}

impl Default for CrunchiePad {
    fn default() -> Self {
        Self {
            text: String::new(),
            last_edit: Instant::now(),
            config: Config::default(),
            needs_update: false,
            pending_edits: Vec::new(),
            is_first_frame: true,
        }
    }
}

fn generate_ghost_text(text: &str, edits: &[crunchie_core::model::TextEdit]) -> String {
    if edits.is_empty() {
        return String::new();
    }

    // 1. Create a version of 'text' where every non-whitespace character is a space.
    // This preserves visual alignment in monospace.
    let mut ghost_chars: Vec<char> = text
        .chars()
        .map(|c| if c.is_whitespace() { c } else { ' ' })
        .collect();

    // 2. Apply edits to this ghost string using char-index mapping.
    let mut sorted_edits = edits.to_vec();
    sorted_edits.sort_by(|a, b| b.span.start.offset.cmp(&a.span.start.offset));

    for edit in sorted_edits {
        let start_byte = edit.span.start.offset as usize;
        let end_byte = edit.span.end.offset as usize;

        // Map byte offsets to char indices safely
        let start_char = text.char_indices().take_while(|(i, _)| *i < start_byte).count();
        let end_char = text.char_indices().take_while(|(i, _)| *i < end_byte).count();

        if start_char <= ghost_chars.len() && end_char <= ghost_chars.len() {
            let new_chars: Vec<char> = edit.new_text.chars().collect();
            ghost_chars.splice(start_char..end_char, new_chars);
        }
    }

    ghost_chars.into_iter().collect()
}

impl eframe::App for CrunchiePad {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let mut visuals = egui::Visuals::light();
        visuals.panel_fill = egui::Color32::from_rgb(255, 240, 150); // Sticky note yellow
        ctx.set_visuals(visuals);

        // Handle Tab to commit
        if !self.pending_edits.is_empty()
            && ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::Tab))
        {
            self.text = crunchie_core::apply_edits(&self.text, &self.pending_edits);
            self.pending_edits.clear();
            self.needs_update = true;
            self.last_edit = Instant::now();
        }

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.add_space(10.0);

            let rect = ui.available_rect_before_wrap();

            // Render ghost text
            let ghost_text = generate_ghost_text(&self.text, &self.pending_edits);
            if !ghost_text.is_empty() {
                let font_id = egui::FontId::monospace(18.0);
                let galley = ui.painter().layout_no_wrap(
                    ghost_text,
                    font_id,
                    egui::Color32::from_gray(160),
                );
                ui.painter().galley(
                    rect.min + egui::vec2(10.0, 10.0),
                    galley,
                    egui::Color32::from_gray(160),
                );
            }

            let edit = ui
                .centered_and_justified(|ui| {
                    let response = ui.add(
                        egui::TextEdit::multiline(&mut self.text)
                            .font(egui::FontId::monospace(18.0))
                            .frame(false)
                            .desired_width(f32::INFINITY)
                            .margin(egui::vec2(10.0, 10.0)),
                    );

                    if self.is_first_frame {
                        response.request_focus();
                        self.is_first_frame = false;
                    }

                    response
                })
                .inner;

            if edit.changed() {
                self.last_edit = Instant::now();
                self.needs_update = true;
                self.pending_edits.clear();
            }

            // Debounce: 500ms
            if self.needs_update && self.last_edit.elapsed() >= Duration::from_millis(500) {
                let builtins = crunchie_core::builtins::generate_symbol_map();
                let constants = self.config.constants.keys().map(|s| s.as_str());

                let mut workspace = crunchie_core::parse(&self.text, &builtins, constants);
                let engine_result =
                    crunchie_core::evaluate(&self.text, &mut workspace, &self.config);

                self.pending_edits = engine_result.edits;
                self.needs_update = false;
            }
        });

        // Keep the UI responsive for the debounce timer
        if self.needs_update {
            ctx.request_repaint_after(Duration::from_millis(100));
        }
    }
}
