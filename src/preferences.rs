use gpui::{App, SharedString};
use gpui_component::ActiveTheme as _;

pub(crate) const SYSTEM_DEFAULT_FONT: &str = "system_default";
pub(crate) const SANS_SERIF_FONT: &str = "sans_serif";
pub(crate) const SERIF_FONT: &str = "serif";
pub(crate) const MONOSPACE_FONT: &str = "monospace";
pub(crate) const CURSIVE_FONT: &str = "cursive";
pub(crate) const FANTASY_FONT: &str = "fantasy";
pub(crate) const DEFAULT_EDITOR_FONT_SIZE: usize = 16;
pub(crate) const MIN_EDITOR_FONT_SIZE: usize = 8;
pub(crate) const MAX_EDITOR_FONT_SIZE: usize = 48;
const EDITOR_FONT_SIZE_STEP: usize = 1;

const EDITOR_FONT_OPTIONS: &[&str] = &[
    SYSTEM_DEFAULT_FONT,
    SANS_SERIF_FONT,
    SERIF_FONT,
    MONOSPACE_FONT,
    CURSIVE_FONT,
    FANTASY_FONT,
];

const SERIF_FONT_CANDIDATES: &[&str] = &[
    "Source Serif Pro",
    "Noto Serif",
    "Liberation Serif",
    "DejaVu Serif",
    "Georgia",
    "Times New Roman",
    "Times",
    "Adobe Times",
    "Luxi Serif",
];

const CURSIVE_FONT_CANDIDATES: &[&str] = &[
    "Comic Neue",
    "Comic Sans MS",
    "Apple Chancery",
    "Segoe Script",
    "Bradley Hand",
    "Brush Script MT",
    "URW Chancery L",
    "Snell Roundhand",
    "Zapfino",
    "Z003",
];

const FANTASY_FONT_CANDIDATES: &[&str] = &[
    "Lobster 1.4",
    "Impact",
    "Papyrus",
    "Copperplate",
    "Herculanum",
    "Jazz LET",
    "Luminari",
];

pub(crate) fn default_editor_font() -> String {
    SYSTEM_DEFAULT_FONT.to_string()
}

pub(crate) fn default_editor_font_size() -> usize {
    DEFAULT_EDITOR_FONT_SIZE
}

pub(crate) fn available_editor_fonts() -> Vec<&'static str> {
    EDITOR_FONT_OPTIONS.to_vec()
}

pub(crate) fn normalize_editor_font(requested_family: &str) -> String {
    let requested_family = requested_family.trim();
    if EDITOR_FONT_OPTIONS.contains(&requested_family) {
        requested_family.to_string()
    } else {
        default_editor_font()
    }
}

pub(crate) fn normalize_editor_font_size(requested_size: usize) -> usize {
    requested_size.clamp(MIN_EDITOR_FONT_SIZE, MAX_EDITOR_FONT_SIZE)
}

pub(crate) fn editor_font_label(font_family: &str) -> &'static str {
    match font_family {
        SYSTEM_DEFAULT_FONT => "System Default",
        SANS_SERIF_FONT => "Sans Serif",
        SERIF_FONT => "Serif",
        MONOSPACE_FONT => "Monospace",
        CURSIVE_FONT => "Cursive",
        FANTASY_FONT => "Fantasy",
        _ => "System Default",
    }
}

pub(crate) fn editor_font_family(
    font_family: &str,
    available_font_families: &[String],
    cx: &App,
) -> SharedString {
    editor_font_family_from_available(
        font_family,
        cx.theme().font_family.clone(),
        cx.theme().mono_font_family.clone(),
        available_font_families,
    )
}

fn editor_font_family_from_available(
    font_family: &str,
    default_family: SharedString,
    mono_family: SharedString,
    available_font_families: &[String],
) -> SharedString {
    match font_family {
        SYSTEM_DEFAULT_FONT | SANS_SERIF_FONT => default_family.clone(),
        MONOSPACE_FONT => mono_family,
        SERIF_FONT => first_available_font(available_font_families, SERIF_FONT_CANDIDATES)
            .unwrap_or(default_family),
        CURSIVE_FONT => first_available_font(available_font_families, CURSIVE_FONT_CANDIDATES)
            .unwrap_or(default_family),
        FANTASY_FONT => first_available_font(available_font_families, FANTASY_FONT_CANDIDATES)
            .unwrap_or(default_family),
        _ => default_family,
    }
}

fn first_available_font(
    available_font_families: &[String],
    candidates: &[&str],
) -> Option<SharedString> {
    candidates.iter().find_map(|candidate| {
        available_font_families
            .iter()
            .find(|font| font.eq_ignore_ascii_case(candidate))
            .map(|font| font.clone().into())
    })
}

pub(crate) fn increase_editor_font_size(current_size: usize) -> usize {
    normalize_editor_font_size(current_size.saturating_add(EDITOR_FONT_SIZE_STEP))
}

pub(crate) fn decrease_editor_font_size(current_size: usize) -> usize {
    normalize_editor_font_size(current_size.saturating_sub(EDITOR_FONT_SIZE_STEP))
}

#[cfg(test)]
mod tests {
    use super::{
        CURSIVE_FONT, FANTASY_FONT, MAX_EDITOR_FONT_SIZE, MIN_EDITOR_FONT_SIZE, MONOSPACE_FONT,
        SERIF_FONT, SYSTEM_DEFAULT_FONT, available_editor_fonts, default_editor_font,
        default_editor_font_size, editor_font_family_from_available, editor_font_label,
        normalize_editor_font, normalize_editor_font_size,
    };

    #[test]
    fn system_default_font_option_is_first() {
        let fonts = available_editor_fonts();
        assert_eq!(fonts.first().copied(), Some(SYSTEM_DEFAULT_FONT));
    }

    #[test]
    fn generic_font_labels_are_human_readable() {
        assert_eq!(editor_font_label(MONOSPACE_FONT), "Monospace");
    }

    #[test]
    fn unknown_font_falls_back_to_system_default() {
        assert_eq!(
            normalize_editor_font("definitely-not-a-real-font"),
            default_editor_font()
        );
    }

    #[test]
    fn editor_font_size_defaults_to_app_default() {
        assert_eq!(default_editor_font_size(), 16);
    }

    #[test]
    fn editor_font_size_is_clamped_to_supported_range() {
        assert_eq!(normalize_editor_font_size(0), MIN_EDITOR_FONT_SIZE);
        assert_eq!(
            normalize_editor_font_size(MAX_EDITOR_FONT_SIZE + 20),
            MAX_EDITOR_FONT_SIZE
        );
    }

    #[test]
    fn generic_font_families_resolve_to_installed_concrete_fonts() {
        let available = vec![
            "Default Sans".to_string(),
            "Liberation Serif".to_string(),
            "Comic Neue".to_string(),
            "Lobster 1.4".to_string(),
        ];

        assert_eq!(
            editor_font_family_from_available(
                SERIF_FONT,
                "Default Sans".into(),
                "Default Mono".into(),
                &available,
            )
            .as_ref(),
            "Liberation Serif"
        );
        assert_eq!(
            editor_font_family_from_available(
                CURSIVE_FONT,
                "Default Sans".into(),
                "Default Mono".into(),
                &available,
            )
            .as_ref(),
            "Comic Neue"
        );
        assert_eq!(
            editor_font_family_from_available(
                FANTASY_FONT,
                "Default Sans".into(),
                "Default Mono".into(),
                &available,
            )
            .as_ref(),
            "Lobster 1.4"
        );
    }

    #[test]
    fn generic_font_families_fall_back_when_no_concrete_font_is_installed() {
        let available = vec!["Default Sans".to_string()];

        assert_eq!(
            editor_font_family_from_available(
                SERIF_FONT,
                "Default Sans".into(),
                "Default Mono".into(),
                &available,
            )
            .as_ref(),
            "Default Sans"
        );
    }
}
