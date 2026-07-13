use eframe::egui::{
    self, Color32, CornerRadius, FontFamily, FontId, Stroke, TextStyle, Theme, ThemePreference,
    Vec2, pos2,
};

pub(crate) fn configure(context: &egui::Context) {
    context.set_theme(ThemePreference::System);
    context.all_styles_mut(|style| {
        style.spacing.item_spacing = Vec2::new(8.0, 8.0);
        style.spacing.button_padding = Vec2::new(11.0, 6.0);
        style.spacing.menu_margin = 8.into();
        style.url_in_tooltip = true;
    });

    context.style_mut_of(Theme::Dark, |style| {
        let visuals = &mut style.visuals;
        visuals.panel_fill = Color32::from_rgb(12, 15, 24);
        visuals.window_fill = Color32::from_rgb(21, 25, 38);
        visuals.faint_bg_color = Color32::from_rgb(38, 43, 62);
        visuals.extreme_bg_color = Color32::from_rgb(10, 13, 21);
        visuals.code_bg_color = Color32::from_rgb(17, 21, 33);
        visuals.hyperlink_color = Color32::from_rgb(174, 163, 255);
        visuals.selection.bg_fill = Color32::from_rgb(91, 70, 170);
        visuals.window_stroke =
            Stroke::new(1.0, Color32::from_rgba_unmultiplied(210, 220, 255, 42));
        style_widgets(style, true);
    });

    context.style_mut_of(Theme::Light, |style| {
        let visuals = &mut style.visuals;
        visuals.panel_fill = Color32::from_rgb(235, 239, 249);
        visuals.window_fill = Color32::from_rgb(249, 250, 255);
        visuals.faint_bg_color = Color32::from_rgb(222, 228, 243);
        visuals.extreme_bg_color = Color32::from_rgb(239, 242, 250);
        visuals.code_bg_color = Color32::from_rgb(240, 243, 250);
        visuals.hyperlink_color = Color32::from_rgb(84, 69, 186);
        visuals.selection.bg_fill = Color32::from_rgb(205, 195, 255);
        visuals.window_stroke = Stroke::new(1.0, Color32::from_rgba_unmultiplied(74, 89, 153, 66));
        style_widgets(style, false);
    });

    apply_font_scale(context, 1.0);
}

pub(crate) fn apply_font_scale(context: &egui::Context, scale: f32) {
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
        widgets.inactive.weak_bg_fill = Color32::from_rgba_unmultiplied(255, 255, 255, 15);
        widgets.hovered.weak_bg_fill = Color32::from_rgba_unmultiplied(172, 157, 255, 42);
        widgets.active.weak_bg_fill = Color32::from_rgba_unmultiplied(172, 157, 255, 64);
    } else {
        widgets.inactive.weak_bg_fill = Color32::from_rgba_unmultiplied(255, 255, 255, 125);
        widgets.hovered.weak_bg_fill = Color32::from_rgba_unmultiplied(124, 111, 220, 35);
        widgets.active.weak_bg_fill = Color32::from_rgba_unmultiplied(124, 111, 220, 55);
    }
}

pub(crate) fn glass_fill(ui: &egui::Ui) -> Color32 {
    if ui.visuals().dark_mode {
        Color32::from_rgba_unmultiplied(27, 32, 49, 220)
    } else {
        Color32::from_rgba_unmultiplied(255, 255, 255, 185)
    }
}

pub(crate) fn elevated_glass_fill(ui: &egui::Ui) -> Color32 {
    if ui.visuals().dark_mode {
        Color32::from_rgba_unmultiplied(35, 41, 62, 232)
    } else {
        Color32::from_rgba_unmultiplied(255, 255, 255, 220)
    }
}

pub(crate) fn glass_stroke(ui: &egui::Ui) -> Stroke {
    if ui.visuals().dark_mode {
        Stroke::new(1.0, Color32::from_rgba_unmultiplied(215, 224, 255, 55))
    } else {
        Stroke::new(1.0, Color32::from_rgba_unmultiplied(90, 105, 165, 68))
    }
}

pub(crate) fn muted_fill(ui: &egui::Ui) -> Color32 {
    if ui.visuals().dark_mode {
        Color32::from_rgba_unmultiplied(255, 255, 255, 10)
    } else {
        Color32::from_rgba_unmultiplied(88, 100, 160, 10)
    }
}

pub(crate) fn word_heat_fill(ui: &egui::Ui, heat: f32) -> Color32 {
    let heat = heat.clamp(0.0, 1.0);
    let (cold, hot) = if ui.visuals().dark_mode {
        (
            Color32::from_rgb(42, 47, 65),
            Color32::from_rgb(127, 96, 235),
        )
    } else {
        (
            Color32::from_rgb(238, 240, 248),
            Color32::from_rgb(185, 162, 255),
        )
    };
    Color32::from_rgb(
        lerp_channel(cold.r(), hot.r(), heat),
        lerp_channel(cold.g(), hot.g(), heat),
        lerp_channel(cold.b(), hot.b(), heat),
    )
}

fn lerp_channel(start: u8, end: u8, amount: f32) -> u8 {
    (start as f32 + (end as f32 - start as f32) * amount).round() as u8
}

pub(crate) fn paint_background(ui: &mut egui::Ui) {
    let rect = ui.available_rect_before_wrap();
    let painter = ui.painter();
    let dark = ui.visuals().dark_mode;
    painter.rect_filled(rect, 0.0, ui.visuals().panel_fill);

    let first = if dark {
        Color32::from_rgba_unmultiplied(126, 102, 255, 37)
    } else {
        Color32::from_rgba_unmultiplied(145, 126, 255, 49)
    };
    let second = if dark {
        Color32::from_rgba_unmultiplied(49, 150, 255, 28)
    } else {
        Color32::from_rgba_unmultiplied(76, 166, 255, 39)
    };

    painter.circle_filled(
        pos2(rect.left() + rect.width() * 0.08, rect.top() + 60.0),
        250.0,
        first,
    );
    painter.circle_filled(
        pos2(rect.right() - 30.0, rect.bottom() - 10.0),
        330.0,
        second,
    );
}
