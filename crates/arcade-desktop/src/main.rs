//! Desktop entry point for the custom GPUI-rendered ArcadeEdit shell.

use arcade_ui::ArcadeShell;
use gpui::{px, size, App, AppContext, Bounds, WindowBounds, WindowOptions};
use gpui_platform::application;

fn main() {
    application().run(|cx: &mut App| {
        let bounds = Bounds::centered(None, size(px(1100.0), px(720.0)), cx);

        cx.open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(bounds)),
                ..Default::default()
            },
            |_, cx| cx.new(|cx| ArcadeShell::welcome(cx)),
        )
        .expect("ArcadeEdit could not open its desktop window");

        cx.activate(true);
    });
}
