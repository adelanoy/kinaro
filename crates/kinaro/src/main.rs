use assets::Assets;
use gpui::*;
use gpui_component::{
    button::{Button, ButtonVariants}, Root,
    StyledExt,
};
use log::{ info};

pub struct HelloWorld;
impl Render for HelloWorld {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div()
            .v_flex()
            .gap_2()
            .size_full()
            .items_center()
            .justify_center()
            .child("Hello, World!")
            .child(
                Button::new("ok")
                    .primary()
                    .label("Let's Go!")
                    .on_click(|_, _, _| println!("Clicked!")),
            )
    }
}

fn main() {
    let config_dir = dirs::config_local_dir().unwrap().join("Kinaro");
    Application::new().with_assets(Assets).run(move |cx| {
        init_app(&config_dir, cx);
        cx.spawn(async move |cx| {
            cx.open_window(WindowOptions::default(), |window, cx| {
                let view = cx.new(|_| HelloWorld);
                cx.new(|cx| Root::new(view, window, cx))
            })
            .expect("Failed to open window");
        })
        .detach();
    });
}

fn init_app(config_dir: &std::path::PathBuf, cx: &mut App) {
    if let Err(err) = klog::init(config_dir) {
        eprintln!("Failed to initialize logger: {}", err);
    }

    info!("Initializing component lib");
    gpui_component::init(cx);
}
