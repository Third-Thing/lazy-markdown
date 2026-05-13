#[cfg(any(target_os = "linux", target_os = "freebsd"))]
use std::{
    ffi::{CStr, CString},
    os::raw::c_char,
    ptr,
};

use gpui::{App, SharedString};
use gpui_component::ActiveTheme as _;

pub(crate) const SYSTEM_DEFAULT_FONT: &str = "system_default";
pub(crate) const SANS_SERIF_FONT: &str = "sans_serif";
pub(crate) const SERIF_FONT: &str = "serif";
pub(crate) const MONOSPACE_FONT: &str = "monospace";
pub(crate) const DEFAULT_EDITOR_FONT_SIZE: usize = 16;
pub(crate) const MIN_EDITOR_FONT_SIZE: usize = 8;
pub(crate) const MAX_EDITOR_FONT_SIZE: usize = 48;
const EDITOR_FONT_SIZE_STEP: usize = 1;

const EDITOR_FONT_OPTIONS: &[&str] =
    &[SYSTEM_DEFAULT_FONT, SANS_SERIF_FONT, SERIF_FONT, MONOSPACE_FONT];

#[derive(Clone, Debug, Default)]
pub(crate) struct EditorFontFamilies {
    system: Option<SharedString>,
    sans_serif: Option<SharedString>,
    serif: Option<SharedString>,
    monospace: Option<SharedString>,
}

impl EditorFontFamilies {
    pub(crate) fn from_fontconfig() -> Self {
        Self {
            system: fontconfig_family("system-ui"),
            sans_serif: fontconfig_family("sans-serif"),
            serif: fontconfig_family("serif"),
            monospace: fontconfig_family("monospace"),
        }
    }

    #[cfg(test)]
    fn from_resolved(
        system: Option<&str>,
        sans_serif: Option<&str>,
        serif: Option<&str>,
        monospace: Option<&str>,
    ) -> Self {
        Self {
            system: system.map(SharedString::from),
            sans_serif: sans_serif.map(SharedString::from),
            serif: serif.map(SharedString::from),
            monospace: monospace.map(SharedString::from),
        }
    }
}

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
        SYSTEM_DEFAULT_FONT => "System",
        SANS_SERIF_FONT => "Sans Serif",
        SERIF_FONT => "Serif",
        MONOSPACE_FONT => "Monospace",
        _ => "System",
    }
}

pub(crate) fn editor_font_family(
    font_family: &str,
    editor_font_families: &EditorFontFamilies,
    cx: &App,
) -> SharedString {
    editor_font_family_from_fontconfig(
        font_family,
        cx.theme().font_family.clone(),
        cx.theme().mono_font_family.clone(),
        editor_font_families,
    )
}

fn editor_font_family_from_fontconfig(
    font_family: &str,
    default_family: SharedString,
    mono_family: SharedString,
    editor_font_families: &EditorFontFamilies,
) -> SharedString {
    match font_family {
        SYSTEM_DEFAULT_FONT => editor_font_families
            .system
            .clone()
            .unwrap_or(default_family),
        SANS_SERIF_FONT => editor_font_families
            .sans_serif
            .clone()
            .unwrap_or(default_family),
        SERIF_FONT => editor_font_families.serif.clone().unwrap_or(default_family),
        MONOSPACE_FONT => editor_font_families.monospace.clone().unwrap_or(mono_family),
        _ => default_family,
    }
}

fn fontconfig_family(family: &str) -> Option<SharedString> {
    #[cfg(any(target_os = "linux", target_os = "freebsd"))]
    {
        return linux_fontconfig_family(family);
    }

    #[cfg(not(any(target_os = "linux", target_os = "freebsd")))]
    {
        let _ = family;
        None
    }
}

#[cfg(any(target_os = "linux", target_os = "freebsd"))]
fn linux_fontconfig_family(family: &str) -> Option<SharedString> {
    let family = CString::new(family).ok()?;
    let config = unsafe { fontconfig_sys::FcInitLoadConfigAndFonts() };
    if config.is_null() {
        return None;
    }

    let pattern = unsafe { fontconfig_sys::FcNameParse(family.as_ptr() as *const _) };
    if pattern.is_null() {
        unsafe {
            fontconfig_sys::FcConfigDestroy(config);
        }
        return None;
    }

    unsafe {
        fontconfig_sys::FcConfigSubstitute(config, pattern, fontconfig_sys::FcMatchPattern);
        fontconfig_sys::FcDefaultSubstitute(pattern);
    }

    let mut result = fontconfig_sys::FcResultNoMatch;
    let matched = unsafe { fontconfig_sys::FcFontMatch(config, pattern, &mut result) };
    let resolved_family = if result == fontconfig_sys::FcResultMatch && !matched.is_null() {
        fontconfig_pattern_family(matched)
    } else {
        None
    };

    unsafe {
        if !matched.is_null() {
            fontconfig_sys::FcPatternDestroy(matched);
        }
        fontconfig_sys::FcPatternDestroy(pattern);
        fontconfig_sys::FcConfigDestroy(config);
    }

    resolved_family
        .filter(|family| !family.trim().is_empty())
        .map(SharedString::from)
}

#[cfg(any(target_os = "linux", target_os = "freebsd"))]
fn fontconfig_pattern_family(pattern: *mut fontconfig_sys::FcPattern) -> Option<String> {
    let mut family = ptr::null_mut();
    let result = unsafe {
        fontconfig_sys::FcPatternGetString(
            pattern,
            fontconfig_sys::constants::FC_FAMILY.as_ptr(),
            0,
            &mut family,
        )
    };
    if result != fontconfig_sys::FcResultMatch || family.is_null() {
        return None;
    }

    unsafe { CStr::from_ptr(family as *const c_char) }
        .to_str()
        .ok()
        .map(str::to_owned)
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
        EditorFontFamilies, MAX_EDITOR_FONT_SIZE, MIN_EDITOR_FONT_SIZE, MONOSPACE_FONT,
        SANS_SERIF_FONT, SERIF_FONT, SYSTEM_DEFAULT_FONT, available_editor_fonts,
        default_editor_font, default_editor_font_size, editor_font_family_from_fontconfig,
        editor_font_label, normalize_editor_font, normalize_editor_font_size,
    };

    #[test]
    fn system_font_option_is_first() {
        let fonts = available_editor_fonts();
        assert_eq!(fonts.first().copied(), Some(SYSTEM_DEFAULT_FONT));
    }

    #[test]
    fn available_fonts_exclude_cursive_and_fantasy() {
        let fonts = available_editor_fonts();

        assert_eq!(
            fonts,
            vec![
                SYSTEM_DEFAULT_FONT,
                SANS_SERIF_FONT,
                SERIF_FONT,
                MONOSPACE_FONT
            ]
        );
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
    fn generic_font_families_use_fontconfig_resolved_fonts() {
        let resolved = EditorFontFamilies::from_resolved(
            Some("System UI"),
            Some("Default Sans"),
            Some("Liberation Serif"),
            Some("Source Code Pro"),
        );

        assert_eq!(
            editor_font_family_from_fontconfig(
                SYSTEM_DEFAULT_FONT,
                "Default Sans".into(),
                "Default Mono".into(),
                &resolved,
            )
            .as_ref(),
            "System UI"
        );
        assert_eq!(
            editor_font_family_from_fontconfig(
                SANS_SERIF_FONT,
                "Default Sans".into(),
                "Default Mono".into(),
                &resolved,
            )
            .as_ref(),
            "Default Sans"
        );
        assert_eq!(
            editor_font_family_from_fontconfig(
                SERIF_FONT,
                "Default Sans".into(),
                "Default Mono".into(),
                &resolved,
            )
            .as_ref(),
            "Liberation Serif"
        );
        assert_eq!(
            editor_font_family_from_fontconfig(
                MONOSPACE_FONT,
                "Default Sans".into(),
                "Default Mono".into(),
                &resolved,
            )
            .as_ref(),
            "Source Code Pro"
        );
    }

    #[test]
    fn generic_font_families_fall_back_when_fontconfig_does_not_resolve() {
        let resolved = EditorFontFamilies::default();

        assert_eq!(
            editor_font_family_from_fontconfig(
                SERIF_FONT,
                "Default Sans".into(),
                "Default Mono".into(),
                &resolved,
            )
            .as_ref(),
            "Default Sans"
        );
        assert_eq!(
            editor_font_family_from_fontconfig(
                MONOSPACE_FONT,
                "Default Sans".into(),
                "Default Mono".into(),
                &resolved,
            )
            .as_ref(),
            "Default Mono"
        );
    }
}
