use floem::{
    peniko::Color,
    prelude::*,
    views::{
        editor::{command::CommandExecuted, keypress::KeypressMap, view::editor_container_view},
        tab,
    },
};

use crate::{
    commands::{CommandRegistry, invoke_command},
    shortcuts::resolve_shortcut_command,
    state::{AppState, DocumentState},
    theme::editor_theme_style,
};

fn document_editor_view(
    document: DocumentState,
    command_registry: CommandRegistry,
    state: AppState,
) -> impl IntoView {
    let editor_sig = RwSignal::new(document.editor.clone());
    let keymap = KeypressMap::default();

    editor_container_view(
        editor_sig,
        |_| true,
        move |keypress| {
            if let Some(command_id) = resolve_shortcut_command(&command_registry, &keypress) {
                invoke_command(command_id, &state);
                return CommandExecuted::Yes;
            }

            keymap.handle_keypress(editor_sig, &keypress)
        },
    )
    .style(|s| {
        s.apply(editor_theme_style())
            .width_full()
            .min_size(0, 0)
            .flex_grow(1.0)
            .border(1.0)
            .border_color(Color::from_rgb8(220, 223, 227))
    })
}

pub(crate) fn tab_content_view(
    state: AppState,
    command_registry: CommandRegistry,
) -> impl IntoView {
    let active_state = state.clone();
    let documents_state = state.clone();
    let content_state = state;

    tab(
        move || active_state.active_index(),
        move || documents_state.documents(),
        DocumentState::id,
        move |document| {
            document_editor_view(document, command_registry.clone(), content_state.clone())
        },
    )
    .style(|s| s.width_full().min_size(0, 0).flex_grow(1.0))
}
