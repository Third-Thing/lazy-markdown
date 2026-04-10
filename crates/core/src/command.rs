use std::collections::BTreeMap;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ShortcutModifier {
    Control,
    Alt,
    Shift,
    Meta,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ShortcutKey {
    Character(&'static str),
    Named(&'static str),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Shortcut {
    pub key: ShortcutKey,
    pub modifiers: &'static [ShortcutModifier],
}

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
    pub default_shortcut: Option<Shortcut>,
    pub placements: &'static [CommandPlacement],
}

#[derive(Clone, Default)]
pub struct CommandRegistry {
    commands: BTreeMap<&'static str, CommandMetadata>,
}

impl CommandRegistry {
    pub fn new() -> Self {
        Self::default()
    }

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

    pub fn commands_for_placement(&self, placement: CommandPlacement) -> Vec<CommandMetadata> {
        self.commands
            .values()
            .copied()
            .filter(|command| command.placements.contains(&placement))
            .collect()
    }
}

pub mod ids {
    pub const FILE_NEW: &str = "file.new";
    pub const FILE_OPEN: &str = "file.open";
    pub const FILE_SAVE: &str = "file.save";
    pub const FILE_SAVE_AS: &str = "file.save_as";
}

const PRIMARY: &[ShortcutModifier] = &[ShortcutModifier::Control];
const PRIMARY_SHIFT: &[ShortcutModifier] = &[ShortcutModifier::Control, ShortcutModifier::Shift];
const FILE_COMMAND_PLACEMENTS: &[CommandPlacement] =
    &[CommandPlacement::Menu, CommandPlacement::Palette];

const FILE_NEW_COMMAND: CommandMetadata = CommandMetadata {
    id: ids::FILE_NEW,
    title: "New",
    default_shortcut: Some(Shortcut {
        key: ShortcutKey::Character("n"),
        modifiers: PRIMARY,
    }),
    placements: FILE_COMMAND_PLACEMENTS,
};

const FILE_OPEN_COMMAND: CommandMetadata = CommandMetadata {
    id: ids::FILE_OPEN,
    title: "Open",
    default_shortcut: Some(Shortcut {
        key: ShortcutKey::Character("o"),
        modifiers: PRIMARY,
    }),
    placements: FILE_COMMAND_PLACEMENTS,
};

const FILE_SAVE_COMMAND: CommandMetadata = CommandMetadata {
    id: ids::FILE_SAVE,
    title: "Save",
    default_shortcut: Some(Shortcut {
        key: ShortcutKey::Character("s"),
        modifiers: PRIMARY,
    }),
    placements: FILE_COMMAND_PLACEMENTS,
};

const FILE_SAVE_AS_COMMAND: CommandMetadata = CommandMetadata {
    id: ids::FILE_SAVE_AS,
    title: "Save As",
    default_shortcut: Some(Shortcut {
        key: ShortcutKey::Character("s"),
        modifiers: PRIMARY_SHIFT,
    }),
    placements: FILE_COMMAND_PLACEMENTS,
};

const BUILTIN_COMMANDS: &[CommandMetadata] = &[
    FILE_NEW_COMMAND,
    FILE_OPEN_COMMAND,
    FILE_SAVE_COMMAND,
    FILE_SAVE_AS_COMMAND,
];

pub fn register_builtin_commands(registry: &mut CommandRegistry) -> Result<(), String> {
    for command in BUILTIN_COMMANDS {
        registry.register(*command)?;
    }

    Ok(())
}
