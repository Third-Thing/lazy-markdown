use std::{fs, path::PathBuf, rc::Rc};

use gpui::{Context, Entity, SharedString, Subscription, Window};
use gpui_component::{
    Theme, ThemeConfig, ThemeRegistry, ThemeSet, WindowExt, button::ButtonVariant,
    dialog::DialogButtonProps, menu::AppMenuBar,
};
use serde_json::{Map, Value};

use crate::{
    ClearRecentFiles, New, Open, OpenRecent, ResetFontSize, Save, SaveAs, SelectEditorFont,
    SelectTheme, ZoomIn, ZoomOut,
    documents::{Document, DocumentId},
    menus::{install_app_menus, set_app_menus},
    persistence::{
        AppConfig, GpuiThemePreference, RecentFiles, custom_theme_path, load_app_config,
        load_recent_files, store_app_config,
    },
    preferences::{
        EditorFontFamilies, decrease_editor_font_size, default_editor_font_size,
        editor_font_label, increase_editor_font_size, normalize_editor_font,
        normalize_editor_font_size,
    },
};

pub(crate) struct AppWindow {
    pub(crate) documents: Vec<Document>,
    pub(crate) active_document_id: Option<DocumentId>,
    pub(crate) next_document_id: DocumentId,
    pub(crate) app_menu_bar: Entity<AppMenuBar>,
    pub(crate) recent_files: RecentFiles,
    pub(crate) app_config: AppConfig,
    pub(crate) editor_font_families: EditorFontFamilies,
    pub(crate) status: SharedString,
    pub(crate) _subscriptions: Vec<Subscription>,
}

impl AppWindow {
    pub(crate) fn new(
        initial_path: Option<PathBuf>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let (recent_files, load_status) = match load_recent_files() {
            Ok(recent_files) => (recent_files, None),
            Err(err) => (RecentFiles::default(), Some(err.into())),
        };
        let (app_config, config_status) = match load_app_config() {
            Ok(app_config) => (app_config, None),
            Err(err) => (AppConfig::default(), Some(err.into())),
        };

        let theme_status = apply_startup_theme(&app_config, window, cx).err();
        install_app_menus(
            cx,
            &recent_files,
            &app_config.editor_font,
            app_config.gpui_theme,
        );
        let app_menu_bar = AppMenuBar::new(cx);
        app_menu_bar.update(cx, |menu_bar, cx| menu_bar.reload(cx));
        let editor_font_families = EditorFontFamilies::from_fontconfig();

        let mut this = Self {
            documents: Vec::new(),
            active_document_id: None,
            next_document_id: DocumentId::initial(),
            app_menu_bar,
            recent_files,
            app_config,
            editor_font_families,
            status: "Ready".into(),
            _subscriptions: Vec::new(),
        };
        let opened_startup_path = initial_path.is_some();
        this.open_startup_document(initial_path, window, cx);
        if !opened_startup_path {
            if let Some(status) = load_status {
                this.status = status;
            }
            if let Some(status) = config_status {
                this.status = status;
            }
            if let Some(status) = theme_status {
                this.status = status.into();
            }
        }
        this
    }

    fn open_startup_document(
        &mut self,
        initial_path: Option<PathBuf>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(path) = initial_path else {
            self.create_blank_startup_document(window, cx, "Ready".into());
            return;
        };

        match fs::read_to_string(&path) {
            Ok(contents) => {
                self.create_document(
                    Some(path.clone()),
                    contents,
                    format!("Opened {}", path.display()).into(),
                    window,
                    cx,
                );
                self.record_recent_file(&path, cx);
            }
            Err(err) => {
                self.create_blank_startup_document(
                    window,
                    cx,
                    format!("Failed to open {}: {err}", path.display()).into(),
                );
            }
        }
    }

    fn create_blank_startup_document(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
        status: SharedString,
    ) {
        self.create_document(None, String::new(), status, window, cx);
    }

    pub(crate) fn on_new(&mut self, _: &New, window: &mut Window, cx: &mut Context<Self>) {
        self.create_new_document(window, cx);
    }

    pub(crate) fn on_open(&mut self, _: &Open, window: &mut Window, cx: &mut Context<Self>) {
        self.prompt_open_document(window, cx);
    }

    pub(crate) fn on_save(&mut self, _: &Save, window: &mut Window, cx: &mut Context<Self>) {
        let Some(document) = self.active_document() else {
            return;
        };
        let document_id = document.id;

        match document.current_path.clone() {
            Some(path) => self.save_to_path(document_id, &path, cx),
            None => self.prompt_save_as(document_id, window, cx),
        }
    }

    pub(crate) fn on_save_as(&mut self, _: &SaveAs, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(document_id) = self.active_document_id {
            self.prompt_save_as(document_id, window, cx);
        }
    }

    pub(crate) fn on_open_recent(
        &mut self,
        action: &OpenRecent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.open_document_path_from_disk(action.0.clone().into(), window, cx);
    }

    pub(crate) fn on_clear_recent_files(
        &mut self,
        _: &ClearRecentFiles,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.clear_recent_files(cx);
    }

    pub(crate) fn on_select_editor_font(
        &mut self,
        action: &SelectEditorFont,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.apply_editor_font(action.0.clone(), cx);
    }

    pub(crate) fn on_select_theme(
        &mut self,
        action: &SelectTheme,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(theme_preference) = GpuiThemePreference::from_config_value(&action.0) else {
            self.status = format!("Unknown theme: {}", action.0).into();
            cx.notify();
            return;
        };

        match apply_theme_preference(theme_preference, window, cx) {
            Ok(status) => {
                self.app_config.gpui_theme = theme_preference;
                self.status = status.into();
                self.persist_app_config(cx);
            }
            Err(err) => {
                self.status = err.into();
            }
        }
        self.reload_app_menus(cx);
        cx.notify();
    }

    pub(crate) fn on_zoom_in(&mut self, _: &ZoomIn, _: &mut Window, cx: &mut Context<Self>) {
        let size = increase_editor_font_size(self.app_config.editor_font_size);
        self.apply_editor_font_size(size, cx);
    }

    pub(crate) fn on_zoom_out(&mut self, _: &ZoomOut, _: &mut Window, cx: &mut Context<Self>) {
        let size = decrease_editor_font_size(self.app_config.editor_font_size);
        self.apply_editor_font_size(size, cx);
    }

    pub(crate) fn on_reset_font_size(
        &mut self,
        _: &ResetFontSize,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.apply_editor_font_size(default_editor_font_size(), cx);
    }

    pub(crate) fn should_close_window(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> bool {
        let dirty_documents = self.dirty_document_ids();
        if dirty_documents.is_empty() {
            return true;
        }

        if !window.has_active_dialog(cx) {
            self.activate_document(dirty_documents[0], window, cx);
            self.open_close_window_dialog(window, cx);
        }

        false
    }

    fn open_close_window_dialog(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let document_name = self
            .active_document()
            .map(Document::saved_title)
            .unwrap_or_else(|| "Untitled".to_string());
        let description = format!("{document_name} has unsaved changes. Close without saving?");

        window.open_alert_dialog(cx, move |dialog, _, _| {
            dialog
                .title("Discard changes and close?")
                .description(description.clone())
                .button_props(
                    DialogButtonProps::default()
                        .ok_text("Discard and Close")
                        .ok_variant(ButtonVariant::Danger)
                        .cancel_text("Keep Editing")
                        .show_cancel(true),
                )
                .on_ok(|_, window, _| {
                    window.remove_window();
                    true
                })
        });
    }

    pub(crate) fn reload_app_menus(&self, cx: &mut Context<Self>) {
        set_app_menus(
            cx,
            &self.recent_files,
            &self.app_config.editor_font,
            self.app_config.gpui_theme,
        );
        self.app_menu_bar.update(cx, |menu_bar, cx| {
            menu_bar.reload(cx);
        });
    }

    fn apply_editor_font(&mut self, font_family: String, cx: &mut Context<Self>) {
        let font_family = normalize_editor_font(&font_family);
        self.app_config.editor_font = font_family.clone();
        self.status = format!("Editor font set to {}", editor_font_label(&font_family)).into();
        self.persist_app_config(cx);
        self.reload_app_menus(cx);
        cx.notify();
    }

    fn apply_editor_font_size(&mut self, font_size: usize, cx: &mut Context<Self>) {
        let font_size = normalize_editor_font_size(font_size);
        self.app_config.editor_font_size = font_size;
        self.status = format!("Editor font size set to {font_size}px").into();
        self.persist_app_config(cx);
        cx.notify();
    }

    fn persist_app_config(&mut self, cx: &mut Context<Self>) {
        if let Err(err) = store_app_config(&self.app_config) {
            self.status = err.into();
            cx.notify();
        }
    }
}

fn apply_startup_theme(
    config: &AppConfig,
    window: &mut Window,
    cx: &mut Context<AppWindow>,
) -> Result<(), String> {
    apply_theme_preference(config.gpui_theme, window, cx).map(|_| ())
}

fn apply_theme_preference(
    theme_preference: GpuiThemePreference,
    window: &mut Window,
    cx: &mut Context<AppWindow>,
) -> Result<String, String> {
    match theme_preference {
        GpuiThemePreference::DefaultLight => {
            let theme_config = ThemeRegistry::global(cx).default_light_theme().clone();
            apply_editor_theme_config(&theme_config, &theme_config, cx)?;
            window.refresh();
            Ok("Theme set to Default Light".to_string())
        }
        GpuiThemePreference::DefaultDark => {
            let theme_config = ThemeRegistry::global(cx).default_dark_theme().clone();
            apply_editor_theme_config(&theme_config, &theme_config, cx)?;
            window.refresh();
            Ok("Theme set to Default Dark".to_string())
        }
        GpuiThemePreference::Custom => apply_custom_theme(window, cx),
    }
}

fn apply_custom_theme(window: &mut Window, cx: &mut Context<AppWindow>) -> Result<String, String> {
    let path = custom_theme_path()?;
    let contents = fs::read_to_string(&path)
        .map_err(|err| format!("Failed to read {}: {err}", path.display()))?;
    let theme_set = serde_json::from_str::<ThemeSet>(&contents)
        .map_err(|err| format!("Failed to parse {}: {err}", path.display()))?;
    let Some(theme_config) = theme_set.themes.first() else {
        return Err(format!("No themes found in {}", path.display()));
    };

    let default_theme_config = if theme_config.mode.is_dark() {
        ThemeRegistry::global(cx).default_dark_theme().clone()
    } else {
        ThemeRegistry::global(cx).default_light_theme().clone()
    };
    apply_editor_theme_config(theme_config, &default_theme_config, cx)?;
    window.refresh();
    Ok(format!("Custom theme loaded: {}", theme_config.name))
}

fn apply_editor_theme_config(
    theme_config: &ThemeConfig,
    default_theme_config: &ThemeConfig,
    cx: &mut Context<AppWindow>,
) -> Result<(), String> {
    let theme_config = theme_config_with_default_highlights(theme_config, default_theme_config)?;
    Theme::global_mut(cx).apply_config(&theme_config);
    Ok(())
}

fn theme_config_with_default_highlights(
    theme_config: &ThemeConfig,
    default_theme_config: &ThemeConfig,
) -> Result<Rc<ThemeConfig>, String> {
    let mut value = serde_json::to_value(theme_config)
        .map_err(|err| format!("Failed to prepare editor theme: {err}"))?;
    let default_value = serde_json::to_value(default_theme_config)
        .map_err(|err| format!("Failed to prepare editor theme: {err}"))?;
    let Some(theme) = value.as_object_mut() else {
        return Err("Failed to prepare editor theme: expected theme object".to_string());
    };

    let highlight = object_field(theme, "highlight");
    if let Some(default_highlight) = default_value.get("highlight") {
        merge_missing_fields(highlight, default_highlight);
    }

    serde_json::from_value::<ThemeConfig>(value)
        .map(Rc::new)
        .map_err(|err| format!("Failed to prepare editor theme: {err}"))
}

fn object_field<'a>(object: &'a mut Map<String, Value>, key: &str) -> &'a mut Map<String, Value> {
    let value = object
        .entry(key.to_string())
        .or_insert_with(|| Value::Object(Map::new()));
    if !value.is_object() {
        *value = Value::Object(Map::new());
    }
    value
        .as_object_mut()
        .expect("object field was just created")
}

fn merge_missing_fields(object: &mut Map<String, Value>, defaults: &Value) {
    let Some(defaults) = defaults.as_object() else {
        return;
    };

    for (key, default_value) in defaults {
        match (object.get_mut(key), default_value.as_object()) {
            (Some(Value::Object(value)), Some(_)) => merge_missing_fields(value, default_value),
            (Some(value @ Value::Null), _) => {
                if !default_value.is_null() {
                    *value = default_value.clone();
                }
            }
            (Some(_), _) => {}
            (None, _) => {
                object.insert(key.clone(), default_value.clone());
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use gpui_component::{ThemeConfig, ThemeMode, highlighter::LanguageRegistry};
    use serde_json::json;

    use super::theme_config_with_default_highlights;

    #[test]
    fn markdown_language_is_registered_for_code_editor() {
        let registry = LanguageRegistry::singleton();

        assert!(registry.language("markdown").is_some());
        assert!(registry.language("markdown_inline").is_some());
    }

    #[test]
    fn markdown_strong_theme_style_inherits_default_weight() {
        let theme_config = ThemeConfig {
            name: "Test".into(),
            mode: ThemeMode::Light,
            ..ThemeConfig::default()
        };
        let default_theme_config = serde_json::from_value::<ThemeConfig>(json!({
            "name": "Default",
            "mode": "light",
            "colors": {},
            "highlight": {
                "syntax": {
                    "emphasis.strong": {
                        "font_weight": 700
                    }
                }
            }
        }))
        .unwrap();

        let theme_config =
            theme_config_with_default_highlights(&theme_config, &default_theme_config).unwrap();
        let theme_config = serde_json::to_value(&*theme_config).unwrap();

        assert_eq!(
            theme_config["highlight"]["syntax"]["emphasis.strong"]["font_weight"],
            json!(700)
        );
        assert!(theme_config["highlight"]["syntax"]["emphasis.strong"]["color"].is_null());
    }

    #[test]
    fn markdown_strong_theme_style_keeps_existing_style() {
        let theme_config = serde_json::from_value::<ThemeConfig>(json!({
            "name": "Test",
            "mode": "light",
            "colors": {},
            "highlight": {
                "syntax": {
                    "emphasis.strong": {
                        "color": "#111111",
                        "font_weight": 600
                    }
                }
            }
        }))
        .unwrap();

        let theme_config =
            theme_config_with_default_highlights(&theme_config, &theme_config).unwrap();
        let theme_config = serde_json::to_value(&*theme_config).unwrap();

        assert_eq!(
            theme_config["highlight"]["syntax"]["emphasis.strong"]["font_weight"],
            json!(600)
        );
        assert_eq!(
            theme_config["highlight"]["syntax"]["emphasis.strong"]["color"],
            json!("#111111ff")
        );
    }

    #[test]
    fn markdown_highlights_inherit_missing_default_styles() {
        let theme_config = serde_json::from_value::<ThemeConfig>(json!({
            "name": "Custom",
            "mode": "light",
            "colors": {},
            "highlight": {
                "syntax": {
                    "title": {
                        "color": "#111111"
                    }
                }
            }
        }))
        .unwrap();
        let default_theme_config = serde_json::from_value::<ThemeConfig>(json!({
            "name": "Default",
            "mode": "light",
            "colors": {},
            "highlight": {
                "editor.background": "#ffffff",
                "syntax": {
                    "title": {
                        "color": "#222222"
                    },
                    "link_uri": {
                        "color": "#333333"
                    }
                }
            }
        }))
        .unwrap();

        let theme_config =
            theme_config_with_default_highlights(&theme_config, &default_theme_config).unwrap();
        let theme_config = serde_json::to_value(&*theme_config).unwrap();

        assert_eq!(
            theme_config["highlight"]["syntax"]["title"]["color"],
            json!("#111111ff")
        );
        assert_eq!(
            theme_config["highlight"]["syntax"]["link_uri"]["color"],
            json!("#333333ff")
        );
        assert_eq!(
            theme_config["highlight"]["editor.background"],
            json!("#ffffffff")
        );
    }
}
