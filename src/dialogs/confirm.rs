use floem::{
    prelude::*,
    views::{Button, Empty, Label, Overlay, Stack},
};

use crate::{
    commands::{command_ids, invoke_command},
    workspace::{AppState, complete_dialog_action},
};

pub(crate) fn dialog_overlay(state: AppState) -> Overlay {
    let backdrop_state = state.clone();
    let backdrop = Empty::new()
        .style({
            let state = state.clone();
            move |s| {
                let theme = state.app_theme();
                s.absolute()
                    .inset(0.0)
                    .background(theme.dialog_scrim)
                    .z_index(1)
            }
        })
        .on_event_cont(listener::Click, move |_, _| {
            backdrop_state.clear_active_dialog();
        });

    let buttons = {
        let state = state.clone();
        Stack::horizontal((
            {
                let action_state = state.clone();
                let style_state = state.clone();
                Button::new("Save")
                    .action(move || {
                        invoke_command(command_ids::FILE_SAVE, &action_state);
                    })
                    .style(move |s| {
                        s.apply_if(
                            !style_state
                                .active_dialog()
                                .is_some_and(|dialog| dialog.needs_save_decision()),
                            |s| s.hide(),
                        )
                    })
            },
            {
                let action_state = state.clone();
                let style_state = state.clone();
                Button::new("Don't Save")
                    .action(move || {
                        if let Some(dialog) = action_state.active_dialog_untracked() {
                            complete_dialog_action(dialog, &action_state);
                        } else {
                            action_state.clear_active_dialog();
                        }
                    })
                    .style(move |s| {
                        s.apply_if(
                            !style_state
                                .active_dialog()
                                .is_some_and(|dialog| dialog.needs_save_decision()),
                            |s| s.hide(),
                        )
                    })
            },
            {
                let action_state = state.clone();
                let style_state = state.clone();
                Button::new("Cancel")
                    .action(move || {
                        action_state.clear_active_dialog();
                    })
                    .style(move |s| {
                        s.apply_if(
                            style_state
                                .active_dialog()
                                .is_some_and(|dialog| !dialog.needs_save_decision()),
                            |s| s.hide(),
                        )
                    })
            },
            {
                let action_state = state.clone();
                let style_state = state.clone();
                Button::new("OK")
                    .action(move || {
                        action_state.clear_active_dialog();
                    })
                    .style(move |s| {
                        s.apply_if(
                            !style_state
                                .active_dialog()
                                .is_some_and(|dialog| !dialog.needs_save_decision()),
                            |s| s.hide(),
                        )
                    })
            },
        ))
        .style(|s| s.col_gap(8.0))
    };

    let title = {
        let state = state.clone();
        Label::derived(move || {
            state
                .active_dialog()
                .map(|dialog| dialog.title_text())
                .unwrap_or_else(|| "Unsaved changes".to_string())
        })
    }
    .style(|s| s.font_size(18.0).font_bold());

    let message = {
        let state = state.clone();
        Label::derived(move || {
            state
                .active_dialog()
                .map(|dialog| dialog.message_text())
                .unwrap_or_else(|| "Save your changes before continuing?".to_string())
        })
    }
    .style(|s| s.width_full().max_width_full().min_width(0.0).text_wrap());
    let message = message.style({
        let state = state.clone();
        move |s| {
            let theme = state.app_theme();
            s.color(theme.text_muted)
        }
    });

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
            let theme = state.app_theme();
            s.width_full()
                .max_width_full()
                .padding(10.0)
                .font_size(12.0)
                .text_wrap()
                .color(theme.text)
                .background(theme.dialog_path_bg)
                .border(1.0)
                .border_color(theme.dialog_path_border)
                .border_radius(8.0)
                .apply_if(
                    !state
                        .active_dialog()
                        .is_some_and(|dialog| dialog.shows_document_path()),
                    |s| s.hide(),
                )
        }
    });

    let dialog = Stack::vertical((title, message, target_path, buttons)).style({
        let state = state.clone();
        move |s| {
            let theme = state.app_theme();
            s.absolute()
                .inset_left(40.0)
                .inset_top(40.0)
                .width(420.0)
                .padding(16.0)
                .row_gap(12.0)
                .border(1.0)
                .border_radius(12.0)
                .border_color(theme.border)
                .background(theme.dialog_bg)
                .color(theme.text)
                .z_index(10)
        }
    });

    Overlay::new({
        let state = state.clone();
        Stack::new((backdrop, dialog)).style(move |s| {
            s.fixed()
                .inset(0.0)
                .width_full()
                .height_full()
                .apply_if(state.active_dialog().is_none(), |s| s.hide())
        })
    })
}
