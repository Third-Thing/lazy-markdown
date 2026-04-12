use floem::prelude::{Key, KeyboardEvent, Modifiers, NamedKey};

use crate::commands::{CommandRegistry, Shortcut, ShortcutKey, ShortcutModifier};

fn supported_modifiers(modifiers: Modifiers) -> Modifiers {
    modifiers & (Modifiers::CONTROL | Modifiers::ALT | Modifiers::SHIFT | Modifiers::META)
}

fn matches_shortcut(shortcut: Shortcut, keypress: &KeyboardEvent) -> bool {
    supported_modifiers(keypress.modifiers) == shortcut_modifiers(shortcut.modifiers)
        && shortcut_key_matches(shortcut.key, &keypress.key)
}

fn shortcut_modifiers(modifiers: &[ShortcutModifier]) -> Modifiers {
    let mut resolved = Modifiers::empty();

    for modifier in modifiers {
        match modifier {
            ShortcutModifier::Control => resolved |= Modifiers::CONTROL,
            ShortcutModifier::Alt => resolved |= Modifiers::ALT,
            ShortcutModifier::Shift => resolved |= Modifiers::SHIFT,
            ShortcutModifier::Meta => resolved |= Modifiers::META,
        }
    }

    resolved
}

fn shortcut_key_matches(shortcut_key: ShortcutKey, actual_key: &Key) -> bool {
    match (shortcut_key, actual_key) {
        (ShortcutKey::Character(expected), Key::Character(actual)) => {
            actual.eq_ignore_ascii_case(expected)
        }
        (ShortcutKey::Named("Enter"), Key::Named(NamedKey::Enter)) => true,
        (ShortcutKey::Named("Tab"), Key::Named(NamedKey::Tab)) => true,
        (ShortcutKey::Named("Escape"), Key::Named(NamedKey::Escape)) => true,
        _ => false,
    }
}

pub(crate) fn resolve_shortcut_command(
    command_registry: &CommandRegistry,
    keypress: &KeyboardEvent,
) -> Option<&'static str> {
    command_registry.iter().find_map(|command| {
        command
            .default_shortcut
            .filter(|shortcut| matches_shortcut(*shortcut, keypress))
            .map(|_| command.id)
    })
}
