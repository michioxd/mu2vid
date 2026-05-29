use crate::ui::new_queue_ui::NewQueueUI;
use crate::ui::new_queue_ui::PREVIEW_SIZE;
use crate::ui::utils::{
    double_click_interval, format_file_size, is_cover_artwork, open_file_location,
};
use image::imageops::FilterType;
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

pub fn show(status_bar: StatusBar) {
    if focus_open_queue_window() {
        return;
    }

    let queue_ui = NewQueueUI::new();
    setup_events(&queue_ui, status_bar);
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

fn setup_events(queue_ui: &NewQueueUI, status_bar: StatusBar) {
    let load_generation = Arc::new(AtomicU64::new(0));
    let selected_artwork_path = Rc::new(RefCell::new(None::<PathBuf>));
    let last_artwork_click = Rc::new(RefCell::new(None::<Instant>));

    queue_ui.add_button.enable(false);

    let frame = queue_ui.frame;
    queue_ui.cancel_button.on_click(move |_| {
        frame.close(true);
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

fn is_audio_file(path: &Path) -> bool {
    path.extension()
        .and_then(|value| value.to_str())
        .map(|extension| {
            matches!(
                extension.to_ascii_lowercase().as_str(),
                "flac"
                    | "wav"
                    | "mp3"
                    | "m4a"
                    | "aac"
                    | "ogg"
                    | "opus"
                    | "wma"
                    | "alac"
                    | "aiff"
                    | "aif"
                    | "ape"
                    | "wv"
            )
        })
        .unwrap_or(false)
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
    let image = image::open(path).ok()?.to_rgba8();
    let (width, height) = image.dimensions();
    if width == 0 || height == 0 {
        return None;
    }

    let preview_size = PREVIEW_SIZE as u32;
    let scale = (preview_size as f32 / width as f32).max(preview_size as f32 / height as f32);
    let scaled_width = (width as f32 * scale).ceil() as u32;
    let scaled_height = (height as f32 * scale).ceil() as u32;
    let resized =
        image::imageops::resize(&image, scaled_width, scaled_height, FilterType::Lanczos3);
    let x = scaled_width.saturating_sub(preview_size) / 2;
    let y = scaled_height.saturating_sub(preview_size) / 2;
    let cropped = image::imageops::crop_imm(&resized, x, y, preview_size, preview_size).to_image();

    Some(cropped.into_raw())
}
