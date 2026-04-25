use gpui::{App, AppContext as _, WindowBounds, WindowOptions, actions, px, size};
use gpui_component::Root;
use gpui_component_assets::Assets;

mod documents;
mod menus;
mod view;
mod window;

use window::ProbeWindow;

actions!(probe, [New, Open, Save, SaveAs]);

fn main() {
    let app = gpui_platform::application().with_assets(Assets);

    app.run(move |cx: &mut App| {
        gpui_component::init(cx);

        let window_options = WindowOptions {
            window_bounds: Some(WindowBounds::centered(size(px(900.), px(650.)), cx)),
            titlebar: None,
            ..Default::default()
        };

        cx.spawn(async move |cx| {
            cx.open_window(window_options, |window, cx| {
                let view = cx.new(|cx| ProbeWindow::new(window, cx));
                window.on_window_should_close(cx, {
                    let view = view.clone();
                    move |window, cx| {
                        view.update(cx, |this, cx| this.should_close_window(window, cx))
                    }
                });
                cx.new(|cx| Root::new(view, window, cx))
            })
            .expect("open GPUI editor probe window");
        })
        .detach();
    });
}
