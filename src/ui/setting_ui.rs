use wxdragon::geometry::Size;
use wxdragon::id::{ID_CANCEL, ID_OK};
use wxdragon::prelude::*;
use wxdragon::sizers::StaticBoxSizerBuilder;

const WINDOW_WIDTH: i32 = 560;
const WINDOW_HEIGHT: i32 = 520;
const MIN_WINDOW_WIDTH: i32 = 520;
const MIN_WINDOW_HEIGHT: i32 = 480;
const FIELD_HEIGHT: i32 = 26;
const BORDER: i32 = 12;
const GAP: i32 = 8;

pub const ID_APPLY: i32 = ID_OK + 1000;

#[derive(Clone)]
pub struct SettingUI {
    pub frame: Frame,
    pub appearance_radio: RadioBox,
    pub ffmpeg_path_text: TextCtrl,
    pub ffmpeg_browse_button: Button,
    pub ffmpeg_validate_button: Button,
    pub ffmpeg_status_text: StaticText,
    pub video_encoder_choice: Choice,
    pub video_quality_choice: Choice,
    pub audio_encoder_choice: Choice,
    pub audio_bitrate_slider: Slider,
    pub audio_bitrate_label: StaticText,
    pub ok_button: Button,
    pub cancel_button: Button,
    pub apply_button: Button,
}

impl SettingUI {
    pub fn new(_parent: &Frame, system_supported: bool) -> Self {
        let frame = Frame::builder()
            .with_title("Settings")
            .with_size(Size::new(WINDOW_WIDTH, WINDOW_HEIGHT))
            .with_style(FrameStyle::Caption | FrameStyle::SystemMenu | FrameStyle::CloseBox)
            .build();
        frame.set_min_size(Size::new(MIN_WINDOW_WIDTH, MIN_WINDOW_HEIGHT));

        let panel = Panel::builder(&frame).build();
        let root_sizer = BoxSizer::builder(Orientation::Vertical).build();

        let appearance_sizer =
            StaticBoxSizerBuilder::new_with_label(Orientation::Vertical, &panel, "Appearance")
                .build();
        let appearance_choices = ["System", "Dark", "Light"];
        let appearance_radio = RadioBox::builder(&panel, &appearance_choices)
            .with_label("")
            .with_major_dimension(3)
            .with_style(RadioBoxStyle::SpecifyCols)
            .build();
        if !system_supported {
            appearance_radio.enable_item(0, false);
        }
        appearance_sizer.add(
            &appearance_radio,
            0,
            SizerFlag::Expand | SizerFlag::All,
            GAP,
        );
        appearance_sizer.add(
            &StaticText::builder(&panel)
                .with_label("Restart app to apply appearance changes.")
                .build(),
            0,
            SizerFlag::Expand | SizerFlag::Left | SizerFlag::Right | SizerFlag::Bottom,
            GAP,
        );
        root_sizer.add_sizer(
            &appearance_sizer,
            0,
            SizerFlag::Expand | SizerFlag::Left | SizerFlag::Right | SizerFlag::Top,
            BORDER,
        );

        let encoder_sizer =
            StaticBoxSizerBuilder::new_with_label(Orientation::Vertical, &panel, "Encoder").build();
        encoder_sizer.add(
            &StaticText::builder(&panel)
                .with_label("FFmpeg path")
                .build(),
            0,
            SizerFlag::Expand | SizerFlag::All,
            GAP,
        );
        let ffmpeg_row = BoxSizer::builder(Orientation::Horizontal).build();
        let ffmpeg_path_text = TextCtrl::builder(&panel)
            .with_size(Size::new(-1, FIELD_HEIGHT))
            .build();
        let ffmpeg_browse_button = Button::builder(&panel)
            .with_label("Browse")
            .with_size(Size::new(92, FIELD_HEIGHT))
            .build();
        let ffmpeg_validate_button = Button::builder(&panel)
            .with_label("Validate")
            .with_size(Size::new(92, FIELD_HEIGHT))
            .build();
        ffmpeg_row.add(
            &ffmpeg_path_text,
            1,
            SizerFlag::Expand | SizerFlag::Right,
            GAP,
        );
        ffmpeg_row.add(
            &ffmpeg_browse_button,
            0,
            SizerFlag::Expand | SizerFlag::Right,
            GAP,
        );
        ffmpeg_row.add(&ffmpeg_validate_button, 0, SizerFlag::Expand, 0);
        encoder_sizer.add_sizer(
            &ffmpeg_row,
            0,
            SizerFlag::Expand | SizerFlag::Left | SizerFlag::Right,
            GAP,
        );
        let ffmpeg_status_text = StaticText::builder(&panel)
            .with_label("Click Validate to check FFmpeg status and get available encoders.")
            .build();
        encoder_sizer.add(
            &ffmpeg_status_text,
            0,
            SizerFlag::Expand | SizerFlag::Left | SizerFlag::Right | SizerFlag::Top,
            GAP,
        );

        let quality_row = BoxSizer::builder(Orientation::Horizontal).build();
        let video_encoder_sizer = BoxSizer::builder(Orientation::Vertical).build();
        video_encoder_sizer.add(
            &StaticText::builder(&panel)
                .with_label("Video encoder")
                .build(),
            0,
            SizerFlag::Expand | SizerFlag::Bottom,
            4,
        );
        let video_encoder_choice = Choice::builder(&panel).build();
        video_encoder_sizer.add(&video_encoder_choice, 0, SizerFlag::Expand, 0);

        let video_quality_sizer = BoxSizer::builder(Orientation::Vertical).build();
        video_quality_sizer.add(
            &StaticText::builder(&panel)
                .with_label("Default video quality")
                .build(),
            0,
            SizerFlag::Expand | SizerFlag::Bottom,
            4,
        );
        let video_quality_choice = Choice::builder(&panel)
            .with_choices(video_quality_choices())
            .with_selection(Some(5))
            .build();
        video_quality_sizer.add(&video_quality_choice, 0, SizerFlag::Expand, 0);

        quality_row.add_sizer(
            &video_encoder_sizer,
            1,
            SizerFlag::Expand | SizerFlag::Right,
            GAP,
        );
        quality_row.add_sizer(
            &video_quality_sizer,
            1,
            SizerFlag::Expand | SizerFlag::Left,
            GAP,
        );
        encoder_sizer.add_sizer(&quality_row, 0, SizerFlag::Expand | SizerFlag::All, GAP);

        let audio_row = BoxSizer::builder(Orientation::Horizontal).build();
        let audio_encoder_sizer = BoxSizer::builder(Orientation::Vertical).build();
        audio_encoder_sizer.add(
            &StaticText::builder(&panel)
                .with_label("Default audio encoder")
                .build(),
            0,
            SizerFlag::Expand | SizerFlag::Bottom,
            4,
        );
        let audio_encoder_choice = Choice::builder(&panel)
            .with_choices(audio_encoder_choices())
            .with_selection(Some(1))
            .build();
        audio_encoder_sizer.add(&audio_encoder_choice, 0, SizerFlag::Expand, 0);

        let audio_bitrate_sizer = BoxSizer::builder(Orientation::Vertical).build();
        let audio_bitrate_label = StaticText::builder(&panel)
            .with_label("Default audio bitrate: 320kbps")
            .build();
        let audio_bitrate_slider = Slider::builder(&panel)
            .with_min_value(64)
            .with_max_value(512)
            .with_value(320)
            .with_style(SliderStyle::Default | SliderStyle::MinMaxLabels)
            .build();
        audio_bitrate_sizer.add(
            &audio_bitrate_label,
            0,
            SizerFlag::Expand | SizerFlag::Bottom,
            4,
        );
        audio_bitrate_sizer.add(&audio_bitrate_slider, 0, SizerFlag::Expand, 0);

        audio_row.add_sizer(
            &audio_encoder_sizer,
            1,
            SizerFlag::Expand | SizerFlag::Right,
            GAP,
        );
        audio_row.add_sizer(
            &audio_bitrate_sizer,
            1,
            SizerFlag::Expand | SizerFlag::Left,
            GAP,
        );
        encoder_sizer.add_sizer(
            &audio_row,
            0,
            SizerFlag::Expand | SizerFlag::Left | SizerFlag::Right | SizerFlag::Bottom,
            GAP,
        );
        root_sizer.add_sizer(
            &encoder_sizer,
            1,
            SizerFlag::Expand | SizerFlag::Left | SizerFlag::Right | SizerFlag::Top,
            BORDER,
        );

        let line = StaticLine::builder(&panel).build();
        root_sizer.add(
            &line,
            0,
            SizerFlag::Expand | SizerFlag::Left | SizerFlag::Right | SizerFlag::Top,
            BORDER,
        );

        let buttons_sizer = BoxSizer::builder(Orientation::Horizontal).build();
        buttons_sizer.add_stretch_spacer(1);
        let ok_button = Button::builder(&panel)
            .with_id(ID_OK)
            .with_label("OK")
            .build();
        let cancel_button = Button::builder(&panel)
            .with_id(ID_CANCEL)
            .with_label("Cancel")
            .build();
        let apply_button = Button::builder(&panel)
            .with_id(ID_APPLY)
            .with_label("Apply")
            .build();
        ok_button.set_default();
        buttons_sizer.add(&ok_button, 0, SizerFlag::Right, GAP);
        buttons_sizer.add(&cancel_button, 0, SizerFlag::Right, GAP);
        buttons_sizer.add(&apply_button, 0, SizerFlag::AlignCenterVertical, 0);
        root_sizer.add_sizer(
            &buttons_sizer,
            0,
            SizerFlag::Expand | SizerFlag::All,
            BORDER,
        );

        panel.set_sizer(root_sizer, true);
        frame.layout();
        frame.center_on_screen();

        Self {
            frame,
            appearance_radio,
            ffmpeg_path_text,
            ffmpeg_browse_button,
            ffmpeg_validate_button,
            ffmpeg_status_text,
            video_encoder_choice,
            video_quality_choice,
            audio_encoder_choice,
            audio_bitrate_slider,
            audio_bitrate_label,
            ok_button,
            cancel_button,
            apply_button,
        }
    }
}

fn video_quality_choices() -> Vec<String> {
    [
        "144p", "240p", "360p", "480p", "720p", "1080p", "1440p", "2160p", "4320p",
    ]
    .iter()
    .map(|value| value.to_string())
    .collect()
}

fn audio_encoder_choices() -> Vec<String> {
    ["opus", "aac", "original"]
        .iter()
        .map(|value| value.to_string())
        .collect()
}
