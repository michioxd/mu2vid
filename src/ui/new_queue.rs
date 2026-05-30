use crate::media::artwork::make_square_cover_rgba;
use crate::ui::new_queue_ui::NewQueueUI;
use crate::ui::new_queue_ui::PREVIEW_SIZE;
use crate::ui::utils::{
    double_click_interval, format_file_size, is_audio_file, is_cover_artwork, open_file_location,
};
use std::cell::RefCell;
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::sync::{
    Arc,
    atomic::{AtomicU64, Ordering},
};
use std::time::Instant;
use wxdragon::id::ID_OK;
use wxdragon::prelude::*;

thread_local! {
    static OPEN_QUEUE_FRAME: RefCell<Option<Frame>> = const { RefCell::new(None) };
}

#[derive(Clone)]
pub struct QueueItemDraft {
    pub album_path: String,
    pub artwork_path: PathBuf,
    pub title: String,
    pub video_quality: String,
    pub audio_codec: String,
    pub audio_bitrate_kbps: u32,
}

pub const ORIGINAL_AUDIO_CODEC: &str = "original";
pub const DEFAULT_AUDIO_CODEC: &str = "aac";
pub const DEFAULT_AUDIO_BITRATE_KBPS: u32 = 320;
pub const MIN_AUDIO_BITRATE_KBPS: u32 = 64;
pub const MAX_AUDIO_BITRATE_KBPS: u32 = 512;

impl QueueItemDraft {
    pub fn audio_display_label(&self) -> String {
        if self.audio_codec == ORIGINAL_AUDIO_CODEC {
            ORIGINAL_AUDIO_CODEC.to_string()
        } else {
            format!("{} {}kbps", self.audio_codec, self.audio_bitrate_kbps)
        }
    }

    #[allow(dead_code)]
    pub fn audio_ffmpeg_args(&self) -> Vec<String> {
        if self.audio_codec == ORIGINAL_AUDIO_CODEC {
            vec!["-c:a".to_string(), "copy".to_string()]
        } else {
            vec![
                "-c:a".to_string(),
                self.audio_codec.clone(),
                "-b:a".to_string(),
                format!("{}k", self.audio_bitrate_kbps),
            ]
        }
    }
}

pub fn show(status_bar: StatusBar, on_add: Rc<dyn Fn(QueueItemDraft)>) {
    show_with_initial(status_bar, None, "Add queue", on_add);
}

pub fn show_edit(status_bar: StatusBar, item: QueueItemDraft, on_save: Rc<dyn Fn(QueueItemDraft)>) {
    show_with_initial(status_bar, Some(item), "Save", on_save);
}

fn show_with_initial(
    status_bar: StatusBar,
    initial_item: Option<QueueItemDraft>,
    action_label: &str,
    on_submit: Rc<dyn Fn(QueueItemDraft)>,
) {
    if focus_open_queue_window() {
        return;
    }

    let queue_ui = NewQueueUI::new();
    setup_initial_values(&queue_ui, status_bar, initial_item.clone(), action_label);
    setup_events(&queue_ui, status_bar, on_submit, initial_item);
    remember_queue_window(queue_ui.frame);
    queue_ui.frame.show(true);
    queue_ui.frame.set_focus();
}

fn focus_open_queue_window() -> bool {
    OPEN_QUEUE_FRAME.with(|cell| {
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

fn remember_queue_window(frame: Frame) {
    OPEN_QUEUE_FRAME.with(|cell| {
        *cell.borrow_mut() = Some(frame);
    });
}

fn setup_initial_values(
    queue_ui: &NewQueueUI,
    status_bar: StatusBar,
    initial_item: Option<QueueItemDraft>,
    action_label: &str,
) {
    queue_ui.add_button.set_label(action_label);

    let Some(item) = initial_item else {
        return;
    };

    queue_ui.album_path_text.set_value(&item.album_path);
    queue_ui.title_text.set_value(&item.title);
    set_choice_to_value(&queue_ui.video_quality_choice, &item.video_quality);
    set_choice_to_value(&queue_ui.audio_codec_choice, &item.audio_codec);
    queue_ui
        .audio_bitrate_slider
        .set_value(clamp_audio_bitrate(item.audio_bitrate_kbps) as i32);
    update_audio_bitrate_controls(
        queue_ui.audio_codec_choice,
        queue_ui.audio_bitrate_slider,
        queue_ui.audio_bitrate_label,
    );
    update_artwork_info(&queue_ui.artwork_info_text, &item.artwork_path);
    load_artwork_preview_async(
        queue_ui.artwork_preview_bitmap,
        queue_ui.artwork_preview_text,
        status_bar,
        item.artwork_path,
        Arc::new(AtomicU64::new(0)),
    );
}

fn setup_events(
    queue_ui: &NewQueueUI,
    status_bar: StatusBar,
    on_submit: Rc<dyn Fn(QueueItemDraft)>,
    initial_item: Option<QueueItemDraft>,
) {
    let load_generation = Arc::new(AtomicU64::new(0));
    let selected_artwork_path = Rc::new(RefCell::new(
        initial_item.as_ref().map(|item| item.artwork_path.clone()),
    ));
    let last_artwork_click = Rc::new(RefCell::new(None::<Instant>));

    update_create_button(
        queue_ui.add_button,
        queue_ui.album_path_text,
        queue_ui.title_text,
        &selected_artwork_path,
    );

    let frame = queue_ui.frame;
    queue_ui.cancel_button.on_click(move |_| {
        frame.close(true);
    });

    let frame = queue_ui.frame;
    let album_path_text = queue_ui.album_path_text;
    let title_text = queue_ui.title_text;
    let video_quality_choice = queue_ui.video_quality_choice;
    let audio_codec_choice = queue_ui.audio_codec_choice;
    let audio_bitrate_slider = queue_ui.audio_bitrate_slider;
    let selected_artwork = Rc::clone(&selected_artwork_path);
    queue_ui.add_button.on_click(move |_| {
        let Some(artwork_path) = selected_artwork.borrow().clone() else {
            return;
        };

        let album_path = album_path_text.get_value();
        let title = title_text.get_value().trim().to_string();
        if album_path.trim().is_empty() || title.is_empty() {
            return;
        }

        on_submit(QueueItemDraft {
            album_path,
            artwork_path,
            title,
            video_quality: video_quality_choice
                .get_string_selection()
                .unwrap_or_else(|| "1080p".to_string()),
            audio_codec: audio_codec_choice
                .get_string_selection()
                .unwrap_or_else(|| DEFAULT_AUDIO_CODEC.to_string()),
            audio_bitrate_kbps: clamp_audio_bitrate(audio_bitrate_slider.get_value() as u32),
        });
        frame.close(true);
    });

    update_audio_bitrate_controls(
        queue_ui.audio_codec_choice,
        queue_ui.audio_bitrate_slider,
        queue_ui.audio_bitrate_label,
    );

    let audio_codec_choice = queue_ui.audio_codec_choice;
    let audio_bitrate_slider = queue_ui.audio_bitrate_slider;
    let audio_bitrate_label = queue_ui.audio_bitrate_label;
    queue_ui.audio_codec_choice.on_selection_changed(move |_| {
        update_audio_bitrate_controls(
            audio_codec_choice,
            audio_bitrate_slider,
            audio_bitrate_label,
        );
    });

    let audio_codec_choice = queue_ui.audio_codec_choice;
    let audio_bitrate_slider = queue_ui.audio_bitrate_slider;
    let audio_bitrate_label = queue_ui.audio_bitrate_label;
    queue_ui.audio_bitrate_slider.on_slider(move |_| {
        update_audio_bitrate_controls(
            audio_codec_choice,
            audio_bitrate_slider,
            audio_bitrate_label,
        );
    });

    let add_button = queue_ui.add_button;
    let album_path_text = queue_ui.album_path_text;
    let title_text = queue_ui.title_text;
    let selected_artwork = Rc::clone(&selected_artwork_path);
    queue_ui.album_path_text.on_text_updated(move |_| {
        update_create_button(add_button, album_path_text, title_text, &selected_artwork);
    });

    let add_button = queue_ui.add_button;
    let album_path_text = queue_ui.album_path_text;
    let title_text = queue_ui.title_text;
    let selected_artwork = Rc::clone(&selected_artwork_path);
    queue_ui.title_text.on_text_updated(move |_| {
        update_create_button(add_button, album_path_text, title_text, &selected_artwork);
    });

    let selected_artwork = Rc::clone(&selected_artwork_path);
    let last_click = Rc::clone(&last_artwork_click);
    queue_ui
        .artwork_preview_panel
        .on_mouse_left_down(move |event| {
            event.skip(false);
            open_artwork_location_on_double_click(&selected_artwork, &last_click);
        });

    let selected_artwork = Rc::clone(&selected_artwork_path);
    let last_click = Rc::clone(&last_artwork_click);
    queue_ui
        .artwork_preview_bitmap
        .on_mouse_left_down(move |event| {
            event.skip(false);
            open_artwork_location_on_double_click(&selected_artwork, &last_click);
        });

    let frame = queue_ui.frame;
    let album_path_text = queue_ui.album_path_text;
    let artwork_preview_bitmap = queue_ui.artwork_preview_bitmap;
    let artwork_preview_text = queue_ui.artwork_preview_text;
    let artwork_info_text = queue_ui.artwork_info_text;
    let add_button = queue_ui.add_button;
    let title_text = queue_ui.title_text;
    let browse_load_generation = Arc::clone(&load_generation);
    let selected_artwork = Rc::clone(&selected_artwork_path);
    queue_ui.browse_button.on_click(move |_| {
        if let Some(folder) = choose_album_folder(&frame, &album_path_text.get_value()) {
            if !contains_audio_file(&folder) {
                show_no_audio_files_message(&frame);
                return;
            }

            album_path_text.set_value(&folder.to_string_lossy());
            if let Some(folder_name) = folder.file_name().and_then(|value| value.to_str()) {
                title_text.set_value(folder_name);
            }
            if let Some(cover_path) = find_cover_artwork(&folder) {
                *selected_artwork.borrow_mut() = Some(cover_path.clone());
                update_artwork_info(&artwork_info_text, &cover_path);
                update_create_button(add_button, album_path_text, title_text, &selected_artwork);
                load_artwork_preview_async(
                    artwork_preview_bitmap,
                    artwork_preview_text,
                    status_bar,
                    cover_path,
                    Arc::clone(&browse_load_generation),
                );
            } else {
                *selected_artwork.borrow_mut() = None;
                artwork_preview_text.set_label("No cover artwork found");
                artwork_info_text.set_label("No artwork selected");
                status_bar.set_status_text("No cover artwork found", 0);
                update_create_button(add_button, album_path_text, title_text, &selected_artwork);
            }
        }
    });

    let frame = queue_ui.frame;

    fn show_no_audio_files_message(parent: &Frame) {
        let dialog = MessageDialog::builder(
            parent,
            "Sorry we couldn't find any audio files in the selected folder. Please choose a different folder that contains your music files. Currently supported audio formats are FLAC, WAV, MP3, M4A, AAC, OGG, OPUS, WMA, ALAC, AIFF, AIF, APE and WV.",
            "Folder is invalid",
        )
        .with_style(
            MessageDialogStyle::OK | MessageDialogStyle::IconWarning | MessageDialogStyle::Centre,
        )
        .build();

        dialog.show_modal();
    }
    let album_path_text = queue_ui.album_path_text;
    let artwork_preview_bitmap = queue_ui.artwork_preview_bitmap;
    let artwork_preview_text = queue_ui.artwork_preview_text;
    let artwork_info_text = queue_ui.artwork_info_text;
    let add_button = queue_ui.add_button;
    let title_text = queue_ui.title_text;
    let select_load_generation = Arc::clone(&load_generation);
    let selected_artwork = Rc::clone(&selected_artwork_path);
    queue_ui.select_artwork_button.on_click(move |_| {
        if let Some(artwork_path) = choose_artwork_file(&frame, &album_path_text.get_value()) {
            *selected_artwork.borrow_mut() = Some(artwork_path.clone());
            update_artwork_info(&artwork_info_text, &artwork_path);
            update_create_button(add_button, album_path_text, title_text, &selected_artwork);
            load_artwork_preview_async(
                artwork_preview_bitmap,
                artwork_preview_text,
                status_bar,
                artwork_path,
                Arc::clone(&select_load_generation),
            );
        }
    });
}

fn set_choice_to_value(choice: &Choice, value: &str) {
    for index in 0..choice.get_count() {
        if choice.get_string(index).as_deref() == Some(value) {
            choice.set_selection(index);
            return;
        }
    }
}

fn update_audio_bitrate_controls(
    codec_choice: Choice,
    bitrate_slider: Slider,
    bitrate_label: StaticText,
) {
    let codec = codec_choice
        .get_string_selection()
        .unwrap_or_else(|| DEFAULT_AUDIO_CODEC.to_string());
    let original_audio = codec == ORIGINAL_AUDIO_CODEC;
    bitrate_slider.enable(!original_audio);

    if original_audio {
        bitrate_label.set_label("Bitrate: disabled for original");
    } else {
        let bitrate = clamp_audio_bitrate(bitrate_slider.get_value() as u32);
        bitrate_slider.set_value(bitrate as i32);
        bitrate_label.set_label(&format!("Bitrate: {bitrate}kbps"));
    }
}

pub fn clamp_audio_bitrate(value: u32) -> u32 {
    value.clamp(MIN_AUDIO_BITRATE_KBPS, MAX_AUDIO_BITRATE_KBPS)
}

fn update_create_button(
    add_button: Button,
    album_path_text: TextCtrl,
    title_text: TextCtrl,
    selected_artwork_path: &Rc<RefCell<Option<PathBuf>>>,
) {
    let form_complete = !album_path_text.get_value().trim().is_empty()
        && !title_text.get_value().trim().is_empty()
        && selected_artwork_path.borrow().is_some();
    add_button.enable(form_complete);
}

fn open_artwork_location_on_double_click(
    selected_artwork_path: &Rc<RefCell<Option<PathBuf>>>,
    last_click: &Rc<RefCell<Option<Instant>>>,
) {
    if selected_artwork_path.borrow().is_none() {
        *last_click.borrow_mut() = None;
        return;
    }

    let now = Instant::now();
    let double_clicked = last_click
        .borrow()
        .map(|previous| now.duration_since(previous) <= double_click_interval())
        .unwrap_or(false);
    *last_click.borrow_mut() = Some(now);

    if double_clicked {
        if let Some(path) = selected_artwork_path.borrow().as_deref() {
            open_file_location(path);
        }
        *last_click.borrow_mut() = None;
    }
}

fn update_artwork_info(info_text: &StaticText, path: &Path) {
    let dimensions = image::image_dimensions(path)
        .map(|(width, height)| format!("{width} x {height}px"))
        .unwrap_or_else(|_| "Unknown dimensions".to_string());
    let size = std::fs::metadata(path)
        .map(|metadata| format_file_size(metadata.len()))
        .unwrap_or_else(|_| "Unknown size".to_string());
    let file_name = path
        .file_name()
        .map(|name| name.to_string_lossy().to_string())
        .unwrap_or_else(|| "Artwork".to_string());

    info_text.set_label(&format!("{file_name}\n{dimensions}\n{size}"));
}

fn choose_album_folder(parent: &Frame, current_path: &str) -> Option<PathBuf> {
    let dialog = DirDialog::builder(parent, "Choose album folder", current_path)
        .with_style((DirDialogStyle::Default | DirDialogStyle::MustExist).bits())
        .build();

    if dialog.show_modal() == ID_OK {
        dialog.get_path().map(PathBuf::from)
    } else {
        None
    }
}

fn choose_artwork_file(parent: &Frame, current_path: &str) -> Option<PathBuf> {
    let dialog = FileDialog::builder(parent)
        .with_message("Choose artwork")
        .with_default_dir(current_path)
        .with_wildcard("Image files (*.jpg;*.jpeg;*.png)|*.jpg;*.jpeg;*.png")
        .with_style(FileDialogStyle::Open | FileDialogStyle::FileMustExist)
        .build();

    if dialog.show_modal() == ID_OK {
        dialog.get_path().map(PathBuf::from)
    } else {
        None
    }
}

fn contains_audio_file(folder: &Path) -> bool {
    std::fs::read_dir(folder)
        .ok()
        .into_iter()
        .flatten()
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .any(|path| path.is_file() && is_audio_file(&path))
}

fn find_cover_artwork(folder: &Path) -> Option<PathBuf> {
    let mut candidates = std::fs::read_dir(folder)
        .ok()?
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| path.is_file() && is_cover_artwork(path))
        .collect::<Vec<_>>();

    candidates.sort_by_key(|path| {
        path.file_name()
            .map(|name| name.to_string_lossy().to_lowercase())
    });
    candidates.into_iter().next()
}

fn load_artwork_preview_async(
    preview: StaticBitmap,
    preview_text: StaticText,
    status_bar: StatusBar,
    path: PathBuf,
    load_generation: Arc<AtomicU64>,
) {
    let token = load_generation.fetch_add(1, Ordering::SeqCst) + 1;
    preview_text.set_label("Loading artwork...");
    status_bar.set_status_text("Loading artwork...", 0);

    std::thread::spawn(move || {
        let preview_rgba = make_cover_rgba(&path);

        wxdragon::call_after(Box::new(move || {
            if load_generation.load(Ordering::SeqCst) != token {
                return;
            }

            match preview_rgba
                .and_then(|rgba| Bitmap::from_rgba(&rgba, PREVIEW_SIZE as u32, PREVIEW_SIZE as u32))
            {
                Some(bitmap) => {
                    preview.set_bitmap(&bitmap);
                    preview_text.set_label("");
                    status_bar.set_status_text("Artwork loaded", 0);
                }
                None => {
                    preview_text.set_label("Cannot load artwork");
                    status_bar.set_status_text("Cannot load artwork", 0);
                }
            }
        }));
    });
}

fn make_cover_rgba(path: &Path) -> Option<Vec<u8>> {
    make_square_cover_rgba(path, PREVIEW_SIZE as u32)
}
