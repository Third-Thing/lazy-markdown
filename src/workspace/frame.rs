use floem::{
    prelude::*,
    views::{Label, Stack},
};

use crate::preferences::editor_font::editor_font_label;

use super::{AppState, tab_content_view, tab_strip_view};

fn top_bar_view(menu_bar: impl IntoView + 'static, state: AppState) -> impl IntoView {
    Stack::horizontal((menu_bar,)).style(move |s| {
        let theme = state.app_theme();
        s.width_full()
            .items_center()
            .padding_horiz(10.0)
            .padding_vert(6.0)
            .background(theme.chrome_bg)
    })
}

fn status_strip_view(state: AppState) -> impl IntoView {
    Stack::horizontal((
        {
            let state = state.clone();
            Label::derived(move || {
                format!(
                    "{} {}px",
                    editor_font_label(&state.editor_font()),
                    state.editor_font_size()
                )
            })
        }
        .style({
            let state = state.clone();
            move |s| {
                let theme = state.app_theme();
                s.font_size(12.0).font_bold().color(theme.text)
            }
        }),
        {
            let state = state.clone();
            Label::derived(move || state.status_message.get().unwrap_or_default())
        }
        .style({
            let state = state.clone();
            move |s| {
                let theme = state.app_theme();
                s.font_size(12.0)
                    .color(theme.text_muted)
                    .text_ellipsis()
                    .min_width(0.0)
                    .flex_grow(1.0)
                    .justify_end()
            }
        }),
    ))
    .style(move |s| {
        let theme = state.app_theme();
        s.width_full()
            .items_center()
            .justify_between()
            .col_gap(12.0)
            .padding_horiz(12.0)
            .padding_vert(8.0)
            .background(theme.status_bg)
            .border_top(1.0)
            .border_color(theme.border)
    })
}

pub(crate) fn workspace_frame_view(
    menu_bar: impl IntoView + 'static,
    state: AppState,
) -> impl IntoView {
    Stack::vertical((
        top_bar_view(menu_bar, state.clone()),
        tab_strip_view(state.clone()),
        tab_content_view(state.clone()),
        status_strip_view(state.clone()),
    ))
    .style(move |s| {
        let theme = state.app_theme();
        s.size_full()
            .padding(10.0)
            .row_gap(0.0)
            .background(theme.panel_bg)
    })
}
