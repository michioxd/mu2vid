use crate::config::{self, AppearanceConfig};
use crate::deps::ffmpeg;
use crate::ui::new_queue::{
    MAX_AUDIO_BITRATE_KBPS, MIN_AUDIO_BITRATE_KBPS, ORIGINAL_AUDIO_CODEC, clamp_audio_bitrate,
};
use crate::ui::setting_ui::SettingUI;
use std::cell::RefCell;
use std::path::PathBuf;
use std::rc::Rc;
use std::sync::{
    Arc,
    atomic::{AtomicBool, AtomicU64, Ordering},
};
use wxdragon::appearance::{Appearance, AppearanceResult, get_app as get_appearance_app};
use wxdragon::id::ID_OK;
use wxdragon::prelude::*;

thread_local! {
    static OPEN_SETTING_FRAME: RefCell<Option<Frame>> = const { RefCell::new(None) };
}

pub fn show(parent: &Frame, status_bar: StatusBar) {
    if focus_open_setting_window() {
        return;
    }

    let system_supported = system_appearance_supported();
    let setting_ui = SettingUI::new(parent, system_supported);
    setup_initial_values(&setting_ui, system_supported);
    setup_events(&setting_ui, status_bar);
    remember_setting_window(setting_ui.frame);
    setting_ui.frame.show(true);
    setting_ui.frame.set_focus();
}

pub fn apply_configured_appearance() -> bool {
    let config = config::load();
    let _ = apply_appearance(config.appearance);

    match config.appearance {
        AppearanceConfig::Dark => true,
        AppearanceConfig::Light => false,
        AppearanceConfig::System => wxdragon::appearance::is_system_dark_mode(),
    }
}

fn focus_open_setting_window() -> bool {
    OPEN_SETTING_FRAME.with(|cell| {
        let Some(frame) = *cell.borrow() else {
            return false;
        };

        if !frame.is_valid() {
            *cell.borrow_mut() = None;
            return false;
        }

        if frame.is_iconized() {
            frame.iconize(false);
        }
        frame.show(true);
        frame.set_focus();
        true
    })
}

fn remember_setting_window(frame: Frame) {
    OPEN_SETTING_FRAME.with(|cell| {
        *cell.borrow_mut() = Some(frame);
    });
}

fn setup_initial_values(setting_ui: &SettingUI, system_supported: bool) {
    let config = config::load();
    setting_ui
        .ffmpeg_path_text
        .set_value(config.ffmpeg_path.as_deref().unwrap_or("ffmpeg"));
    set_appearance_selection(
        setting_ui.appearance_radio,
        config.appearance,
        system_supported,
    );
    set_choice_to_value(
        &setting_ui.video_quality_choice,
        &config.encoder.default_video_quality,
    );
    set_choice_to_value(
        &setting_ui.audio_encoder_choice,
        &config.encoder.default_audio_encoder,
    );
    setting_ui
        .audio_bitrate_slider
        .set_value(clamp_audio_bitrate(config.encoder.default_audio_bitrate_kbps) as i32);
    update_audio_bitrate_label(
        setting_ui.audio_bitrate_slider,
        setting_ui.audio_bitrate_label,
    );

    setting_ui.video_encoder_choice.append(
        config
            .encoder
            .video_encoder
            .as_deref()
            .unwrap_or("Validate FFmpeg to load encoders"),
    );
    setting_ui.video_encoder_choice.set_selection(0);
    setting_ui.video_encoder_choice.enable(false);
    setting_ui.apply_button.enable(false);
}

fn setup_events(setting_ui: &SettingUI, status_bar: StatusBar) {
    let dirty = Rc::new(RefCell::new(false));
    let valid_ffmpeg = Arc::new(AtomicBool::new(false));
    let validate_generation = Arc::new(AtomicU64::new(0));

    let frame = setting_ui.frame;
    setting_ui.cancel_button.on_click(move |_| {
        frame.close(true);
    });

    let frame = setting_ui.frame;
    let ffmpeg_path_text = setting_ui.ffmpeg_path_text;
    setting_ui.ffmpeg_browse_button.on_click(move |_| {
        if let Some(path) = choose_ffmpeg_file(&frame, &ffmpeg_path_text.get_value()) {
            ffmpeg_path_text.set_value(&path.to_string_lossy());
        }
    });

    let ffmpeg_path_text = setting_ui.ffmpeg_path_text;
    let video_encoder_choice = setting_ui.video_encoder_choice;
    let ffmpeg_status_text = setting_ui.ffmpeg_status_text;
    let ffmpeg_validate_button = setting_ui.ffmpeg_validate_button;
    let valid_for_validate = Arc::clone(&valid_ffmpeg);
    let generation_for_validate = Arc::clone(&validate_generation);
    setting_ui.ffmpeg_validate_button.on_click(move |_| {
        let selected_encoder = video_encoder_choice.get_string_selection();
        validate_ffmpeg_controls(
            ffmpeg_path_text,
            video_encoder_choice,
            ffmpeg_status_text,
            ffmpeg_validate_button,
            status_bar,
            Arc::clone(&valid_for_validate),
            Arc::clone(&generation_for_validate),
            selected_encoder.as_deref(),
        );
    });

    let audio_bitrate_slider = setting_ui.audio_bitrate_slider;
    let audio_bitrate_label = setting_ui.audio_bitrate_label;
    let apply_button = setting_ui.apply_button;
    let dirty_for_slider = Rc::clone(&dirty);
    setting_ui.audio_bitrate_slider.on_slider(move |_| {
        update_audio_bitrate_label(audio_bitrate_slider, audio_bitrate_label);
        mark_dirty(&dirty_for_slider, apply_button);
    });

    bind_dirty_choice(
        setting_ui.video_encoder_choice,
        setting_ui.apply_button,
        Rc::clone(&dirty),
    );
    bind_dirty_choice(
        setting_ui.video_quality_choice,
        setting_ui.apply_button,
        Rc::clone(&dirty),
    );
    bind_dirty_choice(
        setting_ui.audio_encoder_choice,
        setting_ui.apply_button,
        Rc::clone(&dirty),
    );
    bind_dirty_radio(
        setting_ui.appearance_radio,
        setting_ui.apply_button,
        Rc::clone(&dirty),
    );

    let apply_ui = setting_ui.clone();
    let dirty_for_apply = Rc::clone(&dirty);
    let valid_for_apply = Arc::clone(&valid_ffmpeg);
    setting_ui.apply_button.on_click(move |_| {
        if apply_settings(&apply_ui, &valid_for_apply) {
            *dirty_for_apply.borrow_mut() = false;
            apply_ui.apply_button.enable(false);
            status_bar.set_status_text("Settings saved", 0);
        }
    });

    let ok_ui = setting_ui.clone();
    let valid_for_ok = Arc::clone(&valid_ffmpeg);
    let frame = setting_ui.frame;
    setting_ui.ok_button.on_click(move |_| {
        if apply_settings(&ok_ui, &valid_for_ok) {
            status_bar.set_status_text("Settings saved", 0);
            frame.close(true);
        }
    });

    let dirty_for_text = Rc::clone(&dirty);
    let apply_button = setting_ui.apply_button;
    let valid_for_text = Arc::clone(&valid_ffmpeg);
    let generation_for_text = Arc::clone(&validate_generation);
    let video_encoder_choice = setting_ui.video_encoder_choice;
    let ffmpeg_status_text = setting_ui.ffmpeg_status_text;
    let ffmpeg_validate_button = setting_ui.ffmpeg_validate_button;
    setting_ui.ffmpeg_path_text.on_text_updated(move |_| {
        valid_for_text.store(false, Ordering::SeqCst);
        generation_for_text.fetch_add(1, Ordering::SeqCst);
        ffmpeg_validate_button.enable(true);
        video_encoder_choice.clear();
        video_encoder_choice.append("Validate FFmpeg to load encoders");
        video_encoder_choice.set_selection(0);
        video_encoder_choice.enable(false);
        ffmpeg_status_text.set_label("FFmpeg path changed. Please validate again.");
        mark_dirty(&dirty_for_text, apply_button);
    });
}

fn bind_dirty_choice(choice: Choice, apply_button: Button, dirty: Rc<RefCell<bool>>) {
    choice.on_selection_changed(move |_| {
        mark_dirty(&dirty, apply_button);
    });
}

fn bind_dirty_radio(radio: RadioBox, apply_button: Button, dirty: Rc<RefCell<bool>>) {
    radio.on_selected(move |_| {
        mark_dirty(&dirty, apply_button);
    });
}

fn mark_dirty(dirty: &Rc<RefCell<bool>>, apply_button: Button) {
    *dirty.borrow_mut() = true;
    apply_button.enable(true);
}

fn validate_ffmpeg_controls(
    ffmpeg_path_text: TextCtrl,
    video_encoder_choice: Choice,
    ffmpeg_status_text: StaticText,
    ffmpeg_validate_button: Button,
    status_bar: StatusBar,
    valid_ffmpeg: Arc<AtomicBool>,
    validate_generation: Arc<AtomicU64>,
    selected_encoder: Option<&str>,
) {
    let ffmpeg_path = ffmpeg_path_text.get_value();
    let selected_encoder = selected_encoder.map(str::to_string);
    valid_ffmpeg.store(false, Ordering::SeqCst);
    let generation = validate_generation.fetch_add(1, Ordering::SeqCst) + 1;
    ffmpeg_validate_button.enable(false);
    video_encoder_choice.enable(false);
    ffmpeg_status_text.set_label("Validating FFmpeg...");
    status_bar.set_status_text("Validating FFmpeg...", 0);

    std::thread::spawn(move || {
        let result = validate_ffmpeg_path_and_encoders(&ffmpeg_path);

        wxdragon::call_after(Box::new(move || {
            if validate_generation.load(Ordering::SeqCst) != generation {
                return;
            }

            ffmpeg_validate_button.enable(true);
            apply_validate_result(
                video_encoder_choice,
                ffmpeg_status_text,
                status_bar,
                &valid_ffmpeg,
                result,
                selected_encoder.as_deref(),
            );
        }));
    });
}

fn validate_ffmpeg_path_and_encoders(ffmpeg_path: &str) -> FfmpegValidateResult {
    match ffmpeg::check_ffmpeg_path(ffmpeg_path) {
        3 => match ffmpeg::video_encoders(ffmpeg_path) {
            Ok(encoders) if !encoders.is_empty() => FfmpegValidateResult::Valid(encoders),
            Ok(_) => FfmpegValidateResult::NoVideoEncoders,
            Err(err) => FfmpegValidateResult::CannotFetchEncoders(err.to_string()),
        },
        2 => FfmpegValidateResult::VersionTooOld,
        _ => FfmpegValidateResult::NotWorking,
    }
}

fn apply_validate_result(
    video_encoder_choice: Choice,
    ffmpeg_status_text: StaticText,
    status_bar: StatusBar,
    valid_ffmpeg: &Arc<AtomicBool>,
    result: FfmpegValidateResult,
    selected_encoder: Option<&str>,
) {
    match result {
        FfmpegValidateResult::Valid(encoders) => {
            populate_video_encoders(video_encoder_choice, &encoders, selected_encoder);
            valid_ffmpeg.store(true, Ordering::SeqCst);
            ffmpeg_status_text.set_label("FFmpeg is valid.");
            status_bar.set_status_text("FFmpeg is valid", 0);
        }
        FfmpegValidateResult::NoVideoEncoders => {
            valid_ffmpeg.store(false, Ordering::SeqCst);
            video_encoder_choice.clear();
            video_encoder_choice.append("No video encoders found");
            video_encoder_choice.set_selection(0);
            video_encoder_choice.enable(false);
            ffmpeg_status_text.set_label("FFmpeg valid but no video encoders found.");
            status_bar.set_status_text("No video encoders found", 0);
        }
        FfmpegValidateResult::CannotFetchEncoders(err) => {
            valid_ffmpeg.store(false, Ordering::SeqCst);
            ffmpeg_status_text.set_label(&format!("Cannot fetch encoders: {err}"));
            status_bar.set_status_text("Cannot fetch FFmpeg encoders", 0);
        }
        FfmpegValidateResult::VersionTooOld => {
            valid_ffmpeg.store(false, Ordering::SeqCst);
            ffmpeg_status_text.set_label("FFmpeg version must be >= 8.0.");
            status_bar.set_status_text("FFmpeg version is too old", 0);
        }
        FfmpegValidateResult::NotWorking => {
            valid_ffmpeg.store(false, Ordering::SeqCst);
            ffmpeg_status_text.set_label("FFmpeg is not working.");
            status_bar.set_status_text("FFmpeg validation failed", 0);
        }
    }
}

enum FfmpegValidateResult {
    Valid(Vec<String>),
    NoVideoEncoders,
    CannotFetchEncoders(String),
    VersionTooOld,
    NotWorking,
}

fn populate_video_encoders(choice: Choice, encoders: &[String], selected: Option<&str>) {
    choice.clear();
    for encoder in encoders {
        choice.append(encoder);
    }
    choice.enable(true);

    if let Some(selected) = selected {
        set_choice_to_value(&choice, selected);
    }

    if choice.get_selection().is_none() && choice.get_count() > 0 {
        choice.set_selection(0);
    }
}

fn apply_settings(setting_ui: &SettingUI, valid_ffmpeg: &Arc<AtomicBool>) -> bool {
    let mut config = config::load();
    let ffmpeg_path = setting_ui.ffmpeg_path_text.get_value();

    config.ffmpeg_path = normalized_saved_ffmpeg_path(&ffmpeg_path);
    config.appearance = selected_appearance(setting_ui.appearance_radio);
    if valid_ffmpeg.load(Ordering::SeqCst) {
        config.encoder.video_encoder = setting_ui.video_encoder_choice.get_string_selection();
    }
    config.encoder.default_video_quality = setting_ui
        .video_quality_choice
        .get_string_selection()
        .unwrap_or_else(|| "1080p".to_string());
    config.encoder.default_audio_encoder = setting_ui
        .audio_encoder_choice
        .get_string_selection()
        .unwrap_or_else(|| ORIGINAL_AUDIO_CODEC.to_string());
    config.encoder.default_audio_bitrate_kbps =
        clamp_audio_bitrate(setting_ui.audio_bitrate_slider.get_value() as u32);

    match config::save(&config) {
        Ok(()) => true,
        Err(err) => {
            show_settings_error(
                &setting_ui.frame,
                &format!("Failed to save settings.\n\n{err}"),
            );
            false
        }
    }
}

fn normalized_saved_ffmpeg_path(path: &str) -> Option<String> {
    let path = path.trim();

    if path.is_empty() || path.eq_ignore_ascii_case("ffmpeg") {
        None
    } else {
        Some(path.to_string())
    }
}

fn selected_appearance(radio: RadioBox) -> AppearanceConfig {
    match radio.get_selection() {
        1 => AppearanceConfig::Dark,
        2 => AppearanceConfig::Light,
        _ => AppearanceConfig::System,
    }
}

fn set_appearance_selection(radio: RadioBox, appearance: AppearanceConfig, system_supported: bool) {
    let selection = match appearance {
        AppearanceConfig::System if system_supported => 0,
        AppearanceConfig::Dark => 1,
        AppearanceConfig::Light => 2,
        AppearanceConfig::System => 2,
    };
    radio.set_selection(selection);
}

fn apply_appearance(appearance: AppearanceConfig) -> Option<bool> {
    let app = get_appearance_app()?;
    let appearance = match appearance {
        AppearanceConfig::System => Appearance::System,
        AppearanceConfig::Dark => Appearance::Dark,
        AppearanceConfig::Light => Appearance::Light,
    };

    Some(matches!(
        app.set_appearance(appearance),
        AppearanceResult::Ok | AppearanceResult::CannotChange
    ))
}

fn system_appearance_supported() -> bool {
    apply_appearance(AppearanceConfig::System).unwrap_or(false)
}

fn update_audio_bitrate_label(slider: Slider, label: StaticText) {
    let bitrate = clamp_audio_bitrate(slider.get_value() as u32);
    slider.set_value(bitrate as i32);
    label.set_label(&format!("Default audio bitrate: {bitrate}kbps"));
}

fn set_choice_to_value(choice: &Choice, value: &str) {
    for index in 0..choice.get_count() {
        if choice.get_string(index).as_deref() == Some(value) {
            choice.set_selection(index);
            return;
        }
    }
}

fn choose_ffmpeg_file(parent: &Frame, current_path: &str) -> Option<PathBuf> {
    let current_path = PathBuf::from(current_path);
    let default_dir = current_path
        .parent()
        .map(|path| path.to_string_lossy().to_string())
        .unwrap_or_default();
    let dialog = FileDialog::builder(parent)
        .with_message("Choose FFmpeg executable")
        .with_default_dir(&default_dir)
        .with_wildcard("Executable files (*.exe)|*.exe|All files (*.*)|*.*")
        .with_style(FileDialogStyle::Open | FileDialogStyle::FileMustExist)
        .build();

    if dialog.show_modal() == ID_OK {
        dialog.get_path().map(PathBuf::from)
    } else {
        None
    }
}

fn show_settings_error(parent: &Frame, message: &str) {
    let dialog = MessageDialog::builder(parent, message, "Settings")
        .with_style(
            MessageDialogStyle::OK | MessageDialogStyle::IconWarning | MessageDialogStyle::Centre,
        )
        .build();

    dialog.show_modal();
}

#[allow(dead_code)]
fn _keep_limits_referenced() -> (u32, u32) {
    (MIN_AUDIO_BITRATE_KBPS, MAX_AUDIO_BITRATE_KBPS)
}
