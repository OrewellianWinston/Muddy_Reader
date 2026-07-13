#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::path::{Path, PathBuf};

use eframe::egui::{
    self, Align, Color32, CornerRadius, FontFamily, FontId, Frame, Key, KeyboardShortcut, Label,
    Layout, Margin, Modifiers, RichText, ScrollArea, Stroke, TextEdit, TextStyle, Theme,
    ThemePreference, Vec2, ViewportBuilder, ViewportCommand, pos2,
};
use egui_commonmark::{CommonMarkCache, CommonMarkViewer};
use md_reader::{
    LinkKind, LoadedDocument, NavigationEntry, NavigationHistory, classify_link, load_document,
    search_sections,
};

const APP_NAME: &str = "MD Reader";
const DEFAULT_WINDOW_SIZE: Vec2 = Vec2::new(960.0, 720.0);
const MIN_WINDOW_SIZE: Vec2 = Vec2::new(480.0, 320.0);
const MAX_READING_WIDTH: f32 = 980.0;
const MAX_CARD_WIDTH: f32 = 1080.0;
const MIN_FONT_SCALE: f32 = 0.8;
const MAX_FONT_SCALE: f32 = 1.5;
const FONT_SCALE_STEP: f32 = 0.1;

#[derive(Default)]
struct ToolbarActions {
    open: bool,
    back: bool,
    outline: bool,
    search: bool,
    zoom_in: bool,
    zoom_out: bool,
    zoom_reset: bool,
}

fn main() -> eframe::Result {
    let initial_path = std::env::args_os().nth(1).map(PathBuf::from);
    let native_options = eframe::NativeOptions {
        viewport: ViewportBuilder::default()
            .with_inner_size(DEFAULT_WINDOW_SIZE)
            .with_min_inner_size(MIN_WINDOW_SIZE)
            .with_resizable(true),
        renderer: eframe::Renderer::Glow,
        ..Default::default()
    };

    eframe::run_native(
        APP_NAME,
        native_options,
        Box::new(move |creation_context| {
            Ok(Box::new(MdReaderApp::new(creation_context, initial_path)))
        }),
    )
}

struct MdReaderApp {
    document: Option<LoadedDocument>,
    markdown_cache: CommonMarkCache,
    history: NavigationHistory,
    error_message: Option<String>,
    current_scroll_offset: f32,
    restore_scroll_offset: Option<f32>,
    show_outline: bool,
    show_search: bool,
    search_query: String,
    search_focus_requested: bool,
    font_scale: f32,
}

impl MdReaderApp {
    fn new(context: &eframe::CreationContext<'_>, initial_path: Option<PathBuf>) -> Self {
        configure_style(&context.egui_ctx);

        let mut app = Self {
            document: None,
            markdown_cache: CommonMarkCache::default(),
            history: NavigationHistory::default(),
            error_message: None,
            current_scroll_offset: 0.0,
            restore_scroll_offset: None,
            show_outline: false,
            show_search: false,
            search_query: String::new(),
            search_focus_requested: false,
            font_scale: 1.0,
        };

        if let Some(path) = initial_path {
            app.open_explicit(path, &context.egui_ctx);
        }

        app
    }

    fn install_document(
        &mut self,
        document: LoadedDocument,
        scroll_offset: f32,
        context: &egui::Context,
    ) {
        self.markdown_cache = CommonMarkCache::default();
        for destination in &document.intercepted_links {
            self.markdown_cache.add_link_hook(destination.clone());
        }

        let document_title = document
            .path
            .file_name()
            .and_then(|name| name.to_str())
            .map(str::to_owned)
            .unwrap_or_else(|| document.path.to_string_lossy().into_owned());
        context.send_viewport_cmd(ViewportCommand::Title(format!(
            "{APP_NAME} - {document_title}"
        )));
        self.document = Some(document);
        self.error_message = None;
        self.current_scroll_offset = scroll_offset;
        self.restore_scroll_offset = Some(scroll_offset);
        self.show_outline = false;
        self.show_search = false;
        self.search_query.clear();
        self.search_focus_requested = false;
    }

    fn set_font_scale(&mut self, scale: f32, context: &egui::Context) {
        self.font_scale = scale.clamp(MIN_FONT_SCALE, MAX_FONT_SCALE);
        apply_font_scale(context, self.font_scale);
    }

    fn request_scroll_to_heading(&mut self, anchor: String) {
        *self.markdown_cache.scroll_to_id_target_mut() = Some(anchor);
    }

    fn open_explicit(&mut self, path: impl AsRef<Path>, context: &egui::Context) {
        match load_document(path) {
            Ok(document) => {
                self.history.clear();
                self.install_document(document, 0.0, context);
            }
            Err(error) => self.error_message = Some(error.to_string()),
        }
    }

    fn follow_local_link(&mut self, path: PathBuf, context: &egui::Context) {
        match load_document(path) {
            Ok(next_document) => {
                if let Some(current_document) = &self.document {
                    self.history.push(NavigationEntry {
                        path: current_document.path.clone(),
                        scroll_offset: self.current_scroll_offset,
                    });
                }
                self.install_document(next_document, 0.0, context);
            }
            Err(error) => self.error_message = Some(error.to_string()),
        }
    }

    fn go_back(&mut self, context: &egui::Context) {
        let Some(entry) = self.history.last().cloned() else {
            return;
        };

        match load_document(&entry.path) {
            Ok(previous_document) => {
                self.history.pop();
                self.install_document(previous_document, entry.scroll_offset, context);
            }
            Err(error) => self.error_message = Some(error.to_string()),
        }
    }

    fn show_open_dialog(&mut self, context: &egui::Context) {
        if let Some(path) = rfd::FileDialog::new()
            .add_filter("Markdown", &["md", "markdown"])
            .set_title("Open Markdown document")
            .pick_file()
        {
            self.open_explicit(path, context);
        }
    }

    fn handle_dropped_files(&mut self, context: &egui::Context) {
        let dropped_paths = context.input(|input| {
            input
                .raw
                .dropped_files
                .iter()
                .filter_map(|file| file.path.clone())
                .collect::<Vec<_>>()
        });

        if dropped_paths.is_empty() {
            return;
        }

        if let Some(path) = dropped_paths
            .iter()
            .find(|path| md_reader::is_markdown_path(path))
        {
            self.open_explicit(path, context);
        } else {
            self.error_message = Some("Drop a .md or .markdown file to open it.".to_owned());
        }
    }

    fn show_toolbar(&mut self, ui: &mut egui::Ui) -> ToolbarActions {
        let mut actions = ToolbarActions::default();
        let toolbar_width = ui.available_width();
        let document_open = self.document.is_some();

        ui.add_space(12.0);
        Frame::new()
            .fill(glass_fill(ui))
            .stroke(glass_stroke(ui))
            .corner_radius(CornerRadius::same(16))
            .inner_margin(Margin::symmetric(16, 10))
            .show(ui, |ui| {
                ui.set_min_width((toolbar_width - 32.0).max(0.0));
                ui.set_height(30.0);
                ui.horizontal(|ui| {
                    if ui
                        .add_enabled(!self.history.is_empty(), egui::Button::new("< Back"))
                        .on_hover_text("Back (Alt+Left)")
                        .clicked()
                    {
                        actions.back = true;
                    }

                    if ui
                        .button("Open...")
                        .on_hover_text("Open Markdown (Ctrl+O)")
                        .clicked()
                    {
                        actions.open = true;
                    }

                    ui.add_enabled_ui(document_open, |ui| {
                        if ui
                            .button("Outline")
                            .on_hover_text("Show headings")
                            .clicked()
                        {
                            actions.outline = true;
                        }
                        if ui
                            .button("Search")
                            .on_hover_text("Find in document (Ctrl+F)")
                            .clicked()
                        {
                            actions.search = true;
                        }
                        ui.menu_button("Aa", |ui| {
                            ui.horizontal(|ui| {
                                if ui
                                    .button("A-")
                                    .on_hover_text("Smaller text (Ctrl+-)")
                                    .clicked()
                                {
                                    actions.zoom_out = true;
                                    ui.close();
                                }
                                ui.label(format!("{}%", (self.font_scale * 100.0).round()));
                                if ui
                                    .button("A+")
                                    .on_hover_text("Larger text (Ctrl++)")
                                    .clicked()
                                {
                                    actions.zoom_in = true;
                                    ui.close();
                                }
                            });
                            if ui.button("Reset size (Ctrl+0)").clicked() {
                                actions.zoom_reset = true;
                                ui.close();
                            }
                        });
                    });

                    ui.separator();
                    let path_text = self
                        .document
                        .as_ref()
                        .map(|document| document.path.to_string_lossy().into_owned())
                        .unwrap_or_else(|| "No document open".to_owned());
                    ui.add_sized(
                        [ui.available_width(), 24.0],
                        Label::new(RichText::new(&path_text).weak()).truncate(),
                    )
                    .on_hover_text(path_text);
                });
            });
        ui.add_space(4.0);

        actions
    }

    fn show_error(&self, ui: &mut egui::Ui) {
        let Some(message) = &self.error_message else {
            return;
        };

        let dark_mode = ui.visuals().dark_mode;
        let error_color = ui.visuals().error_fg_color;
        let fill = if dark_mode {
            Color32::from_rgb(69, 38, 42)
        } else {
            Color32::from_rgb(255, 232, 234)
        };
        Frame::new()
            .fill(fill)
            .stroke(Stroke::new(
                1.0,
                Color32::from_rgba_unmultiplied(255, 120, 140, 70),
            ))
            .corner_radius(CornerRadius::same(12))
            .inner_margin(Margin::symmetric(12, 9))
            .show(ui, |ui| {
                ui.colored_label(error_color, message);
            });
        ui.add_space(8.0);
    }

    fn show_search_panel(&mut self, ui: &mut egui::Ui) -> Option<String> {
        if !self.show_search {
            return None;
        }

        let mut selected_anchor = None;
        Frame::new()
            .fill(glass_fill(ui))
            .stroke(glass_stroke(ui))
            .corner_radius(CornerRadius::same(14))
            .inner_margin(Margin::symmetric(12, 10))
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    let response = ui.add(
                        TextEdit::singleline(&mut self.search_query)
                            .hint_text("Find in document…")
                            .desired_width((ui.available_width() - 130.0).max(140.0)),
                    );
                    if self.search_focus_requested {
                        response.request_focus();
                        self.search_focus_requested = false;
                    }

                    let total_matches = self
                        .document
                        .as_ref()
                        .map(|document| {
                            search_sections(&document.index, &self.search_query).total_matches
                        })
                        .unwrap_or_default();
                    ui.label(
                        RichText::new(format!(
                            "{total_matches} {}",
                            if total_matches == 1 {
                                "match"
                            } else {
                                "matches"
                            }
                        ))
                        .weak(),
                    );
                    if ui
                        .small_button("×")
                        .on_hover_text("Close search (Esc)")
                        .clicked()
                    {
                        self.show_search = false;
                        self.search_query.clear();
                    }
                });

                let summary = self
                    .document
                    .as_ref()
                    .map(|document| search_sections(&document.index, &self.search_query))
                    .unwrap_or_default();
                if !self.search_query.trim().is_empty() {
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
                                        egui::Button::new(
                                            RichText::new(label).size(13.0 * self.font_scale),
                                        )
                                        .wrap(),
                                    );
                                    let hint = if result.matches == 1 {
                                        "1 match in this section".to_owned()
                                    } else {
                                        format!("{} matches in this section", result.matches)
                                    };
                                    if response.on_hover_text(hint).clicked() {
                                        selected_anchor = result.anchor;
                                    }
                                }
                            });
                    }
                }
            });
        ui.add_space(6.0);
        selected_anchor
    }

    fn show_outline_panel(&mut self, ui: &mut egui::Ui) -> Option<String> {
        let headings = self
            .document
            .as_ref()
            .map(|document| document.index.headings.clone())?;
        let mut selected_anchor = None;
        ui.label(RichText::new("Outline").strong());
        ui.add_space(4.0);
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
                                    [available, 24.0],
                                    egui::Button::new(
                                        RichText::new(heading.title).size(13.0 * self.font_scale),
                                    )
                                    .truncate(),
                                )
                                .clicked()
                            {
                                selected_anchor = Some(heading.anchor);
                            }
                        });
                    }
                });
            });
        selected_anchor
    }

    fn show_empty_state(&self, ui: &mut egui::Ui) -> bool {
        let mut open_requested = false;
        let card_width = ui.available_width().min(440.0);
        let card_height = 208.0;
        let spacer = ((ui.available_height() - card_height) / 2.0).max(24.0);
        ui.add_space(spacer);
        ui.with_layout(Layout::top_down(Align::Center), |ui| {
            Frame::new()
                .fill(glass_fill(ui))
                .stroke(glass_stroke(ui))
                .corner_radius(CornerRadius::same(20))
                .inner_margin(Margin::symmetric(28, 24))
                .show(ui, |ui| {
                    ui.set_min_width(card_width - 56.0);
                    ui.set_min_height(card_height - 48.0);
                    ui.vertical_centered(|ui| {
                        ui.label(
                            RichText::new("MD")
                                .font(FontId::new(15.0, FontFamily::Monospace))
                                .color(ui.visuals().hyperlink_color),
                        );
                        ui.add_space(8.0);
                        ui.heading(APP_NAME);
                        ui.add_space(6.0);
                        ui.label(RichText::new("Drop a Markdown file here").weak());
                        ui.add_space(16.0);
                        if ui.button("Open Markdown...").clicked() {
                            open_requested = true;
                        }
                    });
                });
        });
        open_requested
    }

    fn show_document(&mut self, ui: &mut egui::Ui) -> Option<String> {
        let card_width = (ui.available_width() - 32.0).clamp(320.0, MAX_CARD_WIDTH);
        let card_height = (ui.available_height() - 16.0).max(180.0);
        let mut clicked_link = None;

        ui.with_layout(Layout::top_down(Align::Center), |ui| {
            Frame::new()
                .fill(glass_fill(ui))
                .stroke(glass_stroke(ui))
                .corner_radius(CornerRadius::same(20))
                .inner_margin(Margin::symmetric(14, 12))
                .show(ui, |ui| {
                    ui.set_width(card_width - 28.0);
                    ui.set_min_height(card_height - 24.0);
                    ui.horizontal_top(|ui| {
                        if self.show_outline {
                            let outline_height = ui.available_height().max(160.0);
                            ui.allocate_ui_with_layout(
                                Vec2::new(218.0, outline_height),
                                Layout::top_down(Align::Min),
                                |ui| {
                                    if let Some(anchor) = self.show_outline_panel(ui) {
                                        self.request_scroll_to_heading(anchor);
                                    }
                                },
                            );
                            ui.separator();
                        }
                        let reader_size = ui.available_size();
                        ui.allocate_ui_with_layout(
                            reader_size,
                            Layout::top_down(Align::Min),
                            |ui| clicked_link = self.show_document_scroll(ui),
                        );
                    });
                });
        });

        clicked_link
    }

    fn show_document_scroll(&mut self, ui: &mut egui::Ui) -> Option<String> {
        let document = self.document.as_ref()?;
        let reading_width = (ui.available_width() - 48.0).clamp(280.0, MAX_READING_WIDTH);
        let mut scroll_area = ScrollArea::vertical()
            .id_salt("document-scroll")
            .hscroll(true)
            .auto_shrink([false, false])
            .content_margin(Margin::symmetric(24, 18));

        if let Some(offset) = self.restore_scroll_offset.take() {
            scroll_area = scroll_area.vertical_scroll_offset(offset);
        }

        let output = scroll_area.show(ui, |ui| {
            ui.set_width(reading_width);
            CommonMarkViewer::new()
                .default_implicit_uri_scheme(&document.image_base_uri)
                .max_image_width(Some(reading_width.max(1.0) as usize))
                .enable_scroll_to_heading(true)
                .show(
                    ui,
                    &mut self.markdown_cache,
                    &document.index.render_markdown,
                );
        });
        self.current_scroll_offset = output.state.offset.y;

        document
            .intercepted_links
            .iter()
            .find(|destination| {
                self.markdown_cache.get_link_hook(destination.as_str()) == Some(true)
            })
            .cloned()
    }
}

impl eframe::App for MdReaderApp {
    fn ui(&mut self, root_ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let context = root_ui.ctx().clone();
        self.handle_dropped_files(&context);

        let shortcut_open = context.input_mut(|input| {
            input.consume_shortcut(&KeyboardShortcut::new(Modifiers::CTRL, Key::O))
        });
        let shortcut_back =
            context.input_mut(|input| input.consume_key(Modifiers::ALT, Key::ArrowLeft));
        let shortcut_search = self.document.is_some()
            && context.input_mut(|input| {
                input.consume_shortcut(&KeyboardShortcut::new(Modifiers::CTRL, Key::F))
            });
        let shortcut_zoom_in = self.document.is_some()
            && context.input_mut(|input| {
                input.consume_shortcut(&KeyboardShortcut::new(Modifiers::CTRL, Key::Plus))
                    || input.consume_shortcut(&KeyboardShortcut::new(Modifiers::CTRL, Key::Equals))
            });
        let shortcut_zoom_out = self.document.is_some()
            && context.input_mut(|input| {
                input.consume_shortcut(&KeyboardShortcut::new(Modifiers::CTRL, Key::Minus))
            });
        let shortcut_zoom_reset = self.document.is_some()
            && context.input_mut(|input| {
                input.consume_shortcut(&KeyboardShortcut::new(Modifiers::CTRL, Key::Num0))
            });
        let close_search = self.show_search
            && context.input_mut(|input| input.consume_key(Modifiers::NONE, Key::Escape));
        if shortcut_search {
            self.show_search = true;
            self.search_focus_requested = true;
        }
        if close_search {
            self.show_search = false;
            self.search_query.clear();
            self.search_focus_requested = false;
        }

        let mut toolbar_actions = ToolbarActions::default();
        let mut empty_open = false;
        let mut clicked_link = None;
        let available_size = root_ui.available_size();
        paint_liquid_background(root_ui);
        root_ui.allocate_ui_with_layout(available_size, Layout::top_down(Align::Min), |ui| {
            toolbar_actions = self.show_toolbar(ui);
            ui.add_space(8.0);
            self.show_error(ui);
            if let Some(anchor) = self.show_search_panel(ui) {
                self.request_scroll_to_heading(anchor);
            }
            if self.document.is_some() {
                clicked_link = self.show_document(ui);
            } else {
                empty_open = self.show_empty_state(ui);
            }
        });

        if shortcut_back || toolbar_actions.back {
            self.go_back(&context);
        }
        if shortcut_open || toolbar_actions.open || empty_open {
            self.show_open_dialog(&context);
        }
        if toolbar_actions.outline {
            self.show_outline = !self.show_outline;
        }
        if toolbar_actions.search {
            self.show_search = !self.show_search;
            self.search_focus_requested = self.show_search;
            if !self.show_search {
                self.search_query.clear();
            }
        }
        if shortcut_zoom_in || toolbar_actions.zoom_in {
            self.set_font_scale(self.font_scale + FONT_SCALE_STEP, &context);
        }
        if shortcut_zoom_out || toolbar_actions.zoom_out {
            self.set_font_scale(self.font_scale - FONT_SCALE_STEP, &context);
        }
        if shortcut_zoom_reset || toolbar_actions.zoom_reset {
            self.set_font_scale(1.0, &context);
        }

        if let (Some(document), Some(destination)) = (&self.document, clicked_link)
            && let LinkKind::LocalMarkdown(path) = classify_link(&document.path, &destination)
        {
            self.follow_local_link(path, &context);
        }
    }
}

fn configure_style(context: &egui::Context) {
    context.set_theme(ThemePreference::System);
    context.all_styles_mut(|style| {
        style.spacing.item_spacing = Vec2::new(8.0, 8.0);
        style.spacing.button_padding = Vec2::new(10.0, 5.0);
        style.url_in_tooltip = true;
    });

    context.style_mut_of(Theme::Dark, |style| {
        let visuals = &mut style.visuals;
        visuals.panel_fill = Color32::from_rgb(18, 21, 32);
        visuals.window_fill = Color32::from_rgb(24, 28, 43);
        visuals.faint_bg_color = Color32::from_rgb(42, 45, 63);
        visuals.extreme_bg_color = Color32::from_rgb(13, 16, 26);
        visuals.code_bg_color = Color32::from_rgb(20, 24, 38);
        visuals.hyperlink_color = Color32::from_rgb(166, 153, 255);
        visuals.selection.bg_fill = Color32::from_rgb(91, 70, 170);
        visuals.window_stroke =
            Stroke::new(1.0, Color32::from_rgba_unmultiplied(210, 220, 255, 40));
        style_widgets(style, true);
    });

    context.style_mut_of(Theme::Light, |style| {
        let visuals = &mut style.visuals;
        visuals.panel_fill = Color32::from_rgb(232, 237, 249);
        visuals.window_fill = Color32::from_rgb(247, 249, 255);
        visuals.faint_bg_color = Color32::from_rgb(220, 226, 243);
        visuals.extreme_bg_color = Color32::from_rgb(235, 239, 249);
        visuals.code_bg_color = Color32::from_rgb(238, 241, 249);
        visuals.hyperlink_color = Color32::from_rgb(87, 73, 190);
        visuals.selection.bg_fill = Color32::from_rgb(205, 195, 255);
        visuals.window_stroke = Stroke::new(1.0, Color32::from_rgba_unmultiplied(74, 89, 153, 70));
        style_widgets(style, false);
    });

    apply_font_scale(context, 1.0);
}

fn apply_font_scale(context: &egui::Context, scale: f32) {
    context.all_styles_mut(|style| {
        style.text_styles.insert(
            TextStyle::Body,
            FontId::new(16.0 * scale, FontFamily::Proportional),
        );
        style.text_styles.insert(
            TextStyle::Monospace,
            FontId::new(14.0 * scale, FontFamily::Monospace),
        );
        style.text_styles.insert(
            TextStyle::Button,
            FontId::new(14.0 * scale, FontFamily::Proportional),
        );
        style.text_styles.insert(
            TextStyle::Heading,
            FontId::new(28.0 * scale, FontFamily::Proportional),
        );
    });
}

fn style_widgets(style: &mut egui::Style, dark: bool) {
    let widgets = &mut style.visuals.widgets;
    let radius = CornerRadius::same(10);
    widgets.noninteractive.corner_radius = radius;
    widgets.inactive.corner_radius = radius;
    widgets.hovered.corner_radius = radius;
    widgets.active.corner_radius = radius;
    widgets.open.corner_radius = radius;

    if dark {
        widgets.inactive.weak_bg_fill = Color32::from_rgba_unmultiplied(255, 255, 255, 18);
        widgets.hovered.weak_bg_fill = Color32::from_rgba_unmultiplied(172, 157, 255, 45);
        widgets.active.weak_bg_fill = Color32::from_rgba_unmultiplied(172, 157, 255, 65);
    } else {
        widgets.inactive.weak_bg_fill = Color32::from_rgba_unmultiplied(255, 255, 255, 115);
        widgets.hovered.weak_bg_fill = Color32::from_rgba_unmultiplied(124, 111, 220, 35);
        widgets.active.weak_bg_fill = Color32::from_rgba_unmultiplied(124, 111, 220, 55);
    }
}

fn glass_fill(ui: &egui::Ui) -> Color32 {
    if ui.visuals().dark_mode {
        Color32::from_rgba_unmultiplied(31, 37, 58, 215)
    } else {
        Color32::from_rgba_unmultiplied(255, 255, 255, 175)
    }
}

fn glass_stroke(ui: &egui::Ui) -> Stroke {
    if ui.visuals().dark_mode {
        Stroke::new(1.0, Color32::from_rgba_unmultiplied(215, 224, 255, 58))
    } else {
        Stroke::new(1.0, Color32::from_rgba_unmultiplied(90, 105, 165, 74))
    }
}

fn paint_liquid_background(ui: &mut egui::Ui) {
    let rect = ui.available_rect_before_wrap();
    let painter = ui.painter();
    let dark = ui.visuals().dark_mode;
    let base = if dark {
        Color32::from_rgb(18, 21, 32)
    } else {
        Color32::from_rgb(232, 237, 249)
    };
    painter.rect_filled(rect, 0.0, base);

    if dark {
        painter.circle_filled(
            pos2(rect.left() + rect.width() * 0.12, rect.top() + 80.0),
            230.0,
            Color32::from_rgba_unmultiplied(124, 101, 255, 35),
        );
        painter.circle_filled(
            pos2(rect.right() - 80.0, rect.bottom() - 30.0),
            300.0,
            Color32::from_rgba_unmultiplied(51, 154, 255, 29),
        );
    } else {
        painter.circle_filled(
            pos2(rect.left() + rect.width() * 0.12, rect.top() + 80.0),
            230.0,
            Color32::from_rgba_unmultiplied(145, 126, 255, 52),
        );
        painter.circle_filled(
            pos2(rect.right() - 80.0, rect.bottom() - 30.0),
            300.0,
            Color32::from_rgba_unmultiplied(76, 166, 255, 42),
        );
    }
}
