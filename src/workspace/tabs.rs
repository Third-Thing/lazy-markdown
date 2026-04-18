use floem::{
    prelude::*,
    views::{Button, Label, Stack, dyn_stack},
};

use super::{
    documents::{activate_document, document_title_text, request_close_document},
    state::{AppState, DocumentState},
};

fn tab_header_view(document: DocumentState, state: AppState) -> impl IntoView {
    let document_id = document.id();
    let activate_state = state.clone();
    let close_state = state.clone();

    Stack::horizontal((
        Label::derived(move || document_title_text(&document)).style({
            let state = state.clone();
            move |s| {
                let theme = state.app_theme();
                s.font_size(12.0).color(theme.text)
            }
        }),
        Button::new("x")
            .action(move || request_close_document(&close_state, document_id))
            .style({
                let state = state.clone();
                move |s| {
                    let theme = state.app_theme();
                    s.padding_horiz(6.0)
                        .padding_vert(2.0)
                        .font_size(11.0)
                        .border(0.0)
                        .border_radius(4.0)
                        .background(theme.tab_bg.with_alpha(0.0))
                        .hover(|s| s.background(theme.tab_close_hover_bg))
                }
            }),
    ))
    .style(move |s| {
        let theme = state.app_theme();
        let is_active = state.documents.get().active_document_id() == Some(document_id);
        s.items_center()
            .col_gap(6.0)
            .padding_horiz(10.0)
            .padding_vert(6.0)
            .border(1.0)
            .border_color(if is_active {
                theme.tab_border_active
            } else {
                theme.tab_border
            })
            .border_radius(6.0)
            .background(if is_active {
                theme.tab_bg_active
            } else {
                theme.tab_bg
            })
    })
    .on_event_stop(listener::Click, move |_cx, _| {
        activate_document(&activate_state, document_id);
    })
}

pub(crate) fn tab_strip_view(state: AppState) -> impl IntoView {
    let tab_list_state = state.clone();
    let tab_item_state = state.clone();

    dyn_stack(
        move || tab_list_state.documents(),
        DocumentState::id,
        move |document| tab_header_view(document, tab_item_state.clone()),
    )
    .style({
        let state = state.clone();
        move |s| {
            let theme = state.app_theme();
            s.flex_row()
                .width_full()
                .padding_horiz(10.0)
                .padding_vert(8.0)
                .col_gap(6.0)
                .background(theme.status_bg)
        }
    })
    .style(|s| s.width_full().height(46.0))
}
