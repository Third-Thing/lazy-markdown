use floem::{
    action::{current_theme, set_theme},
    peniko::Color,
    prelude::SignalUpdate,
    style::{CursorColor, Style},
    views::editor::{
        CurrentLineColor, IndentGuideColor, PreeditUnderlineColor, SelectionColor,
        VisibleWhitespaceColor,
        gutter::{DimColor, GutterClass},
        view::EditorViewClass,
    },
    window::Theme as WindowTheme,
};

use crate::state::AppState;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ThemePreference {
    FollowOs,
    Light,
    Dark,
}

#[derive(Clone, Copy)]
pub(crate) struct AppTheme {
    pub(crate) chrome_bg: Color,
    pub(crate) panel_bg: Color,
    pub(crate) status_bg: Color,
    pub(crate) border: Color,
    pub(crate) text: Color,
    pub(crate) text_muted: Color,
    pub(crate) menu_popup_bg: Color,
    pub(crate) menu_popup_selected_bg: Color,
    pub(crate) menu_button_bg: Color,
    pub(crate) menu_button_bg_active: Color,
    pub(crate) menu_button_bg_hover: Color,
    pub(crate) menu_button_bg_pressed: Color,
    pub(crate) menu_button_border: Color,
    pub(crate) menu_button_border_active: Color,
    pub(crate) tab_bg: Color,
    pub(crate) tab_bg_active: Color,
    pub(crate) tab_border: Color,
    pub(crate) tab_border_active: Color,
    pub(crate) tab_close_hover_bg: Color,
    pub(crate) dialog_bg: Color,
    pub(crate) dialog_path_bg: Color,
    pub(crate) dialog_path_border: Color,
    pub(crate) dialog_scrim: Color,
    pub(crate) editor_fg: Color,
    pub(crate) editor_bg: Color,
    pub(crate) editor_selection: Color,
    pub(crate) editor_dim: Color,
    pub(crate) editor_cursor: Color,
    pub(crate) editor_current_line: Color,
}

impl AppTheme {
    pub(crate) fn from_window_theme(theme: WindowTheme) -> Self {
        match theme {
            WindowTheme::Light => Self::light(),
            WindowTheme::Dark => Self::dark(),
        }
    }

    fn light() -> Self {
        Self {
            chrome_bg: Color::from_rgb8(232, 235, 239),
            panel_bg: Color::from_rgb8(243, 245, 248),
            status_bg: Color::from_rgb8(237, 240, 244),
            border: Color::from_rgb8(211, 216, 223),
            text: Color::from_rgb8(52, 59, 69),
            text_muted: Color::from_rgb8(94, 103, 116),
            menu_popup_bg: Color::from_rgb8(244, 246, 249),
            menu_popup_selected_bg: Color::from_rgb8(220, 226, 234),
            menu_button_bg: Color::from_rgb8(249, 250, 252),
            menu_button_bg_active: Color::from_rgb8(228, 233, 239),
            menu_button_bg_hover: Color::from_rgb8(235, 239, 244),
            menu_button_bg_pressed: Color::from_rgb8(223, 228, 235),
            menu_button_border: Color::from_rgb8(193, 199, 207),
            menu_button_border_active: Color::from_rgb8(162, 171, 182),
            tab_bg: Color::from_rgb8(233, 237, 242),
            tab_bg_active: Color::from_rgb8(251, 252, 253),
            tab_border: Color::from_rgb8(212, 218, 225),
            tab_border_active: Color::from_rgb8(192, 200, 210),
            tab_close_hover_bg: Color::from_rgb8(223, 228, 234),
            dialog_bg: Color::from_rgb8(255, 255, 255),
            dialog_path_bg: Color::from_rgb8(244, 246, 250),
            dialog_path_border: Color::from_rgb8(228, 232, 237),
            dialog_scrim: Color::from_rgba8(0, 0, 0, 64),
            editor_fg: Color::from_rgb8(0x38, 0x3A, 0x42),
            editor_bg: Color::from_rgb8(0xFA, 0xFA, 0xFA),
            editor_selection: Color::from_rgb8(0xE5, 0xE5, 0xE6),
            editor_dim: Color::from_rgb8(0xA0, 0xA1, 0xA7),
            editor_cursor: Color::from_rgb8(0x52, 0x6F, 0xFF),
            editor_current_line: Color::from_rgb8(0xF2, 0xF2, 0xF2),
        }
    }

    fn dark() -> Self {
        Self {
            chrome_bg: Color::from_rgb8(0x22, 0x26, 0x2D),
            panel_bg: Color::from_rgb8(0x1D, 0x21, 0x27),
            status_bg: Color::from_rgb8(0x24, 0x28, 0x2F),
            border: Color::from_rgb8(0x3A, 0x40, 0x4A),
            text: Color::from_rgb8(0xD6, 0xDB, 0xE3),
            text_muted: Color::from_rgb8(0x9D, 0xA6, 0xB2),
            menu_popup_bg: Color::from_rgb8(0x28, 0x2D, 0x35),
            menu_popup_selected_bg: Color::from_rgb8(0x38, 0x3F, 0x4A),
            menu_button_bg: Color::from_rgb8(0x2C, 0x31, 0x3A),
            menu_button_bg_active: Color::from_rgb8(0x36, 0x3D, 0x47),
            menu_button_bg_hover: Color::from_rgb8(0x34, 0x3A, 0x44),
            menu_button_bg_pressed: Color::from_rgb8(0x3B, 0x42, 0x4E),
            menu_button_border: Color::from_rgb8(0x4B, 0x54, 0x60),
            menu_button_border_active: Color::from_rgb8(0x68, 0x73, 0x82),
            tab_bg: Color::from_rgb8(0x25, 0x29, 0x30),
            tab_bg_active: Color::from_rgb8(0x30, 0x35, 0x3E),
            tab_border: Color::from_rgb8(0x3C, 0x44, 0x50),
            tab_border_active: Color::from_rgb8(0x57, 0x62, 0x71),
            tab_close_hover_bg: Color::from_rgb8(0x43, 0x49, 0x54),
            dialog_bg: Color::from_rgb8(0x26, 0x2B, 0x33),
            dialog_path_bg: Color::from_rgb8(0x1F, 0x24, 0x2B),
            dialog_path_border: Color::from_rgb8(0x41, 0x48, 0x54),
            dialog_scrim: Color::from_rgba8(0, 0, 0, 96),
            editor_fg: Color::from_rgb8(0xAB, 0xB2, 0xBF),
            editor_bg: Color::from_rgb8(0x28, 0x2C, 0x34),
            editor_selection: Color::from_rgb8(0x3E, 0x44, 0x51),
            editor_dim: Color::from_rgb8(0x5C, 0x63, 0x70),
            editor_cursor: Color::from_rgb8(0x52, 0x8B, 0xFF),
            editor_current_line: Color::from_rgb8(0x2C, 0x31, 0x3C),
        }
    }
}

pub(crate) fn apply_theme_preference(state: &AppState, preference: ThemePreference) {
    state.theme_preference.set(preference);

    match preference {
        ThemePreference::Light => {
            state.window_theme.set(WindowTheme::Light);
            set_theme(Some(WindowTheme::Light));
        }
        ThemePreference::Dark => {
            state.window_theme.set(WindowTheme::Dark);
            set_theme(Some(WindowTheme::Dark));
        }
        ThemePreference::FollowOs => {
            set_theme(None);
            if let Some(theme) = current_theme() {
                state.window_theme.set(theme);
            }
        }
    }
}

pub(crate) fn editor_theme_style(theme: AppTheme) -> Style {
    Style::new()
        .color(theme.editor_fg)
        .background(theme.editor_bg)
        .class(GutterClass, move |s| {
            s.background(theme.editor_bg)
                .set(DimColor, Some(theme.editor_dim))
                .set(CurrentLineColor, theme.editor_current_line)
        })
        .class(EditorViewClass, move |s| {
            s.set(CursorColor, theme.editor_cursor)
                .set(SelectionColor, theme.editor_selection)
                .set(CurrentLineColor, theme.editor_current_line)
                .set(VisibleWhitespaceColor, theme.editor_selection)
                .set(PreeditUnderlineColor, theme.editor_fg)
                .set(IndentGuideColor, theme.editor_selection)
        })
}
