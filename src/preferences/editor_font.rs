use std::rc::Rc;

use floem::{
    prelude::SignalUpdate,
    text::FamilyOwned,
    views::editor::text::{SimpleStyling, Styling},
};

use crate::workspace::AppState;

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

pub(crate) fn default_editor_font() -> String {
    SYSTEM_DEFAULT_FONT.to_string()
}

pub(crate) fn default_editor_font_size() -> usize {
    DEFAULT_EDITOR_FONT_SIZE
}

pub(crate) fn available_editor_fonts() -> Vec<String> {
    EDITOR_FONT_OPTIONS
        .iter()
        .map(|font| (*font).to_string())
        .collect()
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

fn editor_font_family(font_family: &str) -> Vec<FamilyOwned> {
    match font_family {
        SYSTEM_DEFAULT_FONT | SANS_SERIF_FONT => vec![FamilyOwned::SansSerif],
        SERIF_FONT => vec![FamilyOwned::Serif],
        MONOSPACE_FONT => vec![FamilyOwned::Monospace],
        CURSIVE_FONT => vec![FamilyOwned::Cursive],
        FANTASY_FONT => vec![FamilyOwned::Fantasy],
        _ => vec![FamilyOwned::SansSerif],
    }
}

pub(crate) fn editor_styling(font_family: &str, font_size: usize) -> Rc<dyn Styling> {
    let mut builder = SimpleStyling::builder();
    builder.font_family(editor_font_family(font_family));
    builder.font_size(normalize_editor_font_size(font_size));
    Rc::new(builder.build())
}

pub(crate) fn apply_editor_font(state: &AppState, font_family: String) {
    let font_family = normalize_editor_font(&font_family);
    apply_editor_font_preferences(
        state,
        font_family.clone(),
        state.editor_font_size_untracked(),
        format!("Editor font set to {}", editor_font_label(&font_family)),
    );
}

pub(crate) fn apply_editor_font_size(state: &AppState, font_size: usize) {
    let font_size = normalize_editor_font_size(font_size);
    apply_editor_font_preferences(
        state,
        state.editor_font_untracked(),
        font_size,
        format!("Editor font size set to {font_size}px"),
    );
}

pub(crate) fn increase_editor_font_size(state: &AppState) {
    let current_size = state.editor_font_size_untracked();
    apply_editor_font_size(state, current_size.saturating_add(EDITOR_FONT_SIZE_STEP));
}

pub(crate) fn decrease_editor_font_size(state: &AppState) {
    let current_size = state.editor_font_size_untracked();
    apply_editor_font_size(state, current_size.saturating_sub(EDITOR_FONT_SIZE_STEP));
}

pub(crate) fn reset_editor_font_size(state: &AppState) {
    apply_editor_font_size(state, default_editor_font_size());
}

fn apply_editor_font_preferences(
    state: &AppState,
    font_family: String,
    font_size: usize,
    success_message: String,
) {
    let font_family = normalize_editor_font(&font_family);
    let font_size = normalize_editor_font_size(font_size);

    state.set_editor_font(font_family.clone());
    state.set_editor_font_size(font_size);
    refresh_editor_styling(state, &font_family, font_size);
    state.status_message.set(Some(success_message));
}

fn refresh_editor_styling(state: &AppState, font_family: &str, font_size: usize) {
    for document in state.documents() {
        document
            .editor
            .update_styling(editor_styling(font_family, font_size));
    }
}

#[cfg(test)]
mod tests {
    use super::{
        MAX_EDITOR_FONT_SIZE, MIN_EDITOR_FONT_SIZE, MONOSPACE_FONT, SYSTEM_DEFAULT_FONT,
        available_editor_fonts, default_editor_font, default_editor_font_size, editor_font_label,
        normalize_editor_font, normalize_editor_font_size,
    };

    #[test]
    fn system_default_font_option_is_first() {
        let fonts = available_editor_fonts();
        assert_eq!(fonts.first().map(String::as_str), Some(SYSTEM_DEFAULT_FONT));
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
    fn editor_font_size_defaults_to_floem_default() {
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
}
