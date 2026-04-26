use gpui::{Context, Entity, SharedString, Subscription, Window};
use gpui_component::{
    WindowExt, button::ButtonVariant, dialog::DialogButtonProps, menu::AppMenuBar,
};

use crate::{
    ClearRecentFiles, New, Open, OpenRecent, ResetFontSize, Save, SaveAs, SelectEditorFont, ZoomIn,
    ZoomOut,
    documents::{DocumentId, ProbeDocument, SAMPLE_MARKDOWN},
    menus::{install_app_menus, set_app_menus},
    persistence::{AppConfig, RecentFiles, load_app_config, load_recent_files, store_app_config},
    preferences::{
        decrease_editor_font_size, default_editor_font_size, editor_font_label,
        increase_editor_font_size, normalize_editor_font, normalize_editor_font_size,
    },
};

pub(crate) struct ProbeWindow {
    pub(crate) documents: Vec<ProbeDocument>,
    pub(crate) active_document_id: Option<DocumentId>,
    pub(crate) next_document_id: DocumentId,
    pub(crate) app_menu_bar: Entity<AppMenuBar>,
    pub(crate) recent_files: RecentFiles,
    pub(crate) app_config: AppConfig,
    pub(crate) available_font_families: Vec<String>,
    pub(crate) status: SharedString,
    pub(crate) _subscriptions: Vec<Subscription>,
}

impl ProbeWindow {
    pub(crate) fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let (recent_files, load_status) = match load_recent_files() {
            Ok(recent_files) => (recent_files, None),
            Err(err) => (RecentFiles::default(), Some(err.into())),
        };
        let (app_config, config_status) = match load_app_config() {
            Ok(app_config) => (app_config, None),
            Err(err) => (AppConfig::default(), Some(err.into())),
        };

        install_app_menus(cx, &recent_files, &app_config.editor_font);
        let app_menu_bar = AppMenuBar::new(cx);
        app_menu_bar.update(cx, |menu_bar, cx| menu_bar.reload(cx));
        let available_font_families = cx.text_system().all_font_names();

        let mut this = Self {
            documents: Vec::new(),
            active_document_id: None,
            next_document_id: DocumentId::initial(),
            app_menu_bar,
            recent_files,
            app_config,
            available_font_families,
            status: "Ready".into(),
            _subscriptions: Vec::new(),
        };
        this.create_document(
            None,
            SAMPLE_MARKDOWN.to_string(),
            "Opened the GPUI editor probe".into(),
            window,
            cx,
        );
        if let Some(status) = load_status {
            this.status = status;
        }
        if let Some(status) = config_status {
            this.status = status;
        }
        this
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
            .map(ProbeDocument::saved_title)
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
        set_app_menus(cx, &self.recent_files, &self.app_config.editor_font);
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
