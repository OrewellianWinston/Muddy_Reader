use std::path::{Path, PathBuf};

use eframe::egui::{
    self, Align, Key, KeyboardShortcut, Layout, Margin, Modifiers, ScrollArea, Vec2,
    ViewportBuilder, ViewportCommand,
};
use egui_commonmark::{CommonMarkCache, CommonMarkViewer};
use md_reader::{
    DocumentLoader, FileDocumentLoader, LinkKind, LoadedDocument, NavigationEntry,
    NavigationHistory, classify_link,
};

use crate::ui::components::{self, SidebarTab, ToolbarActions, ToolbarModel, reader_card_frame};
use crate::ui::theme;

const APP_NAME: &str = "MD Reader";
const DEFAULT_WINDOW_SIZE: Vec2 = Vec2::new(960.0, 720.0);
const MIN_WINDOW_SIZE: Vec2 = Vec2::new(480.0, 320.0);
const MAX_READING_WIDTH: f32 = 980.0;
const MAX_CARD_WIDTH: f32 = 1120.0;
const SIDEBAR_WIDTH: f32 = 252.0;
const SIDEBAR_DRAWER_BREAKPOINT: f32 = 760.0;
const MIN_FONT_SCALE: f32 = 0.8;
const MAX_FONT_SCALE: f32 = 1.5;
const FONT_SCALE_STEP: f32 = 0.1;

pub fn run() -> eframe::Result {
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
    loader: Box<dyn DocumentLoader>,
    document: Option<LoadedDocument>,
    markdown_cache: CommonMarkCache,
    history: NavigationHistory,
    error_message: Option<String>,
    current_scroll_offset: f32,
    restore_scroll_offset: Option<f32>,
    sidebar_open: bool,
    sidebar_tab: SidebarTab,
    search_open: bool,
    search_query: String,
    search_focus_requested: bool,
    font_scale: f32,
}

impl MdReaderApp {
    fn new(context: &eframe::CreationContext<'_>, initial_path: Option<PathBuf>) -> Self {
        theme::configure(&context.egui_ctx);

        let mut app = Self {
            loader: Box::new(FileDocumentLoader),
            document: None,
            markdown_cache: CommonMarkCache::default(),
            history: NavigationHistory::default(),
            error_message: None,
            current_scroll_offset: 0.0,
            restore_scroll_offset: None,
            sidebar_open: false,
            sidebar_tab: SidebarTab::default(),
            search_open: false,
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
        self.sidebar_open = false;
        self.sidebar_tab = SidebarTab::Outline;
        self.close_search();
    }

    fn set_font_scale(&mut self, scale: f32, context: &egui::Context) {
        self.font_scale = scale.clamp(MIN_FONT_SCALE, MAX_FONT_SCALE);
        theme::apply_font_scale(context, self.font_scale);
    }

    fn request_scroll_to_heading(&mut self, anchor: String) {
        *self.markdown_cache.scroll_to_id_target_mut() = Some(anchor);
    }

    fn open_explicit(&mut self, path: impl AsRef<Path>, context: &egui::Context) {
        match self.loader.load(path.as_ref()) {
            Ok(document) => {
                self.history.clear();
                self.install_document(document, 0.0, context);
            }
            Err(error) => self.error_message = Some(error.to_string()),
        }
    }

    fn follow_local_link(&mut self, path: PathBuf, context: &egui::Context) {
        match self.loader.load(&path) {
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

        match self.loader.load(&entry.path) {
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

    fn close_search(&mut self) {
        self.search_open = false;
        self.search_query.clear();
        self.search_focus_requested = false;
    }

    fn show_toolbar(&mut self, ui: &mut egui::Ui) -> ToolbarActions {
        components::toolbar(
            ui,
            ToolbarModel {
                document_path: self
                    .document
                    .as_ref()
                    .map(|document| document.path.as_path()),
                can_go_back: !self.history.is_empty(),
                sidebar_open: self.sidebar_open,
                search_open: self.search_open,
                font_scale: self.font_scale,
            },
        )
    }

    fn show_search_panel(&mut self, ui: &mut egui::Ui) -> Option<String> {
        if !self.search_open {
            return None;
        }

        let output = components::search_panel(
            ui,
            self.document.as_ref().map(|document| &document.index),
            &mut self.search_query,
            &mut self.search_focus_requested,
            self.font_scale,
        );
        if output.close_requested {
            self.close_search();
        }
        output.selected_anchor
    }

    fn show_sidebar_panel(&mut self, ui: &mut egui::Ui) -> Option<String> {
        components::sidebar_tabs(ui, &mut self.sidebar_tab);
        let document = self.document.as_ref()?;
        match self.sidebar_tab {
            SidebarTab::Outline => {
                components::outline_panel(ui, &document.index.headings, self.font_scale)
            }
            SidebarTab::Words => {
                components::word_heatmap_panel(ui, &document.word_heatmap, self.font_scale);
                None
            }
        }
    }

    fn show_document(&mut self, ui: &mut egui::Ui) -> Option<String> {
        let card_width = (ui.available_width() - 24.0).clamp(360.0, MAX_CARD_WIDTH);
        let card_height = (ui.available_height() - 12.0).max(180.0);
        let mut clicked_link = None;

        ui.with_layout(Layout::top_down(Align::Center), |ui| {
            reader_card_frame(ui).show(ui, |ui| {
                ui.set_width(card_width - 28.0);
                ui.set_min_height(card_height - 24.0);

                if let Some(document) = &self.document {
                    components::document_header(
                        ui,
                        &document.path,
                        document.index.headings.len(),
                        document.word_heatmap.total_words,
                        self.font_scale,
                    );
                }

                let narrow_drawer =
                    self.sidebar_open && ui.available_width() < SIDEBAR_DRAWER_BREAKPOINT;
                if narrow_drawer {
                    if let Some(anchor) = self.show_sidebar_panel(ui) {
                        self.request_scroll_to_heading(anchor);
                        self.sidebar_open = false;
                    }
                    return;
                }

                ui.horizontal_top(|ui| {
                    if self.sidebar_open {
                        let sidebar_height = ui.available_height().max(150.0);
                        ui.allocate_ui_with_layout(
                            Vec2::new(SIDEBAR_WIDTH, sidebar_height),
                            Layout::top_down(Align::Min),
                            |ui| {
                                if let Some(anchor) = self.show_sidebar_panel(ui) {
                                    self.request_scroll_to_heading(anchor);
                                }
                            },
                        );
                        ui.separator();
                    }

                    let reader_size = ui.available_size();
                    ui.allocate_ui_with_layout(reader_size, Layout::top_down(Align::Min), |ui| {
                        clicked_link = self.show_document_scroll(ui)
                    });
                });
            });
        });
        clicked_link
    }

    fn show_document_scroll(&mut self, ui: &mut egui::Ui) -> Option<String> {
        let document = self.document.as_ref()?;
        let reading_width = (ui.available_width() - 48.0).clamp(180.0, MAX_READING_WIDTH);
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

    fn consume_shortcuts(&mut self, context: &egui::Context) -> ShortcutActions {
        let document_open = self.document.is_some();
        let actions = ShortcutActions {
            open: context.input_mut(|input| {
                input.consume_shortcut(&KeyboardShortcut::new(Modifiers::CTRL, Key::O))
            }),
            back: context.input_mut(|input| input.consume_key(Modifiers::ALT, Key::ArrowLeft)),
            search: document_open
                && context.input_mut(|input| {
                    input.consume_shortcut(&KeyboardShortcut::new(Modifiers::CTRL, Key::F))
                }),
            zoom_in: document_open
                && context.input_mut(|input| {
                    input.consume_shortcut(&KeyboardShortcut::new(Modifiers::CTRL, Key::Plus))
                        || input
                            .consume_shortcut(&KeyboardShortcut::new(Modifiers::CTRL, Key::Equals))
                }),
            zoom_out: document_open
                && context.input_mut(|input| {
                    input.consume_shortcut(&KeyboardShortcut::new(Modifiers::CTRL, Key::Minus))
                }),
            zoom_reset: document_open
                && context.input_mut(|input| {
                    input.consume_shortcut(&KeyboardShortcut::new(Modifiers::CTRL, Key::Num0))
                }),
        };

        if self.search_open
            && context.input_mut(|input| input.consume_key(Modifiers::NONE, Key::Escape))
        {
            self.close_search();
        }
        if actions.search {
            self.search_open = true;
            self.search_focus_requested = true;
        }
        actions
    }

    fn apply_actions(
        &mut self,
        context: &egui::Context,
        shortcuts: ShortcutActions,
        toolbar: ToolbarActions,
        empty_open: bool,
    ) {
        if shortcuts.back || toolbar.back {
            self.go_back(context);
        }
        if shortcuts.open || toolbar.open || empty_open {
            self.show_open_dialog(context);
        }
        if toolbar.sidebar {
            self.sidebar_open = !self.sidebar_open;
        }
        if toolbar.search {
            self.search_open = !self.search_open;
            self.search_focus_requested = self.search_open;
            if !self.search_open {
                self.search_query.clear();
            }
        }
        if shortcuts.zoom_in || toolbar.zoom_in {
            self.set_font_scale(self.font_scale + FONT_SCALE_STEP, context);
        }
        if shortcuts.zoom_out || toolbar.zoom_out {
            self.set_font_scale(self.font_scale - FONT_SCALE_STEP, context);
        }
        if shortcuts.zoom_reset || toolbar.zoom_reset {
            self.set_font_scale(1.0, context);
        }
    }
}

impl eframe::App for MdReaderApp {
    fn ui(&mut self, root_ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let context = root_ui.ctx().clone();
        self.handle_dropped_files(&context);
        let shortcuts = self.consume_shortcuts(&context);

        let mut toolbar_actions = ToolbarActions::default();
        let mut empty_open = false;
        let mut clicked_link = None;
        let available_size = root_ui.available_size();
        theme::paint_background(root_ui);
        root_ui.allocate_ui_with_layout(available_size, Layout::top_down(Align::Min), |ui| {
            toolbar_actions = self.show_toolbar(ui);
            ui.add_space(7.0);
            components::error_banner(ui, self.error_message.as_deref());
            if let Some(anchor) = self.show_search_panel(ui) {
                self.request_scroll_to_heading(anchor);
            }
            if self.document.is_some() {
                clicked_link = self.show_document(ui);
            } else {
                empty_open = components::empty_state(ui);
            }
        });

        self.apply_actions(&context, shortcuts, toolbar_actions, empty_open);

        if let (Some(document), Some(destination)) = (&self.document, clicked_link)
            && let LinkKind::LocalMarkdown(path) = classify_link(&document.path, &destination)
        {
            self.follow_local_link(path, &context);
        }
    }
}

#[derive(Debug, Default, Clone, Copy)]
struct ShortcutActions {
    open: bool,
    back: bool,
    search: bool,
    zoom_in: bool,
    zoom_out: bool,
    zoom_reset: bool,
}
