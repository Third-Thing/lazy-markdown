use std::collections::BTreeMap;

use floem::prelude::SignalUpdate;
use keyboard_types::{Code, NamedKey};

use crate::{
    documents::{create_new_tab, request_open, request_save, request_save_as},
    preferences::editor_font::{
        decrease_editor_font_size, increase_editor_font_size, reset_editor_font_size,
    },
    state::AppState,
};

#[allow(dead_code)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ShortcutModifier {
    Control,
    Alt,
    Shift,
    Meta,
}

#[allow(dead_code)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ShortcutKey {
    Character(&'static str),
    Named(NamedKey),
    Code(Code),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Shortcut {
    pub key: ShortcutKey,
    pub modifiers: &'static [ShortcutModifier],
}

#[allow(dead_code)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum CommandPlacement {
    Menu,
    ContextMenu,
    Palette,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CommandMetadata {
    pub id: &'static str,
    pub title: &'static str,
    pub default_shortcuts: &'static [Shortcut],
    pub placements: &'static [CommandPlacement],
}

#[derive(Clone, Default)]
pub struct CommandRegistry {
    commands: BTreeMap<&'static str, CommandMetadata>,
}

impl CommandRegistry {
    pub fn register(&mut self, command: CommandMetadata) -> Result<(), String> {
        if self.commands.contains_key(command.id) {
            return Err(format!("Duplicate command id `{}`", command.id));
        }

        self.commands.insert(command.id, command);
        Ok(())
    }

    pub fn get(&self, command_id: &str) -> Option<&CommandMetadata> {
        self.commands.get(command_id)
    }

    pub fn iter(&self) -> impl Iterator<Item = &CommandMetadata> {
        self.commands.values()
    }
}

#[derive(Clone, Default)]
pub struct ModuleRegistry {
    commands: CommandRegistry,
}

impl ModuleRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn commands(&self) -> &CommandRegistry {
        &self.commands
    }

    pub fn commands_mut(&mut self) -> &mut CommandRegistry {
        &mut self.commands
    }
}

pub mod command_ids {
    pub const FILE_NEW: &str = "file.new";
    pub const FILE_OPEN: &str = "file.open";
    pub const FILE_SAVE: &str = "file.save";
    pub const FILE_SAVE_AS: &str = "file.save_as";
    pub const VIEW_ZOOM_IN: &str = "view.zoom_in";
    pub const VIEW_ZOOM_OUT: &str = "view.zoom_out";
    pub const VIEW_ZOOM_RESET: &str = "view.zoom_reset";
}

const PRIMARY: &[ShortcutModifier] = &[ShortcutModifier::Control];
const PRIMARY_SHIFT: &[ShortcutModifier] = &[ShortcutModifier::Control, ShortcutModifier::Shift];
const FILE_COMMAND_PLACEMENTS: &[CommandPlacement] =
    &[CommandPlacement::Menu, CommandPlacement::Palette];
const PALETTE_ONLY: &[CommandPlacement] = &[CommandPlacement::Palette];

const FILE_NEW_SHORTCUTS: &[Shortcut] = &[Shortcut {
    key: ShortcutKey::Character("n"),
    modifiers: PRIMARY,
}];
const FILE_OPEN_SHORTCUTS: &[Shortcut] = &[Shortcut {
    key: ShortcutKey::Character("o"),
    modifiers: PRIMARY,
}];
const FILE_SAVE_SHORTCUTS: &[Shortcut] = &[Shortcut {
    key: ShortcutKey::Character("s"),
    modifiers: PRIMARY,
}];
const FILE_SAVE_AS_SHORTCUTS: &[Shortcut] = &[Shortcut {
    key: ShortcutKey::Character("s"),
    modifiers: PRIMARY_SHIFT,
}];
const VIEW_ZOOM_IN_SHORTCUTS: &[Shortcut] = &[
    Shortcut {
        key: ShortcutKey::Code(Code::Equal),
        modifiers: PRIMARY,
    },
    Shortcut {
        key: ShortcutKey::Code(Code::Equal),
        modifiers: PRIMARY_SHIFT,
    },
    Shortcut {
        key: ShortcutKey::Code(Code::NumpadAdd),
        modifiers: PRIMARY,
    },
];
const VIEW_ZOOM_OUT_SHORTCUTS: &[Shortcut] = &[
    Shortcut {
        key: ShortcutKey::Code(Code::Minus),
        modifiers: PRIMARY,
    },
    Shortcut {
        key: ShortcutKey::Code(Code::NumpadSubtract),
        modifiers: PRIMARY,
    },
];
const VIEW_ZOOM_RESET_SHORTCUTS: &[Shortcut] = &[
    Shortcut {
        key: ShortcutKey::Code(Code::Digit0),
        modifiers: PRIMARY,
    },
    Shortcut {
        key: ShortcutKey::Code(Code::Numpad0),
        modifiers: PRIMARY,
    },
];

const FILE_NEW_COMMAND: CommandMetadata = CommandMetadata {
    id: command_ids::FILE_NEW,
    title: "New",
    default_shortcuts: FILE_NEW_SHORTCUTS,
    placements: FILE_COMMAND_PLACEMENTS,
};

const FILE_OPEN_COMMAND: CommandMetadata = CommandMetadata {
    id: command_ids::FILE_OPEN,
    title: "Open",
    default_shortcuts: FILE_OPEN_SHORTCUTS,
    placements: FILE_COMMAND_PLACEMENTS,
};

const FILE_SAVE_COMMAND: CommandMetadata = CommandMetadata {
    id: command_ids::FILE_SAVE,
    title: "Save",
    default_shortcuts: FILE_SAVE_SHORTCUTS,
    placements: FILE_COMMAND_PLACEMENTS,
};

const FILE_SAVE_AS_COMMAND: CommandMetadata = CommandMetadata {
    id: command_ids::FILE_SAVE_AS,
    title: "Save As",
    default_shortcuts: FILE_SAVE_AS_SHORTCUTS,
    placements: FILE_COMMAND_PLACEMENTS,
};

const VIEW_ZOOM_IN_COMMAND: CommandMetadata = CommandMetadata {
    id: command_ids::VIEW_ZOOM_IN,
    title: "Zoom In",
    default_shortcuts: VIEW_ZOOM_IN_SHORTCUTS,
    placements: PALETTE_ONLY,
};

const VIEW_ZOOM_OUT_COMMAND: CommandMetadata = CommandMetadata {
    id: command_ids::VIEW_ZOOM_OUT,
    title: "Zoom Out",
    default_shortcuts: VIEW_ZOOM_OUT_SHORTCUTS,
    placements: PALETTE_ONLY,
};

const VIEW_ZOOM_RESET_COMMAND: CommandMetadata = CommandMetadata {
    id: command_ids::VIEW_ZOOM_RESET,
    title: "Reset Font Size",
    default_shortcuts: VIEW_ZOOM_RESET_SHORTCUTS,
    placements: PALETTE_ONLY,
};

const BUILTIN_COMMANDS: &[CommandMetadata] = &[
    FILE_NEW_COMMAND,
    FILE_OPEN_COMMAND,
    FILE_SAVE_COMMAND,
    FILE_SAVE_AS_COMMAND,
    VIEW_ZOOM_IN_COMMAND,
    VIEW_ZOOM_OUT_COMMAND,
    VIEW_ZOOM_RESET_COMMAND,
];

pub fn register_builtin_commands(registry: &mut CommandRegistry) -> Result<(), String> {
    for command in BUILTIN_COMMANDS {
        registry.register(*command)?;
    }

    Ok(())
}

pub(crate) fn invoke_command(command_id: &str, state: &AppState) {
    match command_id {
        command_ids::FILE_NEW => create_new_tab(state),
        command_ids::FILE_OPEN => request_open(state.clone()),
        command_ids::FILE_SAVE => {
            let Some(document) = state.active_document_untracked() else {
                state
                    .status_message
                    .set(Some("No active document".to_string()));
                return;
            };
            request_save(state, &document);
        }
        command_ids::FILE_SAVE_AS => {
            let Some(document) = state.active_document_untracked() else {
                state
                    .status_message
                    .set(Some("No active document".to_string()));
                return;
            };
            request_save_as(state.clone(), document);
        }
        command_ids::VIEW_ZOOM_IN => increase_editor_font_size(state),
        command_ids::VIEW_ZOOM_OUT => decrease_editor_font_size(state),
        command_ids::VIEW_ZOOM_RESET => reset_editor_font_size(state),
        _ => state
            .status_message
            .set(Some(format!("Unknown command `{command_id}`"))),
    }
}

pub(crate) fn command_title(
    command_registry: &CommandRegistry,
    command_id: &'static str,
) -> &'static str {
    command_registry
        .get(command_id)
        .map(|command| command.title)
        .unwrap_or(command_id)
}
