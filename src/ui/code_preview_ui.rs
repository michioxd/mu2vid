use wxdragon::appearance::is_system_dark_mode;
use wxdragon::color::Colour;
use wxdragon::font::{Font, FontFamily, FontStyle, FontWeight};
use wxdragon::geometry::Size;
use wxdragon::prelude::*;

const WINDOW_WIDTH: i32 = 720;
const WINDOW_HEIGHT: i32 = 560;
const MIN_WINDOW_WIDTH: i32 = 520;
const MIN_WINDOW_HEIGHT: i32 = 360;
const BORDER: i32 = 8;

#[derive(Clone)]
pub struct CodePreviewUI {
    pub frame: Frame,
    pub code_text: TextCtrl,
}

impl CodePreviewUI {
    pub fn new(_parent: &Frame) -> Self {
        let frame = Frame::builder()
            .with_title("Preview project file")
            .with_size(Size::new(WINDOW_WIDTH, WINDOW_HEIGHT))
            .with_style(
                FrameStyle::Caption
                    | FrameStyle::SystemMenu
                    | FrameStyle::CloseBox
                    | FrameStyle::ResizeBorder
                    | FrameStyle::MinimizeBox
                    | FrameStyle::MaximizeBox,
            )
            .build();
        frame.set_min_size(Size::new(MIN_WINDOW_WIDTH, MIN_WINDOW_HEIGHT));
        frame.center_on_screen();

        let panel = Panel::builder(&frame).build();
        let sizer = BoxSizer::builder(Orientation::Vertical).build();
        let code_text = TextCtrl::builder(&panel)
            .with_style(TextCtrlStyle::MultiLine | TextCtrlStyle::ReadOnly | TextCtrlStyle::Rich2)
            .build();
        configure_code_text(&code_text);
        sizer.add(&code_text, 1, SizerFlag::Expand | SizerFlag::All, BORDER);
        panel.set_sizer(sizer, true);

        Self { frame, code_text }
    }
}

fn configure_code_text(code_text: &TextCtrl) {
    if let Some(font) = Font::new_with_details(
        10,
        FontFamily::Modern.as_i32(),
        FontStyle::Normal.as_i32(),
        FontWeight::Normal.as_i32(),
        false,
        "Consolas",
    ) {
        code_text.set_font(&font);
    }

    code_text.set_foreground_color(if is_system_dark_mode() {
        Colour::rgb(240, 240, 240)
    } else {
        Colour::rgb(0, 0, 0)
    });
    code_text.set_background_color(if is_system_dark_mode() {
        Colour::rgb(32, 32, 32)
    } else {
        Colour::rgb(240, 240, 240)
    });
}
