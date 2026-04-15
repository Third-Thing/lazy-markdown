use floem::prelude::{Key, KeyboardEvent, Modifiers};

use crate::commands::{CommandRegistry, Shortcut, ShortcutKey, ShortcutModifier};

fn supported_modifiers(modifiers: Modifiers) -> Modifiers {
    modifiers & (Modifiers::CONTROL | Modifiers::ALT | Modifiers::SHIFT | Modifiers::META)
}

fn matches_shortcut(shortcut: Shortcut, keypress: &KeyboardEvent) -> bool {
    supported_modifiers(keypress.modifiers) == shortcut_modifiers(shortcut.modifiers)
        && shortcut_key_matches(shortcut.key, keypress)
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

fn shortcut_key_matches(shortcut_key: ShortcutKey, keypress: &KeyboardEvent) -> bool {
    match (shortcut_key, &keypress.key) {
        (ShortcutKey::Character(expected), Key::Character(actual)) => {
            actual.eq_ignore_ascii_case(expected)
        }
        (ShortcutKey::Named(expected), Key::Named(actual)) => *actual == expected,
        (ShortcutKey::Code(expected), _) => keypress.code == expected,
        _ => false,
    }
}

pub(crate) fn resolve_shortcut_command(
    command_registry: &CommandRegistry,
    keypress: &KeyboardEvent,
) -> Option<&'static str> {
    command_registry.iter().find_map(|command| {
        command
            .default_shortcuts
            .iter()
            .copied()
            .find(|shortcut| matches_shortcut(*shortcut, keypress))
            .map(|_| command.id)
    })
}

#[cfg(test)]
mod tests {
    use floem::prelude::{Code, Key, KeyState, KeyboardEvent, Modifiers};

    use crate::commands::{
        CommandMetadata, CommandRegistry, Shortcut, ShortcutKey, ShortcutModifier,
    };

    use super::resolve_shortcut_command;

    const CONTROL: &[ShortcutModifier] = &[ShortcutModifier::Control];
    const CONTROL_SHIFT: &[ShortcutModifier] =
        &[ShortcutModifier::Control, ShortcutModifier::Shift];
    const TEST_SHORTCUTS: &[Shortcut] = &[
        Shortcut {
            key: ShortcutKey::Code(Code::Equal),
            modifiers: CONTROL,
        },
        Shortcut {
            key: ShortcutKey::Code(Code::Equal),
            modifiers: CONTROL_SHIFT,
        },
        Shortcut {
            key: ShortcutKey::Code(Code::NumpadAdd),
            modifiers: CONTROL,
        },
    ];

    #[test]
    fn resolves_matching_physical_code_with_or_without_shift() {
        let registry = test_registry();

        assert_eq!(
            resolve_shortcut_command(
                &registry,
                &keypress(Key::Character("=".into()), Code::Equal, Modifiers::CONTROL),
            ),
            Some("test.zoom_in")
        );
        assert_eq!(
            resolve_shortcut_command(
                &registry,
                &keypress(
                    Key::Character("+".into()),
                    Code::Equal,
                    Modifiers::CONTROL | Modifiers::SHIFT,
                ),
            ),
            Some("test.zoom_in")
        );
    }

    #[test]
    fn resolves_matching_numpad_code() {
        let registry = test_registry();

        assert_eq!(
            resolve_shortcut_command(
                &registry,
                &keypress(
                    Key::Character("+".into()),
                    Code::NumpadAdd,
                    Modifiers::CONTROL
                ),
            ),
            Some("test.zoom_in")
        );
    }

    fn test_registry() -> CommandRegistry {
        let mut registry = CommandRegistry::default();
        registry
            .register(CommandMetadata {
                id: "test.zoom_in",
                title: "Zoom In",
                default_shortcuts: TEST_SHORTCUTS,
                placements: &[],
            })
            .expect("register test command");
        registry
    }

    fn keypress(key: Key, code: Code, modifiers: Modifiers) -> KeyboardEvent {
        KeyboardEvent {
            state: KeyState::Down,
            key,
            code,
            modifiers,
            ..Default::default()
        }
    }
}
