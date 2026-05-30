use image::GenericImageView;
use std::cell::RefCell;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::rc::Rc;
use wxdragon::geometry::Size;
use wxdragon::prelude::*;

pub const QUEUE_COVER_SIZE: i32 = 100;
const QUEUE_CARD_HEIGHT_ESTIMATE: i32 = QUEUE_COVER_SIZE + 20;
const QUEUE_LAZY_PRELOAD_MARGIN: i32 = QUEUE_CARD_HEIGHT_ESTIMATE * 3;

thread_local! {
    static QUEUE_ARTWORK_LOADER: RefCell<Option<Rc<QueueArtworkLoader>>> = const { RefCell::new(None) };
}

pub fn queue_cover_size() -> Size {
    Size::new(QUEUE_COVER_SIZE, QUEUE_COVER_SIZE)
}

pub fn queue_cover_placeholder() -> Bitmap {
    Bitmap::new(QUEUE_COVER_SIZE, QUEUE_COVER_SIZE).unwrap_or_else(Bitmap::null_bitmap)
}

pub fn register_queue_artwork(preview: StaticBitmap, artwork_path: impl Into<PathBuf>) {
    QUEUE_ARTWORK_LOADER.with(|loader| {
        if let Some(loader) = loader.borrow().as_ref() {
            loader.register(preview, artwork_path.into());
            loader.load_visible();
        }
    });
}

pub fn update_queue_artwork(preview: StaticBitmap, artwork_path: impl Into<PathBuf>) {
    let path = artwork_path.into();
    QUEUE_ARTWORK_LOADER.with(|loader| {
        if let Some(loader) = loader.borrow().as_ref() {
            loader.replace_or_register(preview, path);
        } else {
            load_queue_artwork_preview_async(preview, path);
        }
    });
}

pub fn unregister_queue_artwork(preview: StaticBitmap) {
    QUEUE_ARTWORK_LOADER.with(|loader| {
        if let Some(loader) = loader.borrow().as_ref() {
            loader.unregister(preview);
        }
    });
}

pub fn install_queue_artwork_loader(queue_list_panel: &ScrolledWindow) {
    let artwork_loader = QueueArtworkLoader::new(*queue_list_panel);
    QUEUE_ARTWORK_LOADER.with(|loader| {
        *loader.borrow_mut() = Some(artwork_loader.clone());
    });
    bind_queue_artwork_loader(queue_list_panel, artwork_loader);
}

struct QueueArtworkLoader {
    viewport: ScrolledWindow,
    items: RefCell<Vec<QueueArtworkItem>>,
    cache: RefCell<HashMap<PathBuf, Vec<u8>>>,
    next_index: RefCell<usize>,
    last_scroll_position: RefCell<i32>,
}

struct QueueArtworkItem {
    preview: StaticBitmap,
    path: PathBuf,
    state: QueueArtworkState,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum QueueArtworkState {
    Pending,
    Loading,
    Loaded,
}

impl QueueArtworkLoader {
    fn new(viewport: ScrolledWindow) -> Rc<Self> {
        Rc::new(Self {
            viewport,
            items: RefCell::new(Vec::new()),
            cache: RefCell::new(HashMap::new()),
            next_index: RefCell::new(0),
            last_scroll_position: RefCell::new(0),
        })
    }

    fn register(&self, preview: StaticBitmap, path: PathBuf) {
        let state = if self.set_cached_bitmap(preview, &path) {
            QueueArtworkState::Loaded
        } else {
            QueueArtworkState::Pending
        };

        self.items.borrow_mut().push(QueueArtworkItem {
            preview,
            path,
            state,
        });
    }

    fn unregister(&self, preview: StaticBitmap) {
        self.items
            .borrow_mut()
            .retain(|item| item.preview.handle_ptr() != preview.handle_ptr());
    }

    fn replace_or_register(&self, preview: StaticBitmap, path: PathBuf) {
        let mut should_load = false;
        let mut items = self.items.borrow_mut();
        if let Some(item) = items
            .iter_mut()
            .find(|item| item.preview.handle_ptr() == preview.handle_ptr())
        {
            if item.path == path {
                if item.state == QueueArtworkState::Loaded
                    || item.state == QueueArtworkState::Loading
                {
                    return;
                }
            } else {
                item.path = path.clone();
                clear_queue_artwork_preview(preview);
            }

            if self.set_cached_bitmap(preview, &path) {
                item.state = QueueArtworkState::Loaded;
            } else {
                item.state = QueueArtworkState::Loading;
                should_load = true;
            }
        } else {
            let state = if self.set_cached_bitmap(preview, &path) {
                QueueArtworkState::Loaded
            } else {
                should_load = true;
                QueueArtworkState::Loading
            };
            items.push(QueueArtworkItem {
                preview,
                path: path.clone(),
                state,
            });
        }
        drop(items);

        if should_load {
            load_queue_artwork_preview_async(preview, path);
        }
    }

    fn set_cached_bitmap(&self, preview: StaticBitmap, path: &Path) -> bool {
        let Some(rgba) = self.cache.borrow().get(path).cloned() else {
            return false;
        };

        if let Some(bitmap) =
            Bitmap::from_rgba(&rgba, QUEUE_COVER_SIZE as u32, QUEUE_COVER_SIZE as u32)
        {
            preview.set_bitmap(&bitmap);
            return true;
        }

        false
    }

    fn finish_load(&self, preview: StaticBitmap, path: PathBuf, rgba: Vec<u8>) -> bool {
        self.cache.borrow_mut().insert(path.clone(), rgba);

        let mut items = self.items.borrow_mut();
        let Some(item) = items
            .iter_mut()
            .find(|item| item.preview.handle_ptr() == preview.handle_ptr())
        else {
            return false;
        };

        if item.path != path {
            return false;
        }

        item.state = QueueArtworkState::Loaded;
        true
    }

    fn update_scroll_position(&self, position: i32) {
        *self.last_scroll_position.borrow_mut() = position;
        self.load_visible();
    }

    fn load_visible(&self) {
        let viewport_height = self.viewport.get_client_size().height.max(0);
        let start_y = *self.last_scroll_position.borrow();
        let end_y = start_y + viewport_height + QUEUE_LAZY_PRELOAD_MARGIN;
        let start_y = start_y.saturating_sub(QUEUE_LAZY_PRELOAD_MARGIN);

        let mut items = self.items.borrow_mut();
        for (index, item) in items.iter_mut().enumerate() {
            if item.state != QueueArtworkState::Pending {
                continue;
            }

            let item_y = (index as i32) * QUEUE_CARD_HEIGHT_ESTIMATE;
            if item_y + QUEUE_CARD_HEIGHT_ESTIMATE < start_y || item_y > end_y {
                continue;
            }

            item.state = QueueArtworkState::Loading;
            load_queue_artwork_preview_async(item.preview, item.path.clone());
        }
    }

    fn load_next_pending(&self) {
        let mut items = self.items.borrow_mut();
        let mut next_index = self.next_index.borrow_mut();
        while *next_index < items.len() {
            let item = &mut items[*next_index];
            *next_index += 1;
            if item.state == QueueArtworkState::Pending {
                item.state = QueueArtworkState::Loading;
                load_queue_artwork_preview_async(item.preview, item.path.clone());
                break;
            }
        }
    }
}

fn clear_queue_artwork_preview(preview: StaticBitmap) {
    if let Some(bitmap) = Bitmap::new(QUEUE_COVER_SIZE, QUEUE_COVER_SIZE) {
        preview.set_bitmap(&bitmap);
    }
}

fn bind_queue_artwork_loader(queue_list_panel: &ScrolledWindow, loader: Rc<QueueArtworkLoader>) {
    let loader_for_line_down = loader.clone();
    queue_list_panel.on_scroll_linedown(move |event| {
        if let Some(position) = event.get_position() {
            loader_for_line_down.update_scroll_position(position * 5);
        } else {
            loader_for_line_down.load_next_pending();
        }
    });

    let loader_for_line_up = loader.clone();
    queue_list_panel.on_scroll_lineup(move |event| {
        if let Some(position) = event.get_position() {
            loader_for_line_up.update_scroll_position(position * 5);
        } else {
            loader_for_line_up.load_visible();
        }
    });

    let loader_for_page_down = loader.clone();
    queue_list_panel.on_scroll_pagedown(move |event| {
        if let Some(position) = event.get_position() {
            loader_for_page_down.update_scroll_position(position * 5);
        } else {
            loader_for_page_down.load_next_pending();
        }
    });

    let loader_for_page_up = loader.clone();
    queue_list_panel.on_scroll_pageup(move |event| {
        if let Some(position) = event.get_position() {
            loader_for_page_up.update_scroll_position(position * 5);
        } else {
            loader_for_page_up.load_visible();
        }
    });

    let loader_for_thumb_track = loader.clone();
    queue_list_panel.on_thumb_track(move |event| {
        if let Some(position) = event.get_position() {
            loader_for_thumb_track.update_scroll_position(position * 5);
        }
    });

    let loader_for_thumb_release = loader.clone();
    queue_list_panel.on_thumb_release(move |event| {
        if let Some(position) = event.get_position() {
            loader_for_thumb_release.update_scroll_position(position * 5);
        }
    });

    let loader_for_changed = loader.clone();
    queue_list_panel.on_scroll_changed(move |event| {
        if let Some(position) = event.get_position() {
            loader_for_changed.update_scroll_position(position * 5);
        } else {
            loader_for_changed.load_visible();
        }
    });

    let loader_for_wheel = loader.clone();
    queue_list_panel.on_mouse_wheel(move |_| {
        loader_for_wheel.load_next_pending();
    });

    queue_list_panel.on_size(move |_| {
        loader.load_visible();
    });
}

fn load_queue_artwork_preview_async(preview: StaticBitmap, path: PathBuf) {
    std::thread::spawn(move || {
        let preview_rgba = make_queue_cover_rgba(&path);

        wxdragon::call_after(Box::new(move || {
            if !preview.is_valid() {
                return;
            }

            if let Some(rgba) = preview_rgba {
                let should_update = QUEUE_ARTWORK_LOADER.with(|loader| {
                    loader
                        .borrow()
                        .as_ref()
                        .map(|loader| loader.finish_load(preview, path.clone(), rgba.clone()))
                        .unwrap_or(true)
                });

                if should_update {
                    if let Some(bitmap) =
                        Bitmap::from_rgba(&rgba, QUEUE_COVER_SIZE as u32, QUEUE_COVER_SIZE as u32)
                    {
                        preview.set_bitmap(&bitmap);
                    }
                }
            }
        }));
    });
}

fn make_queue_cover_rgba(path: &Path) -> Option<Vec<u8>> {
    make_square_cover_rgba(path, QUEUE_COVER_SIZE as u32)
}

pub fn make_square_cover_rgba(path: &Path, preview_size: u32) -> Option<Vec<u8>> {
    let image = image::ImageReader::open(path)
        .ok()?
        .with_guessed_format()
        .ok()?
        .decode()
        .ok()?;
    let (width, height) = image.dimensions();
    if width == 0 || height == 0 {
        return None;
    }

    let crop_size = width.min(height);
    let x = width.saturating_sub(crop_size) / 2;
    let y = height.saturating_sub(crop_size) / 2;
    let cropped = image.crop_imm(x, y, crop_size, crop_size).to_rgba8();
    let resized = image::imageops::thumbnail(&cropped, preview_size, preview_size);

    Some(resized.into_raw())
}
