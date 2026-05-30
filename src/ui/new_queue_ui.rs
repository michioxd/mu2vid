use wxdragon::geometry::Size;
use wxdragon::id::{ID_CANCEL, ID_OK};
use wxdragon::prelude::*;

const WINDOW_WIDTH: i32 = 450;
const WINDOW_HEIGHT: i32 = 660;
const MIN_WINDOW_WIDTH: i32 = 420;
const MIN_WINDOW_HEIGHT: i32 = 620;
const FIELD_HEIGHT: i32 = 26;
pub const PREVIEW_SIZE: i32 = 180;
const BORDER: i32 = 12;
const GAP: i32 = 8;

#[derive(Clone)]
#[allow(dead_code)]
pub struct NewQueueUI {
    pub frame: Frame,
    pub album_path_text: TextCtrl,
    pub browse_button: Button,
    pub artwork_preview_panel: Panel,
    pub artwork_preview_bitmap: StaticBitmap,
    pub artwork_preview_text: StaticText,
    pub artwork_info_text: StaticText,
    pub select_artwork_button: Button,
    pub title_text: TextCtrl,
    pub description_text: TextCtrl,
    pub video_quality_choice: Choice,
    pub audio_quality_choice: Choice,
    pub add_button: Button,
    pub cancel_button: Button,
}

impl NewQueueUI {
    pub fn new() -> Self {
        let frame = Frame::builder()
            .with_title("New Queue")
            .with_size(Size::new(WINDOW_WIDTH, WINDOW_HEIGHT))
            .with_style(FrameStyle::Caption | FrameStyle::SystemMenu | FrameStyle::CloseBox)
            .build();
        frame.set_min_size(Size::new(MIN_WINDOW_WIDTH, MIN_WINDOW_HEIGHT));

        let panel = Panel::builder(&frame).build();
        let root_sizer = BoxSizer::builder(Orientation::Vertical).build();

        let album_label = StaticText::builder(&panel)
            .with_label("Path to album (contains .flac, .wav, .mp3,...)")
            .build();
        root_sizer.add(
            &album_label,
            0,
            SizerFlag::Expand | SizerFlag::Left | SizerFlag::Right | SizerFlag::Top,
            BORDER,
        );

        let album_path_sizer = BoxSizer::builder(Orientation::Horizontal).build();
        let album_path_text = TextCtrl::builder(&panel)
            .with_size(Size::new(-1, FIELD_HEIGHT))
            .build();
        let browse_button = Button::builder(&panel)
            .with_label("Browse")
            .with_size(Size::new(92, FIELD_HEIGHT))
            .build();
        album_path_sizer.add(
            &album_path_text,
            1,
            SizerFlag::Expand | SizerFlag::Right,
            GAP,
        );
        album_path_sizer.add(&browse_button, 0, SizerFlag::Expand, 0);
        root_sizer.add_sizer(
            &album_path_sizer,
            0,
            SizerFlag::Expand | SizerFlag::Left | SizerFlag::Right | SizerFlag::Top,
            BORDER,
        );

        let artwork_label = StaticText::builder(&panel).with_label("Artwork").build();
        root_sizer.add(
            &artwork_label,
            0,
            SizerFlag::Expand | SizerFlag::Left | SizerFlag::Right | SizerFlag::Top,
            BORDER,
        );

        let artwork_preview_panel = Panel::builder(&panel)
            .with_size(Size::new(PREVIEW_SIZE, PREVIEW_SIZE))
            .with_style(PanelStyle::BorderSimple | PanelStyle::TabTraversal)
            .build();
        let preview_sizer = BoxSizer::builder(Orientation::Vertical).build();
        preview_sizer.add_stretch_spacer(1);
        let artwork_preview_bitmap = StaticBitmap::builder(&artwork_preview_panel)
            .with_bitmap(Some(
                Bitmap::new(PREVIEW_SIZE, PREVIEW_SIZE).unwrap_or_else(Bitmap::null_bitmap),
            ))
            .with_size(Size::new(PREVIEW_SIZE, PREVIEW_SIZE))
            .with_scale_mode(Some(ScaleMode::AspectFill))
            .build();
        let artwork_preview_text = StaticText::builder(&artwork_preview_panel)
            .with_label("Artwork preview")
            .build();
        preview_sizer.add(&artwork_preview_bitmap, 0, SizerFlag::AlignCentre, 0);
        preview_sizer.add(
            &artwork_preview_text,
            0,
            SizerFlag::AlignCentre | SizerFlag::Top,
            GAP,
        );
        preview_sizer.add_stretch_spacer(1);
        artwork_preview_panel.set_sizer(preview_sizer, true);
        let artwork_preview_row_sizer = BoxSizer::builder(Orientation::Horizontal).build();
        artwork_preview_row_sizer.add(&artwork_preview_panel, 0, SizerFlag::Right, BORDER);
        let artwork_info_text = StaticText::builder(&panel)
            .with_label("No artwork selected")
            .build();
        artwork_preview_row_sizer.add(&artwork_info_text, 1, SizerFlag::Expand, 0);
        root_sizer.add_sizer(
            &artwork_preview_row_sizer,
            0,
            SizerFlag::Expand | SizerFlag::Left | SizerFlag::Right | SizerFlag::Top,
            BORDER,
        );

        let select_artwork_button = Button::builder(&panel).with_label("Select artwork").build();
        root_sizer.add(
            &select_artwork_button,
            0,
            SizerFlag::AlignRight | SizerFlag::Right | SizerFlag::Top,
            BORDER,
        );

        let title_label = StaticText::builder(&panel).with_label("Title").build();
        root_sizer.add(
            &title_label,
            0,
            SizerFlag::Expand | SizerFlag::Left | SizerFlag::Right | SizerFlag::Top,
            BORDER,
        );
        let title_text = TextCtrl::builder(&panel)
            .with_size(Size::new(-1, FIELD_HEIGHT))
            .build();
        root_sizer.add(
            &title_text,
            0,
            SizerFlag::Expand | SizerFlag::Left | SizerFlag::Right | SizerFlag::Top,
            BORDER,
        );

        let description_label = StaticText::builder(&panel)
            .with_label("Description")
            .build();
        root_sizer.add(
            &description_label,
            0,
            SizerFlag::Expand | SizerFlag::Left | SizerFlag::Right | SizerFlag::Top,
            BORDER,
        );
        let description_text = TextCtrl::builder(&panel)
            .with_size(Size::new(-1, 130))
            .with_style(TextCtrlStyle::MultiLine | TextCtrlStyle::WordWrap)
            .with_value(
                "\n\n{{timestamp}}\n\nuploaded using mu2vid\nhttps://github.com/michioxd/mu2vid",
            )
            .build();
        root_sizer.add(
            &description_text,
            1,
            SizerFlag::Expand | SizerFlag::Left | SizerFlag::Right | SizerFlag::Top,
            BORDER,
        );

        let quality_sizer = BoxSizer::builder(Orientation::Horizontal).build();
        let video_sizer = BoxSizer::builder(Orientation::Vertical).build();
        let video_quality_label = StaticText::builder(&panel)
            .with_label("Video quality")
            .build();
        let video_quality_choice = Choice::builder(&panel)
            .with_choices(video_quality_choices())
            .with_selection(Some(5))
            .build();
        video_sizer.add(
            &video_quality_label,
            0,
            SizerFlag::Expand | SizerFlag::Bottom,
            4,
        );
        video_sizer.add(&video_quality_choice, 0, SizerFlag::Expand, 0);

        let audio_sizer = BoxSizer::builder(Orientation::Vertical).build();
        let audio_quality_label = StaticText::builder(&panel)
            .with_label("Audio quality")
            .build();
        let audio_quality_choice = Choice::builder(&panel)
            .with_choices(audio_quality_choices())
            .with_selection(Some(6))
            .build();
        audio_sizer.add(
            &audio_quality_label,
            0,
            SizerFlag::Expand | SizerFlag::Bottom,
            4,
        );
        audio_sizer.add(&audio_quality_choice, 0, SizerFlag::Expand, 0);

        quality_sizer.add_sizer(&video_sizer, 1, SizerFlag::Expand | SizerFlag::Right, GAP);
        quality_sizer.add_sizer(&audio_sizer, 1, SizerFlag::Expand | SizerFlag::Left, GAP);
        root_sizer.add_sizer(
            &quality_sizer,
            0,
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
        let add_button = Button::builder(&panel)
            .with_id(ID_OK)
            .with_label("Add queue")
            .build();
        let cancel_button = Button::builder(&panel)
            .with_id(ID_CANCEL)
            .with_label("Cancel")
            .build();
        add_button.set_default();
        buttons_sizer.add(&add_button, 0, SizerFlag::Right, GAP);
        buttons_sizer.add(&cancel_button, 0, SizerFlag::AlignCenterVertical, 0);
        root_sizer.add_sizer(
            &buttons_sizer,
            0,
            SizerFlag::Expand | SizerFlag::All,
            BORDER,
        );

        panel.set_sizer(root_sizer, true);
        frame.layout();
        frame.centre();

        Self {
            frame,
            album_path_text,
            browse_button,
            artwork_preview_panel,
            artwork_preview_bitmap,
            artwork_preview_text,
            artwork_info_text,
            select_artwork_button,
            title_text,
            description_text,
            video_quality_choice,
            audio_quality_choice,
            add_button,
            cancel_button,
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

fn audio_quality_choices() -> Vec<String> {
    [
        "Original",
        "96kbps (aac)",
        "128kbps (aac)",
        "160kbps (aac)",
        "192kbps (aac)",
        "256kbps (aac)",
        "320kbps (aac)",
        "64kbps (opus)",
        "96kbps (opus)",
        "128kbps (opus)",
        "160kbps (opus)",
        "192kbps (opus)",
        "256kbps (opus)",
        "320kbps (opus)",
    ]
    .iter()
    .map(|value| value.to_string())
    .collect()
}
