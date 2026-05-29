use crate::deps::ffmpeg;
use wxdragon::appearance::{
    AppAppearance, Appearance, AppearanceResult, get_app as get_appearance_app,
};
use wxdragon::geometry::Size;
use wxdragon::prelude::*;

wxdragon::include_xrc!("../../xrc/ui.xrc", FrameUI);

pub fn show() {
    if let Some(app) = get_appearance_app() {
        match app.set_appearance(Appearance::System) {
            AppearanceResult::Ok => {}
            AppearanceResult::Failure => {}
            AppearanceResult::CannotChange => {}
        }
    }

    let frame_ui = FrameUI::new(None, false);

    if let Some(app) = wxdragon::app::get_app_instance() {
        app.set_top_window(&frame_ui.MainFrame);
    }

    frame_ui.MainFrame.center_on_screen();
    frame_ui.MainFrame.show(true);
    frame_ui.MainFrame.set_min_size(Size::new(600, 500));
    frame_ui.MainFrame.set_size(Size::new(700, 600));
    frame_ui.MainFrame.layout();
    setup_status_bar(&frame_ui);

    check_ffmpeg_async();

    log::info!("Window loaded successfully!");
}

fn setup_status_bar(frame_ui: &FrameUI) {
    frame_ui.MainStatus.set_fields_count(2);
    frame_ui.MainStatus.set_status_widths(&[-1, 240]);
    frame_ui.MainStatus.set_status_text("Ready", 0);

    let progress_gauge = Gauge::builder(&frame_ui.MainStatus)
        .with_size(Size::new(220, 16))
        .with_range(100)
        .build();
    progress_gauge.set_value(0);
    position_status_progress(&frame_ui.MainStatus, &progress_gauge);

    let status_bar = frame_ui.MainStatus;
    frame_ui.MainFrame.on_size(move |event| {
        position_status_progress(&status_bar, &progress_gauge);
        event.skip(true);
    });

    wxdragon::call_after(Box::new(move || {
        position_status_progress(&status_bar, &progress_gauge);
    }));
}

fn position_status_progress(status_bar: &StatusBar, progress_gauge: &Gauge) {
    let status_size = status_bar.get_client_size();
    let gauge_width = 220.min(status_size.width.saturating_sub(24)).max(80);
    let gauge_height = (status_size.height - 6).max(12);
    let x = (status_size.width - gauge_width - 12).max(0);
    let y = ((status_size.height - gauge_height) / 2).max(0);

    progress_gauge.set_size_with_pos(x, y, gauge_width, gauge_height);
}

fn check_ffmpeg_async() {
    std::thread::spawn(move || {
        let result = ffmpeg::check_ffmpeg();

        wxdragon::call_after(Box::new(move || match result {
            3 => {}
            1 => show_ffmpeg_error(
                "FFmpeg is not installed or cannot be executed. Please install FFmpeg and make sure it's in your system PATH. You can change the FFmpeg path in the app settings if it's installed in a non-standard location.",
            ),
            2 => show_ffmpeg_error(
                "FFmpeg is installed but version is lower than 8.0. Please update FFmpeg to version 8.0 or higher.",
            ),
            _ => show_ffmpeg_error("Unexpected error while checking FFmpeg."),
        }));
    });
}

fn show_ffmpeg_error(message: &str) {
    log::error!("{message}");

    let Some(parent) = wxdragon::app::get_app_instance().and_then(|app| app.get_top_window())
    else {
        return;
    };

    let dialog = MessageDialog::builder(&parent, message, "FFmpeg check failed")
        .with_style(
            MessageDialogStyle::OK | MessageDialogStyle::IconWarning | MessageDialogStyle::Centre,
        )
        .build();

    dialog.show_modal();
}
