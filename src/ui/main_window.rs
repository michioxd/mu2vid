use crate::ui::about;
use crate::ui::main_window_ui::FrameUI;
use crate::{config, deps::ffmpeg};
use wxdragon::appearance::{
    AppAppearance, Appearance, AppearanceResult, get_app as get_appearance_app, is_system_dark_mode,
};
// use wxdragon::event::EventType;
use wxdragon::geometry::{Point, Rect, Size};
use wxdragon::id::{ID_ABOUT, ID_EXIT};
use wxdragon::prelude::*;

const DEFAULT_WINDOW_WIDTH: i32 = 950;
const DEFAULT_WINDOW_HEIGHT: i32 = 600;
const MIN_WINDOW_WIDTH: i32 = 600;
const MIN_WINDOW_HEIGHT: i32 = 500;
const FALLBACK_SCREEN_WIDTH: i32 = 1920;
const FALLBACK_SCREEN_HEIGHT: i32 = 1080;
const MIN_SPLITTER_SASH_POSITION: i32 = 240;

pub fn show() {
    apply_system_appearance();

    let frame_ui = FrameUI::new();

    if let Some(app) = wxdragon::app::get_app_instance() {
        app.set_top_window(&frame_ui.main_frame);
    }

    restore_window_state(&frame_ui);
    frame_ui.main_frame.show(true);
    frame_ui
        .main_frame
        .set_min_size(Size::new(MIN_WINDOW_WIDTH, MIN_WINDOW_HEIGHT));
    frame_ui.main_frame.layout();
    frame_ui.apply_colors(is_system_dark_mode());
    setup_status_bar(&frame_ui);
    setup_help_menu(&frame_ui);
    setup_window_state_persistence(&frame_ui);
    // setup_system_theme_watcher(&frame_ui);

    check_ffmpeg_async();

    log::info!("Window loaded successfully!");
}

fn apply_system_appearance() {
    let Some(app) = get_appearance_app() else {
        return;
    };

    match app.set_appearance(Appearance::System) {
        AppearanceResult::Ok | AppearanceResult::CannotChange => {}
        AppearanceResult::Failure => {
            log::warn!("System appearance is not supported on this platform")
        }
    }
}

// TODO: re-implement later when renderer supports dynamic theme changes
// fn setup_system_theme_watcher(frame_ui: &FrameUI) {
//     let frame_ui = frame_ui.clone();
//     let mut last_dark_mode = is_system_dark_mode();

//     let main_frame = frame_ui.main_frame;
//     main_frame
//         // TODO: implement wxSYS_COLOUR_WINDOW change event after they add it to wxDragon
//         .bind_internal(EventType::ANY, move |event| {
//             let dark_mode = is_system_dark_mode();

//             if dark_mode != last_dark_mode {
//                 last_dark_mode = dark_mode;
//                 apply_system_appearance();
//                 frame_ui.apply_colors(dark_mode);
//             }

//             event.skip(true);
//         });
// }

fn restore_window_state(frame_ui: &FrameUI) {
    let config = config::load();

    match saved_window_rect(&config) {
        Some(rect) => {
            let rect = clamp_window_rect(rect, screen_rect());
            frame_ui
                .main_frame
                .set_size_with_pos(rect.x, rect.y, rect.width, rect.height);
        }
        None => {
            frame_ui
                .main_frame
                .set_size(Size::new(DEFAULT_WINDOW_WIDTH, DEFAULT_WINDOW_HEIGHT));
            frame_ui.main_frame.center_on_screen();
        }
    }

    if let Some(sash_position) = config.window.main_splitter_sash_position {
        frame_ui
            .main_splitter
            .set_sash_position(sash_position.max(MIN_SPLITTER_SASH_POSITION), true);
    }

    if config.window.maximized {
        frame_ui.main_frame.maximize(true);
    }
}

fn saved_window_rect(config: &config::AppConfig) -> Option<Rect> {
    let window = &config.window;

    Some(Rect::new(
        window.x?,
        window.y?,
        window.width?,
        window.height?,
    ))
}

fn clamp_window_rect(rect: Rect, screen: Rect) -> Rect {
    let max_width = screen.width.max(MIN_WINDOW_WIDTH);
    let max_height = screen.height.max(MIN_WINDOW_HEIGHT);
    let width = rect.width.clamp(MIN_WINDOW_WIDTH, max_width);
    let height = rect.height.clamp(MIN_WINDOW_HEIGHT, max_height);
    let min_x = screen.x;
    let min_y = screen.y;
    let max_x = screen.x + screen.width - width;
    let max_y = screen.y + screen.height - height;

    Rect::new(
        rect.x.clamp(min_x, max_x.max(min_x)),
        rect.y.clamp(min_y, max_y.max(min_y)),
        width,
        height,
    )
}

fn screen_rect() -> Rect {
    #[cfg(target_os = "windows")]
    {
        if let Some(rect) = windows_virtual_screen_rect() {
            return rect;
        }
    }

    Rect::new(0, 0, FALLBACK_SCREEN_WIDTH, FALLBACK_SCREEN_HEIGHT)
}

#[cfg(target_os = "windows")]
fn windows_virtual_screen_rect() -> Option<Rect> {
    const SM_XVIRTUALSCREEN: i32 = 76;
    const SM_YVIRTUALSCREEN: i32 = 77;
    const SM_CXVIRTUALSCREEN: i32 = 78;
    const SM_CYVIRTUALSCREEN: i32 = 79;

    unsafe extern "system" {
        fn GetSystemMetrics(n_index: i32) -> i32;
    }

    let width = unsafe { GetSystemMetrics(SM_CXVIRTUALSCREEN) };
    let height = unsafe { GetSystemMetrics(SM_CYVIRTUALSCREEN) };

    if width <= 0 || height <= 0 {
        return None;
    }

    Some(Rect::new(
        unsafe { GetSystemMetrics(SM_XVIRTUALSCREEN) },
        unsafe { GetSystemMetrics(SM_YVIRTUALSCREEN) },
        width,
        height,
    ))
}

fn setup_window_state_persistence(frame_ui: &FrameUI) {
    let frame = frame_ui.main_frame;
    let splitter = frame_ui.main_splitter;

    frame_ui.main_frame.on_move_event(move |event| {
        save_window_state(&frame, &splitter);
        event.skip(true);
    });

    let frame = frame_ui.main_frame;
    let splitter = frame_ui.main_splitter;
    frame_ui.main_frame.on_size(move |event| {
        save_window_state(&frame, &splitter);
        event.skip(true);
    });

    let frame = frame_ui.main_frame;
    let splitter = frame_ui.main_splitter;
    frame_ui
        .main_splitter
        .on_sash_position_changed(move |event| {
            save_window_state(&frame, &splitter);
            event.skip(true);
        });

    let frame = frame_ui.main_frame;
    let splitter = frame_ui.main_splitter;
    frame_ui.main_frame.on_close(move |event| {
        save_window_state(&frame, &splitter);
        event.skip(true);
    });
}

fn save_window_state(frame: &Frame, splitter: &SplitterWindow) {
    if !frame.is_valid() {
        return;
    }

    let position = frame.get_position();
    let size = frame.get_size();
    let maximized = frame.is_maximized();

    if frame.is_iconized() {
        return;
    }

    let mut config = config::load();
    config.window.maximized = maximized;

    if !maximized && is_valid_window_state(position, size) {
        config.window.x = Some(position.x);
        config.window.y = Some(position.y);
        config.window.width = Some(size.width);
        config.window.height = Some(size.height);
    }

    if splitter.is_valid() {
        config.window.main_splitter_sash_position = Some(splitter.sash_position());
    }

    if let Err(err) = config::save(&config) {
        log::warn!("Failed to save window state: {err}");
    }
}

fn is_valid_window_state(position: Point, size: Size) -> bool {
    position.x > -32000
        && position.y > -32000
        && size.width >= MIN_WINDOW_WIDTH
        && size.height >= MIN_WINDOW_HEIGHT
}

fn setup_help_menu(frame_ui: &FrameUI) {
    frame_ui.main_frame.on_menu_selected(move |event| {
        if event.get_id() == ID_ABOUT {
            about::show();
        } else if event.get_id() == ID_EXIT {
            wxdragon::app::get_app_instance()
                .map(|app| app.exit_main_loop())
                .unwrap_or_default();
        } else {
            event.skip(true);
        }
    });
}

fn setup_status_bar(frame_ui: &FrameUI) {
    frame_ui.main_status.set_fields_count(2);
    frame_ui.main_status.set_status_widths(&[-1, 240]);
    frame_ui.main_status.set_status_text("Ready", 0);

    let progress_gauge = Gauge::builder(&frame_ui.main_status)
        .with_size(Size::new(220, 16))
        .with_range(100)
        .build();
    progress_gauge.set_value(0);
    position_status_progress(&frame_ui.main_status, &progress_gauge);

    let status_bar = frame_ui.main_status;
    frame_ui.main_frame.on_size(move |event| {
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
