use std::rc::Rc;

use floem::{
    prelude::{SignalGet, SignalUpdate},
    text::FamilyOwned,
    views::editor::text::{SimpleStyling, Styling},
};

use crate::{config::store_app_config, state::AppState};

pub(crate) const SYSTEM_DEFAULT_FONT: &str = "system_default";
pub(crate) const SANS_SERIF_FONT: &str = "sans_serif";
pub(crate) const SERIF_FONT: &str = "serif";
pub(crate) const MONOSPACE_FONT: &str = "monospace";
pub(crate) const CURSIVE_FONT: &str = "cursive";
pub(crate) const FANTASY_FONT: &str = "fantasy";

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

pub(crate) fn editor_styling(font_family: &str) -> Rc<dyn Styling> {
    let mut builder = SimpleStyling::builder();
    builder.font_family(editor_font_family(font_family));
    Rc::new(builder.build())
}

pub(crate) fn apply_editor_font(state: &AppState, font_family: String) {
    let font_family = normalize_editor_font(&font_family);
    state.set_editor_font(font_family.clone());

    if let Err(err) = store_app_config(state.app_config.get_untracked()) {
        state.status_message.set(Some(err));
        return;
    }

    for document in state.documents() {
        document.editor.update_styling(editor_styling(&font_family));
    }

    state.status_message.set(Some(format!(
        "Editor font set to {}",
        editor_font_label(&font_family)
    )));
}

#[cfg(test)]
mod tests {
    use super::{
        MONOSPACE_FONT, SYSTEM_DEFAULT_FONT, available_editor_fonts, default_editor_font,
        editor_font_label, normalize_editor_font,
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
}
