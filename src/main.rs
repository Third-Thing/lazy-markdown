use std::{path::PathBuf, sync::Arc};

use gpui::{Action, App, AppContext as _, WindowBounds, WindowOptions, actions, px, size};
use gpui_component::{Root, Theme, scroll::ScrollbarShow};
use gpui_component_assets::Assets;
use serde::Deserialize;

mod documents;
mod menus;
mod persistence;
mod preferences;
mod view;
mod window;

use window::AppWindow;

actions!(
    lazy_markdown,
    [New, Open, Save, SaveAs, ZoomIn, ZoomOut, ResetFontSize]
);

#[derive(Action, Clone, PartialEq, Eq, Deserialize)]
#[action(namespace = lazy_markdown, no_json)]
pub(crate) struct OpenRecent(pub(crate) String);

#[derive(Action, Clone, PartialEq, Eq, Deserialize)]
#[action(namespace = lazy_markdown, no_json)]
pub(crate) struct SelectEditorFont(pub(crate) String);

#[derive(Action, Clone, PartialEq, Eq, Deserialize)]
#[action(namespace = lazy_markdown, no_json)]
pub(crate) struct SelectTheme(pub(crate) String);

#[derive(Action, Clone, PartialEq, Eq, Deserialize)]
#[action(namespace = lazy_markdown, no_json)]
pub(crate) struct ClearRecentFiles;

fn main() {
    let app = gpui_platform::application().with_assets(Assets);
    let initial_path = std::env::args().nth(1).map(PathBuf::from);

    app.run(move |cx: &mut App| {
        gpui_component::init(cx);
        Theme::global_mut(cx).scrollbar_show = ScrollbarShow::Always;

        let window_options = WindowOptions {
            window_bounds: Some(WindowBounds::centered(size(px(900.), px(650.)), cx)),
            titlebar: None,
            app_id: Some("lazy-markdown".into()),
            icon: load_window_icon(),
            ..Default::default()
        };

        cx.spawn(async move |cx| {
            cx.open_window(window_options, |window, cx| {
                let view = cx.new(|cx| AppWindow::new(initial_path.clone(), window, cx));
                window.on_window_should_close(cx, {
                    let view = view.clone();
                    move |window, cx| {
                        view.update(cx, |this, cx| this.should_close_window(window, cx))
                    }
                });
                cx.new(|cx| Root::new(view, window, cx))
            })
            .expect("open GPUI editor window");
        })
        .detach();
    });
}

fn load_window_icon() -> Option<Arc<image::RgbaImage>> {
    let bytes = include_bytes!("../pkg/lazy-markdown-icon-256.png");
    match image::load_from_memory_with_format(bytes, image::ImageFormat::Png) {
        Ok(image) => Some(Arc::new(image.into_rgba8())),
        Err(err) => {
            eprintln!("Failed to load window icon: {err}");
            None
        }
    }
}
