use wxdragon::appearance::{AppAppearance, Appearance, AppearanceResult, get_app};
use wxdragon::prelude::*;

wxdragon::include_xrc!("../../xrc/ui.xrc", FrameUI);

pub fn show() {
    if let Some(app) = get_app() {
        match app.set_appearance(Appearance::System) {
            AppearanceResult::Ok => {}
            AppearanceResult::Failure => {}
            AppearanceResult::CannotChange => {}
        }
    }

    let frame_ui = FrameUI::new(None, false);

    frame_ui.MainFrame.center_on_screen();
    frame_ui.MainFrame.show(true);
    frame_ui.MainFrame.layout();

    log::info!("Window loaded successfully!");
}
