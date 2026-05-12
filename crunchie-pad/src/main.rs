use crunchie_core::config::Config;
use eframe::egui;

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

const HINTS: &[&str] = &[
    "// Try this:\n(1cm2 + 3mm^2) *2 =",
    "// Try this:\nradius = 10cm\narea = pi * radius^2\narea =",
    "// Try this:\nsin(45deg) =",
    "// Try this:\n60 mph to m/s=",
    "// Try this:\nx = 5; y = 10; x + y =",
];

struct CrunchiePad {
    text: String,
    config: Config,
    needs_update: bool,
    pending_edits: Vec<crunchie_core::model::TextEdit>,
    is_first_frame: bool,
    hint: String,
}

impl Default for CrunchiePad {
    fn default() -> Self {
        let hint_index = (std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos()
            % HINTS.len() as u128) as usize;

        Self {
            text: String::new(),
            config: Config::default(),
            needs_update: false,
            pending_edits: Vec::new(),
            is_first_frame: true,
            hint: HINTS[hint_index].to_string(),
        }
    }
}

fn format_edit(edit: &crunchie_core::model::TextEdit) -> (String, std::ops::Range<usize>) {
    if let Some(value) = &edit.value {
        let mut ctx = fend_core::Context::new();
        let int = fend_core::interrupt::Never;
        let mut attrs = fend_core::eval::Attrs::default();
        attrs.show_approx = false;

        let mut spans = Vec::new();
        if value
            .format(0, &mut spans, attrs, false, &mut ctx, &int)
            .is_ok()
        {
            let mut full_text = String::from(" ");
            let mut num_start = 1;
            let mut num_end = 1;
            let mut found_number = false;

            for span in spans {
                if span.string == "approx. " {
                    continue;
                }
                if span.kind == fend_core::SpanKind::Number && !found_number {
                    num_start = full_text.chars().count();
                    num_end = num_start + span.string.chars().count();
                    found_number = true;
                }
                full_text.push_str(&span.string);
            }
            return (full_text, num_start..num_end);
        }
    }
    let len = edit.new_text.chars().count();
    (edit.new_text.clone(), 0..len)
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
        let start_char = text
            .char_indices()
            .take_while(|(i, _)| *i < start_byte)
            .count();
        let end_char = text
            .char_indices()
            .take_while(|(i, _)| *i < end_byte)
            .count();

        if start_char <= ghost_chars.len() && end_char <= ghost_chars.len() {
            let (clean_text, _) = format_edit(&edit);
            let new_chars: Vec<char> = clean_text.chars().collect();
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

        let edit_id = egui::Id::new("crunchie-edit");

        // Handle Tab to commit ALL
        if !self.pending_edits.is_empty()
            && ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::Tab))
        {
            let mut all_edits = self.pending_edits.clone();
            for edit in &mut all_edits {
                let (clean, _) = format_edit(edit);
                edit.new_text = clean;
            }
            self.text = crunchie_core::apply_edits(&self.text, &all_edits);
            self.pending_edits.clear();
            self.needs_update = true;
        }

        // Handle Enter to commit current line
        if !self.pending_edits.is_empty() {
            if let Some(state) = egui::TextEdit::load_state(ctx, edit_id) {
                if let Some(char_range) = state.cursor.char_range() {
                    if char_range.primary == char_range.secondary {
                        let ccursor = char_range.primary;
                        // Map absolute char index to line/col
                        let text_before: String = self.text.chars().take(ccursor.index).collect();
                        let line_index = text_before.chars().filter(|&c| c == '\n').count();
                        let line_text = self.text.split('\n').nth(line_index).unwrap_or("");
                        let col_index = text_before.split('\n').last().unwrap_or("").chars().count();

                        if col_index == line_text.chars().count() {
                            let has_edit = self
                                .pending_edits
                                .iter()
                                .any(|e| e.span.start.line == line_index as u32);
                            if has_edit
                                && ctx.input_mut(|i| {
                                    i.consume_key(egui::Modifiers::NONE, egui::Key::Enter)
                                })
                            {
                                let line_edits: Vec<_> = self
                                    .pending_edits
                                    .iter()
                                    .filter(|e| e.span.start.line == line_index as u32)
                                    .cloned()
                                    .collect();

                                // Calculate the range of the inserted text for selection
                                let mut min_offset = usize::MAX;
                                let mut number_range_in_final = 0..0;

                                let mut line_edits_mut = line_edits.clone();
                                for edit in &mut line_edits_mut {
                                    let (clean_text, num_range) = format_edit(edit);
                                    edit.new_text = clean_text;

                                    let offset = edit.span.start.offset as usize;
                                    if offset < min_offset {
                                        min_offset = offset;
                                        number_range_in_final = num_range;
                                    }
                                }

                                self.text = crunchie_core::apply_edits(&self.text, &line_edits_mut);

                                // Map byte offset to char index for selection
                                let start_char_index = self
                                    .text
                                    .char_indices()
                                    .take_while(|(i, _)| *i < min_offset)
                                    .count();
                                
                                let sel_start = start_char_index + number_range_in_final.start;
                                let sel_end = start_char_index + number_range_in_final.end;

                                // Update state to select the new text
                                let mut new_state = state.clone();
                                let ccursor_range = egui::text::CCursorRange::two(
                                    egui::text::CCursor::new(sel_start),
                                    egui::text::CCursor::new(sel_end),
                                );
                                new_state.cursor.set_char_range(Some(ccursor_range));
                                new_state.store(ctx, edit_id);

                                self.pending_edits.clear();
                                self.needs_update = true;
                            }
                        }
                    }
                }
            }
        }

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.add_space(10.0);

            let rect = ui.available_rect_before_wrap();

            // Render ghost text
            let mut ghost_text = generate_ghost_text(&self.text, &self.pending_edits);
            if ghost_text.is_empty() && self.text.trim().is_empty() {
                ghost_text = self.hint.clone();
            }

            if !ghost_text.is_empty() {
                let mut job = egui::text::LayoutJob::default();
                job.append(
                    &ghost_text,
                    0.0,
                    egui::TextFormat {
                        font_id: egui::FontId::monospace(18.0),
                        color: egui::Color32::from_gray(160),
                        italics: true,
                        ..Default::default()
                    },
                );
                let galley = ui.fonts(|f| f.layout_job(job));
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
                            .id(edit_id)
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

            if edit.changed() || self.needs_update {
                let builtins = crunchie_core::builtins::generate_symbol_map();
                let constants = self.config.constants.keys().map(|s| s.as_str());

                let mut workspace = crunchie_core::parse(&self.text, &builtins, constants);
                let engine_result =
                    crunchie_core::evaluate(&self.text, &mut workspace, &self.config);

                self.pending_edits = engine_result.edits;
                self.needs_update = false;
            }
        });
    }
}

