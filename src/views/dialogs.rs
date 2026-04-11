use floem::{
    peniko::{Color, color::palette},
    prelude::*,
    views::{Button, Empty, Label, Overlay, Stack},
};

use crate::{
    commands::{command_ids, invoke_command},
    documents::finish_pending_action,
    state::{AppState, PendingAction},
};

pub(crate) fn confirm_overlay(state: AppState) -> Overlay {
    let backdrop = Empty::new()
        .style(|s| {
            s.absolute()
                .inset(0.0)
                .background(palette::css::BLACK)
                .opacity(0.25)
                .z_index(1)
        })
        .on_event_cont(listener::Click, move |_, _| {
            state.pending_action.set(None);
            state.show_confirm.set(false);
        });

    let save_button = {
        let state = state.clone();
        Button::new("Save").action(move || {
            invoke_command(command_ids::FILE_SAVE, &state);
        })
    };

    let dont_save_button = {
        let state = state.clone();
        Button::new("Don't Save").action(move || {
            if let Some(action) = state.pending_action.get_untracked() {
                finish_pending_action(action, &state);
            } else {
                state.show_confirm.set(false);
            }
        })
    };

    let cancel_button = {
        let state = state.clone();
        Button::new("Cancel").action(move || {
            state.pending_action.set(None);
            state.show_confirm.set(false);
        })
    };

    let buttons =
        Stack::horizontal((save_button, dont_save_button, cancel_button)).style(|s| s.col_gap(8.0));

    let title = {
        let state = state.clone();
        Label::derived(move || match state.pending_action.get() {
            Some(PendingAction::CloseDocument { .. }) => "Unsaved changes".to_string(),
            Some(PendingAction::CloseWindow { .. }) => "Unsaved changes".to_string(),
            None => "Unsaved changes".to_string(),
        })
    }
    .style(|s| s.font_size(18.0).font_bold());

    let message = {
        let state = state.clone();
        Label::derived(move || match state.pending_action.get() {
            Some(PendingAction::CloseDocument { .. }) => {
                "Save your changes before closing this tab?".to_string()
            }
            Some(PendingAction::CloseWindow { .. }) => {
                "Save your changes before closing this window?".to_string()
            }
            None => "Save your changes before continuing?".to_string(),
        })
    }
    .style(|s| s.color(Color::from_rgb8(82, 89, 102)));

    let target_path = {
        let state = state.clone();
        Label::derived(move || {
            let Some(document) = state.active_document() else {
                return String::new();
            };
            document
                .file_path
                .try_get()
                .flatten()
                .map(|path| path.display().to_string())
                .unwrap_or_else(|| "Untitled".to_string())
        })
    }
    .style({
        let state = state.clone();
        move |s| {
            s.width_full()
                .max_width_full()
                .padding(10.0)
                .font_size(12.0)
                .text_wrap()
                .color(Color::from_rgb8(59, 70, 91))
                .background(Color::from_rgb8(244, 246, 250))
                .border(1.0)
                .border_color(Color::from_rgb8(228, 232, 237))
                .border_radius(8.0)
                .apply_if(state.pending_action.get().is_none(), |s| s.hide())
        }
    });

    let dialog = Stack::vertical((title, message, target_path, buttons)).style(|s| {
        s.absolute()
            .inset_left(40.0)
            .inset_top(40.0)
            .width(420.0)
            .padding(16.0)
            .row_gap(12.0)
            .border(1.0)
            .border_radius(12.0)
            .border_color(Color::from_rgb8(224, 228, 233))
            .background(palette::css::WHITE)
            .z_index(10)
    });

    Overlay::new({
        let state = state.clone();
        Stack::new((backdrop, dialog)).style(move |s| {
            s.fixed()
                .inset(0.0)
                .width_full()
                .height_full()
                .apply_if(!state.show_confirm.get(), |s| s.hide())
        })
    })
}
