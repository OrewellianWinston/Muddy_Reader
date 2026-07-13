use std::path::Path;

use eframe::egui::{
    self, Align, CornerRadius, FontFamily, FontId, Frame, Label, Layout, Margin, RichText,
    ScrollArea, Stroke, TextEdit, Vec2,
};
use md_reader::{DocumentIndex, Heading, WordHeatmap, search_sections};

use super::theme::{elevated_glass_fill, glass_fill, glass_stroke, muted_fill, word_heat_fill};

const MAX_HEATMAP_WORDS: usize = 240;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) enum SidebarTab {
    #[default]
    Outline,
    Words,
}

#[derive(Default)]
pub(crate) struct ToolbarActions {
    pub open: bool,
    pub back: bool,
    pub sidebar: bool,
    pub search: bool,
    pub zoom_in: bool,
    pub zoom_out: bool,
    pub zoom_reset: bool,
}

pub(crate) struct ToolbarModel<'a> {
    pub document_path: Option<&'a Path>,
    pub can_go_back: bool,
    pub sidebar_open: bool,
    pub search_open: bool,
    pub font_scale: f32,
}

pub(crate) fn toolbar(ui: &mut egui::Ui, model: ToolbarModel<'_>) -> ToolbarActions {
    let mut actions = ToolbarActions::default();
    let document_open = model.document_path.is_some();
    let toolbar_width = ui.available_width();

    ui.add_space(10.0);
    Frame::new()
        .fill(elevated_glass_fill(ui))
        .stroke(glass_stroke(ui))
        .corner_radius(CornerRadius::same(18))
        .inner_margin(Margin::symmetric(12, 9))
        .show(ui, |ui| {
            ui.set_min_width((toolbar_width - 24.0).max(0.0));
            ui.horizontal(|ui| {
                Frame::new()
                    .fill(ui.visuals().selection.bg_fill)
                    .corner_radius(CornerRadius::same(9))
                    .inner_margin(Margin::symmetric(9, 5))
                    .show(ui, |ui| {
                        ui.label(
                            RichText::new("MD")
                                .font(FontId::new(12.0, FontFamily::Monospace))
                                .strong(),
                        );
                    });

                if ui
                    .add_enabled(model.can_go_back, egui::Button::new("<"))
                    .on_hover_text("Back (Alt+Left)")
                    .clicked()
                {
                    actions.back = true;
                }
                if ui
                    .button("Open")
                    .on_hover_text("Open Markdown (Ctrl+O)")
                    .clicked()
                {
                    actions.open = true;
                }

                ui.add_enabled_ui(document_open, |ui| {
                    if ui
                        .add(egui::Button::new("Outline").selected(model.sidebar_open))
                        .on_hover_text("Document outline and word heatmap")
                        .clicked()
                    {
                        actions.sidebar = true;
                    }
                    if ui
                        .add(egui::Button::new("Find").selected(model.search_open))
                        .on_hover_text("Find in document (Ctrl+F)")
                        .clicked()
                    {
                        actions.search = true;
                    }
                    ui.menu_button("Aa", |ui| {
                        ui.horizontal(|ui| {
                            if ui.button("A−").on_hover_text("Ctrl+-").clicked() {
                                actions.zoom_out = true;
                                ui.close();
                            }
                            ui.label(format!("{}%", (model.font_scale * 100.0).round()));
                            if ui.button("A+").on_hover_text("Ctrl++").clicked() {
                                actions.zoom_in = true;
                                ui.close();
                            }
                        });
                        if ui.button("Reset to 100%  ·  Ctrl+0").clicked() {
                            actions.zoom_reset = true;
                            ui.close();
                        }
                    });
                });

                let (path_text, path_hint) = model.document_path.map_or_else(
                    || ("No document".to_owned(), "No document open".to_owned()),
                    |path| {
                        let short = path
                            .file_name()
                            .and_then(|name| name.to_str())
                            .map(str::to_owned)
                            .unwrap_or_else(|| path.to_string_lossy().into_owned());
                        (short, path.to_string_lossy().into_owned())
                    },
                );
                ui.separator();
                ui.add_sized(
                    [ui.available_width().max(48.0), 25.0],
                    Label::new(RichText::new(path_text).weak()).truncate(),
                )
                .on_hover_text(path_hint);
            });
        });
    ui.add_space(4.0);
    actions
}

pub(crate) fn error_banner(ui: &mut egui::Ui, message: Option<&str>) {
    let Some(message) = message else {
        return;
    };

    let fill = if ui.visuals().dark_mode {
        egui::Color32::from_rgb(69, 38, 42)
    } else {
        egui::Color32::from_rgb(255, 232, 234)
    };
    Frame::new()
        .fill(fill)
        .stroke(Stroke::new(
            1.0,
            egui::Color32::from_rgba_unmultiplied(255, 120, 140, 70),
        ))
        .corner_radius(CornerRadius::same(12))
        .inner_margin(Margin::symmetric(12, 9))
        .show(ui, |ui| {
            ui.colored_label(ui.visuals().error_fg_color, message);
        });
    ui.add_space(8.0);
}

pub(crate) struct SearchPanelOutput {
    pub selected_anchor: Option<String>,
    pub close_requested: bool,
}

pub(crate) fn search_panel(
    ui: &mut egui::Ui,
    index: Option<&DocumentIndex>,
    query: &mut String,
    focus_requested: &mut bool,
    font_scale: f32,
) -> SearchPanelOutput {
    let mut output = SearchPanelOutput {
        selected_anchor: None,
        close_requested: false,
    };
    let summary = index
        .map(|index| search_sections(index, query))
        .unwrap_or_default();

    Frame::new()
        .fill(elevated_glass_fill(ui))
        .stroke(glass_stroke(ui))
        .corner_radius(CornerRadius::same(15))
        .inner_margin(Margin::symmetric(12, 10))
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label(RichText::new("Find").strong());
                let response = ui.add(
                    TextEdit::singleline(query)
                        .hint_text("Search this document…")
                        .desired_width((ui.available_width() - 190.0).max(110.0)),
                );
                if *focus_requested {
                    response.request_focus();
                    *focus_requested = false;
                }
                ui.label(
                    RichText::new(format!("{} matches", summary.total_matches))
                        .small()
                        .weak(),
                );
                if ui.small_button("×").on_hover_text("Close (Esc)").clicked() {
                    output.close_requested = true;
                }
            });

            if !query.trim().is_empty() {
                if summary.results.is_empty() {
                    ui.add_space(4.0);
                    ui.label(RichText::new("No matches").weak());
                } else {
                    ui.add_space(6.0);
                    ScrollArea::vertical()
                        .id_salt("search-results")
                        .max_height(180.0)
                        .show(ui, |ui| {
                            for result in summary.results {
                                let label =
                                    format!("{}  ·  {}", result.section_title, result.snippet);
                                let response = ui.add_enabled(
                                    result.anchor.is_some(),
                                    egui::Button::new(RichText::new(label).size(13.0 * font_scale))
                                        .wrap(),
                                );
                                let hint = format!("{} matches in this section", result.matches);
                                if response.on_hover_text(hint).clicked() {
                                    output.selected_anchor = result.anchor;
                                }
                            }
                        });
                }
            }
        });
    ui.add_space(6.0);
    output
}

pub(crate) fn sidebar_tabs(ui: &mut egui::Ui, active: &mut SidebarTab) {
    ui.horizontal(|ui| {
        ui.selectable_value(active, SidebarTab::Outline, "Contents");
        ui.selectable_value(active, SidebarTab::Words, "Words");
    });
    ui.separator();
    ui.add_space(2.0);
}

pub(crate) fn outline_panel(
    ui: &mut egui::Ui,
    headings: &[Heading],
    font_scale: f32,
) -> Option<String> {
    let mut selected_anchor = None;
    if headings.is_empty() {
        ui.label(RichText::new("No headings in this document").weak());
        return None;
    }

    ScrollArea::vertical()
        .id_salt("document-outline")
        .auto_shrink([true, false])
        .show(ui, |ui| {
            ui.vertical(|ui| {
                for heading in headings {
                    ui.horizontal(|ui| {
                        ui.add_space((heading.level.saturating_sub(1) as f32) * 12.0);
                        let available = ui.available_width().max(40.0);
                        if ui
                            .add_sized(
                                [available, 26.0],
                                egui::Button::new(
                                    RichText::new(&heading.title).size(13.0 * font_scale),
                                )
                                .truncate(),
                            )
                            .clicked()
                        {
                            selected_anchor = Some(heading.anchor.clone());
                        }
                    });
                }
            });
        });
    selected_anchor
}

pub(crate) fn word_heatmap_panel(ui: &mut egui::Ui, heatmap: &WordHeatmap, font_scale: f32) {
    ui.label(
        RichText::new(format!(
            "{} words  ·  {} unique",
            heatmap.total_words,
            heatmap.entries.len()
        ))
        .strong(),
    );
    ui.label(
        RichText::new("Visible Markdown text · logarithmic intensity")
            .size(11.0 * font_scale)
            .weak(),
    );
    ui.add_space(5.0);

    ui.horizontal(|ui| {
        ui.label(RichText::new("rare").size(10.0).weak());
        for heat in [0.15, 0.35, 0.55, 0.75, 1.0] {
            Frame::new()
                .fill(word_heat_fill(ui, heat))
                .corner_radius(CornerRadius::same(3))
                .show(ui, |ui| {
                    ui.allocate_space(Vec2::new(13.0, 8.0));
                });
        }
        ui.label(RichText::new("frequent").size(10.0).weak());
    });
    ui.add_space(5.0);

    if heatmap.entries.is_empty() {
        ui.label(RichText::new("No words found").weak());
        return;
    }
    if heatmap.entries.len() > MAX_HEATMAP_WORDS {
        ui.label(
            RichText::new(format!("Showing top {MAX_HEATMAP_WORDS}"))
                .size(10.0)
                .weak(),
        );
    }

    ScrollArea::vertical()
        .id_salt("document-word-heatmap")
        .auto_shrink([true, false])
        .show(ui, |ui| {
            let columns = if ui.available_width() >= 520.0 {
                4
            } else if ui.available_width() >= 350.0 {
                3
            } else {
                2
            };
            let spacing = 5.0;
            let tile_width = ((ui.available_width() - spacing * (columns - 1) as f32)
                / columns as f32)
                .max(68.0);
            egui::Grid::new("word-heatmap-grid")
                .num_columns(columns)
                .spacing(Vec2::splat(spacing))
                .show(ui, |ui| {
                    for (position, entry) in
                        heatmap.entries.iter().take(MAX_HEATMAP_WORDS).enumerate()
                    {
                        ui.add_sized(
                            [tile_width, 28.0],
                            egui::Button::new(
                                RichText::new(format!("{} · {}", entry.word, entry.count))
                                    .size(11.5 * font_scale),
                            )
                            .fill(word_heat_fill(ui, entry.heat))
                            .truncate(),
                        )
                        .on_hover_text(format!(
                            "{}\n{} occurrences\n{:.2}% of visible words\nheat {:.3}",
                            entry.word,
                            entry.count,
                            entry.share * 100.0,
                            entry.heat
                        ));
                        if position % columns == columns - 1 {
                            ui.end_row();
                        }
                    }
                });
        });
}

pub(crate) fn document_header(
    ui: &mut egui::Ui,
    path: &Path,
    heading_count: usize,
    word_count: usize,
    font_scale: f32,
) {
    Frame::new()
        .fill(muted_fill(ui))
        .corner_radius(CornerRadius::same(12))
        .inner_margin(Margin::symmetric(12, 8))
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                let title = path
                    .file_stem()
                    .and_then(|value| value.to_str())
                    .unwrap_or("Markdown document");
                ui.label(RichText::new(title).strong());
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    ui.label(
                        RichText::new(format!("{}%", (font_scale * 100.0).round()))
                            .small()
                            .weak(),
                    );
                    ui.label(
                        RichText::new(format!("{word_count} words · {heading_count} headings"))
                            .small()
                            .weak(),
                    );
                });
            });
        });
    ui.add_space(8.0);
}

pub(crate) fn empty_state(ui: &mut egui::Ui) -> bool {
    let mut open_requested = false;
    let card_width = ui.available_width().min(460.0);
    let card_height = 222.0;
    let spacer = ((ui.available_height() - card_height) / 2.0).max(24.0);
    ui.add_space(spacer);
    ui.with_layout(Layout::top_down(Align::Center), |ui| {
        ui.allocate_ui_with_layout(
            Vec2::new(card_width, card_height),
            Layout::top_down(Align::Center),
            |ui| {
                Frame::new()
                    .fill(elevated_glass_fill(ui))
                    .stroke(glass_stroke(ui))
                    .corner_radius(CornerRadius::same(24))
                    .inner_margin(Margin::symmetric(30, 26))
                    .show(ui, |ui| {
                        ui.set_width(card_width - 60.0);
                        ui.set_min_height(card_height - 52.0);
                        ui.vertical_centered(|ui| {
                            ui.add_sized(
                                [48.0, 34.0],
                                egui::Button::new(
                                    RichText::new("MD")
                                        .font(FontId::new(14.0, FontFamily::Monospace))
                                        .strong(),
                                )
                                .fill(ui.visuals().selection.bg_fill),
                            );
                            ui.add_space(10.0);
                            ui.heading("Read Markdown. Nothing else.");
                            ui.add_space(5.0);
                            ui.label(
                                RichText::new("Drop a .md file anywhere in this window").weak(),
                            );
                            ui.add_space(15.0);
                            if ui.button("Open Markdown").clicked() {
                                open_requested = true;
                            }
                        });
                    });
            },
        );
    });
    open_requested
}

pub(crate) fn reader_card_frame(ui: &egui::Ui) -> Frame {
    Frame::new()
        .fill(glass_fill(ui))
        .stroke(glass_stroke(ui))
        .corner_radius(CornerRadius::same(22))
        .inner_margin(Margin::symmetric(14, 12))
}
