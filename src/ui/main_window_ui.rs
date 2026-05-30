use wxdragon::color::Colour;
use wxdragon::ffi;
use wxdragon::geometry::Size;
use wxdragon::id::{ID_ABOUT, ID_EXIT, ID_HIGHEST};
use wxdragon::prelude::*;

use image::imageops::FilterType;
use std::cell::RefCell;
use std::path::{Path, PathBuf};
use std::rc::Rc;

const ID_FILE_NEW_PROJECT: i32 = ID_HIGHEST + 1;
const ID_FILE_OPEN: i32 = ID_HIGHEST + 2;
const ID_FILE_SAVE: i32 = ID_HIGHEST + 3;
const ID_FILE_SAVE_AS: i32 = ID_HIGHEST + 4;
const WX_LEFT: ffi::wxd_Direction_t = 0x0010;
const QUEUE_COVER_SIZE: i32 = 96;
const QUEUE_CARD_HEIGHT_ESTIMATE: i32 = QUEUE_COVER_SIZE + 20;
const QUEUE_LAZY_PRELOAD_MARGIN: i32 = QUEUE_CARD_HEIGHT_ESTIMATE * 3;

thread_local! {
    static QUEUE_ARTWORK_LOADER: RefCell<Option<Rc<QueueArtworkLoader>>> = const { RefCell::new(None) };
}

#[derive(Clone)]
pub struct FrameUI {
    pub main_frame: Frame,
    pub main_status: StatusBar,
    pub main_splitter: SplitterWindow,
    pub add_queue_button: Button,
    pub work_dir_text: TextCtrl,
    pub work_dir_browse_button: Button,
    pub main_panel: Panel,
    pub queue_panel: Panel,
    pub queue_list_panel: ScrolledWindow,
    pub queue_list_sizer: BoxSizer,
    pub empty_queue_panel: Panel,
    pub empty_queue_text: StaticText,
    pub preview_panel: Panel,
}

impl FrameUI {
    pub fn new() -> Self {
        let main_frame = Frame::builder()
            .with_title("mu2vid")
            .with_size(Size::new(980, 680))
            .build();

        setup_menu_bar(&main_frame);
        let main_status = StatusBar::builder(&main_frame)
            .with_fields_count(2)
            .with_status_widths(vec![-1, 240])
            .add_initial_text(0, "Ready")
            .build();

        let main_panel = Panel::builder(&main_frame).build();
        let main_sizer = BoxSizer::builder(Orientation::Vertical).build();

        let main_splitter = SplitterWindow::builder(&main_panel)
            .with_style(SplitterWindowStyle::Default | SplitterWindowStyle::LiveUpdate)
            .build();
        main_splitter.set_minimum_pane_size(1);

        let queue_ui = create_queue_panel(&main_splitter);
        let preview_panel = create_preview_panel(&main_splitter);
        main_splitter.split_vertically(&queue_ui.queue_panel, &preview_panel, 560);

        main_sizer.add(&main_splitter, 1, SizerFlag::Expand | SizerFlag::All, 8);
        let bottom_controls_ui = create_bottom_controls(&main_panel);
        main_sizer.add_sizer(
            &bottom_controls_ui.sizer,
            0,
            SizerFlag::Expand | SizerFlag::Left | SizerFlag::Right | SizerFlag::Bottom,
            8,
        );
        main_panel.set_sizer(main_sizer, true);

        Self {
            main_frame,
            main_status,
            main_splitter,
            add_queue_button: bottom_controls_ui.add_queue_button,
            work_dir_text: bottom_controls_ui.work_dir_text,
            work_dir_browse_button: bottom_controls_ui.work_dir_browse_button,
            main_panel,
            queue_panel: queue_ui.queue_panel,
            queue_list_panel: queue_ui.queue_list_panel,
            queue_list_sizer: queue_ui.queue_list_sizer,
            empty_queue_panel: queue_ui.empty_queue_panel,
            empty_queue_text: queue_ui.empty_queue_text,
            preview_panel,
        }
    }

    pub fn add_queue_item(
        &self,
        title: &str,
        artwork_path: &str,
        video_quality: &str,
        audio_quality: &str,
    ) {
        self.empty_queue_panel.show(false);

        let queue_item = Panel::builder(&self.queue_list_panel)
            .with_style(PanelStyle::BorderSimple | PanelStyle::TabTraversal)
            .build();
        let queue_item_sizer = BoxSizer::builder(Orientation::Horizontal).build();

        let cover_placeholder = Panel::builder(&queue_item)
            .with_style(PanelStyle::BorderSimple | PanelStyle::TabTraversal)
            .with_size(Size::new(QUEUE_COVER_SIZE, QUEUE_COVER_SIZE))
            .build();
        let cover_sizer = BoxSizer::builder(Orientation::Vertical).build();
        let cover_bitmap = StaticBitmap::builder(&cover_placeholder)
            .with_bitmap(Some(
                Bitmap::new(QUEUE_COVER_SIZE, QUEUE_COVER_SIZE).unwrap_or_else(Bitmap::null_bitmap),
            ))
            .with_size(Size::new(QUEUE_COVER_SIZE, QUEUE_COVER_SIZE))
            .with_scale_mode(Some(ScaleMode::AspectFill))
            .build();
        cover_sizer.add(&cover_bitmap, 0, SizerFlag::AlignCentre, 0);
        cover_placeholder.set_sizer(cover_sizer, true);
        queue_item_sizer.add(&cover_placeholder, 0, SizerFlag::All, 6);

        let info_sizer = BoxSizer::builder(Orientation::Vertical).build();
        info_sizer.add(
            &StaticText::builder(&queue_item).with_label(title).build(),
            0,
            SizerFlag::Expand,
            0,
        );
        info_sizer.add(
            &StaticText::builder(&queue_item)
                .with_label(&format!("Video: {video_quality} | Audio: {audio_quality}"))
                .build(),
            0,
            SizerFlag::Expand | SizerFlag::Top,
            4,
        );
        info_sizer.add(
            &StaticText::builder(&queue_item)
                .with_label("Status: waiting")
                .build(),
            0,
            SizerFlag::Expand | SizerFlag::Top,
            4,
        );
        let item_progress = Gauge::builder(&queue_item).with_range(100).build();
        item_progress.set_value(0);
        info_sizer.add(&item_progress, 0, SizerFlag::Expand | SizerFlag::Top, 4);
        queue_item_sizer.add_sizer(
            &info_sizer,
            1,
            SizerFlag::Expand | SizerFlag::Top | SizerFlag::Bottom,
            6,
        );

        let actions_sizer = BoxSizer::builder(Orientation::Vertical).build();
        let up_button = Button::builder(&queue_item)
            .with_label("Up")
            .with_size(Size::new(64, -1))
            .build();
        set_button_icon(up_button, ArtId::GoUp);
        actions_sizer.add(&up_button, 0, SizerFlag::Expand | SizerFlag::Bottom, 2);
        let down_button = Button::builder(&queue_item)
            .with_label("Down")
            .with_size(Size::new(64, -1))
            .build();
        set_button_icon(down_button, ArtId::GoDown);
        actions_sizer.add(&down_button, 0, SizerFlag::Expand | SizerFlag::Bottom, 2);
        let delete_button = Button::builder(&queue_item)
            .with_label("Delete")
            .with_size(Size::new(64, -1))
            .build();
        set_button_icon(delete_button, ArtId::Delete);
        actions_sizer.add(&delete_button, 0, SizerFlag::Expand, 0);
        queue_item_sizer.add_sizer(&actions_sizer, 0, SizerFlag::Expand | SizerFlag::All, 6);

        queue_item.set_sizer(queue_item_sizer, true);
        self.queue_list_sizer
            .add(&queue_item, 0, SizerFlag::Expand | SizerFlag::All, 4);
        self.queue_list_panel.layout();
        self.queue_panel.layout();
        self.main_frame.layout();

        QUEUE_ARTWORK_LOADER.with(|loader| {
            if let Some(loader) = loader.borrow().as_ref() {
                loader.register(cover_bitmap, PathBuf::from(artwork_path));
                loader.load_visible();
            }
        });
    }

    pub fn apply_colors(&self, dark_mode: bool) {
        let colors = UiColors::new(dark_mode);

        self.main_frame
            .set_background_style(BackgroundStyle::Colour);
        self.main_panel
            .set_background_style(BackgroundStyle::Colour);
        self.main_splitter
            .set_background_style(BackgroundStyle::Colour);
        self.queue_panel
            .set_background_style(BackgroundStyle::Colour);
        self.queue_list_panel
            .set_background_style(BackgroundStyle::Colour);
        self.empty_queue_panel
            .set_background_style(BackgroundStyle::Colour);
        self.empty_queue_text
            .set_background_style(BackgroundStyle::Colour);
        self.preview_panel
            .set_background_style(BackgroundStyle::Colour);
        self.main_status
            .set_background_style(BackgroundStyle::Colour);

        self.main_frame.set_background_color(colors.background);
        self.main_panel.set_background_color(colors.background);
        self.main_splitter.set_background_color(colors.background);
        self.queue_panel.set_background_color(colors.background);
        self.queue_list_panel
            .set_background_color(colors.background);
        self.empty_queue_panel
            .set_background_color(colors.background);
        self.empty_queue_text
            .set_background_color(colors.background);
        self.preview_panel.set_background_color(colors.background);
        self.main_status.set_background_color(colors.status);

        self.main_frame.set_foreground_color(colors.foreground);
        self.main_panel.set_foreground_color(colors.foreground);
        self.main_splitter.set_foreground_color(colors.foreground);
        self.queue_panel.set_foreground_color(colors.foreground);
        self.queue_list_panel
            .set_foreground_color(colors.foreground);
        self.empty_queue_panel
            .set_foreground_color(colors.foreground);
        self.empty_queue_text
            .set_foreground_color(colors.foreground);
        self.preview_panel.set_foreground_color(colors.foreground);
        self.main_status.set_foreground_color(colors.foreground);

        self.main_frame.layout();
        self.main_frame.refresh(true, None);
        self.main_frame.update();
    }
}

struct QueuePanelUI {
    queue_panel: Panel,
    queue_list_panel: ScrolledWindow,
    queue_list_sizer: BoxSizer,
    empty_queue_panel: Panel,
    empty_queue_text: StaticText,
}

struct BottomControlsUI {
    sizer: BoxSizer,
    add_queue_button: Button,
    work_dir_text: TextCtrl,
    work_dir_browse_button: Button,
}

struct UiColors {
    background: Colour,
    status: Colour,
    foreground: Colour,
}

impl UiColors {
    fn new(dark_mode: bool) -> Self {
        if dark_mode {
            Self {
                background: Colour::rgb(32, 32, 32),
                status: Colour::rgb(32, 32, 32),
                foreground: Colour::rgb(240, 240, 240),
            }
        } else {
            Self {
                background: Colour::rgb(240, 240, 240),
                status: Colour::rgb(240, 240, 240),
                foreground: Colour::rgb(0, 0, 0),
            }
        }
    }
}

fn setup_menu_bar(frame: &Frame) {
    let file_menu = Menu::builder()
        .append_item(ID_FILE_NEW_PROJECT, "New Project\tCtrl+N", "")
        .append_item(ID_FILE_OPEN, "Open\tCtrl+O", "")
        .append_item(ID_FILE_SAVE, "Save\tCtrl+S", "")
        .append_item(ID_FILE_SAVE_AS, "Save as\tCtrl+Alt+S", "")
        .append_separator()
        .append_item(ID_EXIT, "Exit", "")
        .build();
    set_menu_item_icon(&file_menu, ID_FILE_NEW_PROJECT, ArtId::New);
    set_menu_item_icon(&file_menu, ID_FILE_OPEN, ArtId::FileOpen);
    set_menu_item_icon(&file_menu, ID_FILE_SAVE, ArtId::FileSave);
    set_menu_item_icon(&file_menu, ID_FILE_SAVE_AS, ArtId::FileSaveAs);
    set_menu_item_icon(&file_menu, ID_EXIT, ArtId::Quit);

    let help_menu = Menu::builder()
        .append_item(ID_ABOUT, "About mu2vid", "")
        .build();
    set_menu_item_icon(&help_menu, ID_ABOUT, ArtId::Information);

    let menu_bar = MenuBar::builder()
        .append(file_menu, "File")
        .append(help_menu, "Help")
        .build();

    frame.set_menu_bar(menu_bar);
}

fn set_menu_item_icon(menu: &Menu, id: i32, art_id: ArtId) {
    if let (Some(item), Some(bitmap)) = (
        menu.find_item(id),
        ArtProvider::get_bitmap(art_id, ArtClient::Menu, None),
    ) {
        item.set_bitmap(&bitmap);
    }
}

fn button_icon(art_id: ArtId) -> Option<BitmapBundle> {
    ArtProvider::get_bitmap_bundle(art_id, ArtClient::Button, None)
}

fn set_button_icon(button: Button, art_id: ArtId) {
    if let Some(icon) = button_icon(art_id) {
        unsafe {
            ffi::wxd_Button_SetBitmapBundle(
                button.handle_ptr() as *mut ffi::wxd_Button_t,
                icon.as_ptr(),
                WX_LEFT,
            );
        }
    }
}

fn art_button(parent: &dyn WxWidget, label: &str, art_id: ArtId) -> Button {
    let button = Button::builder(parent).with_label(label).build();
    set_button_icon(button, art_id);
    button
}

fn create_queue_panel(parent: &SplitterWindow) -> QueuePanelUI {
    let queue_panel = Panel::builder(parent).build();
    let queue_panel_sizer = BoxSizer::builder(Orientation::Vertical).build();
    let queue_list_panel = ScrolledWindow::builder(&queue_panel)
        .with_style(ScrolledWindowStyle::VScroll)
        .build();
    queue_list_panel.set_scroll_rate(5, 5);

    let artwork_loader = QueueArtworkLoader::new(queue_list_panel);
    QUEUE_ARTWORK_LOADER.with(|loader| {
        *loader.borrow_mut() = Some(artwork_loader.clone());
    });
    bind_queue_artwork_loader(&queue_list_panel, artwork_loader);

    let queue_list_sizer = BoxSizer::builder(Orientation::Vertical).build();
    let empty_queue_panel = Panel::builder(&queue_list_panel).build();
    let empty_queue_sizer = BoxSizer::builder(Orientation::Vertical).build();
    empty_queue_sizer.add_stretch_spacer(1);
    let empty_queue_text = StaticText::builder(&empty_queue_panel)
        .with_label("no album added")
        .with_style(StaticTextStyle::AlignCenterHorizontal)
        .build();
    empty_queue_sizer.add(
        &empty_queue_text,
        0,
        SizerFlag::AlignCentre | SizerFlag::All,
        12,
    );
    empty_queue_sizer.add_stretch_spacer(1);
    empty_queue_panel.set_sizer(empty_queue_sizer, true);

    queue_list_sizer.add(&empty_queue_panel, 1, SizerFlag::Expand | SizerFlag::All, 4);
    queue_list_panel.set_sizer(queue_list_sizer, true);
    queue_panel_sizer.add(&queue_list_panel, 1, SizerFlag::Expand | SizerFlag::All, 8);
    queue_panel.set_sizer(queue_panel_sizer, true);

    QueuePanelUI {
        queue_panel,
        queue_list_panel,
        queue_list_sizer,
        empty_queue_panel,
        empty_queue_text,
    }
}

struct QueueArtworkLoader {
    viewport: ScrolledWindow,
    items: RefCell<Vec<QueueArtworkItem>>,
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
}

impl QueueArtworkLoader {
    fn new(viewport: ScrolledWindow) -> Rc<Self> {
        Rc::new(Self {
            viewport,
            items: RefCell::new(Vec::new()),
            next_index: RefCell::new(0),
            last_scroll_position: RefCell::new(0),
        })
    }

    fn register(&self, preview: StaticBitmap, path: PathBuf) {
        self.items.borrow_mut().push(QueueArtworkItem {
            preview,
            path,
            state: QueueArtworkState::Pending,
        });
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

            if let Some(bitmap) = preview_rgba.and_then(|rgba| {
                Bitmap::from_rgba(&rgba, QUEUE_COVER_SIZE as u32, QUEUE_COVER_SIZE as u32)
            }) {
                preview.set_bitmap(&bitmap);
            }
        }));
    });
}

fn make_queue_cover_rgba(path: &Path) -> Option<Vec<u8>> {
    let image = image::open(path).ok()?.to_rgba8();
    let (width, height) = image.dimensions();
    if width == 0 || height == 0 {
        return None;
    }

    let preview_size = QUEUE_COVER_SIZE as u32;
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

fn create_preview_panel(parent: &SplitterWindow) -> Panel {
    let preview_panel = Panel::builder(parent).build();
    let preview_sizer = BoxSizer::builder(Orientation::Vertical).build();
    preview_sizer.add_stretch_spacer(1);
    preview_sizer.add(
        &StaticText::builder(&preview_panel)
            .with_label("Check back later :3")
            .with_style(StaticTextStyle::AlignCenterHorizontal)
            .build(),
        0,
        SizerFlag::AlignCentre | SizerFlag::All,
        12,
    );
    preview_sizer.add_stretch_spacer(1);
    preview_panel.set_sizer(preview_sizer, true);

    preview_panel
}

fn create_bottom_controls(parent: &Panel) -> BottomControlsUI {
    let bottom_controls_sizer = BoxSizer::builder(Orientation::Horizontal).build();
    let add_queue_button = art_button(parent, "Add new queue", ArtId::New);
    bottom_controls_sizer.add(&add_queue_button, 0, SizerFlag::Right, 4);
    bottom_controls_sizer.add(
        &art_button(parent, "Start", ArtId::TickMark),
        0,
        SizerFlag::Right,
        4,
    );
    bottom_controls_sizer.add(
        &art_button(parent, "Stop", ArtId::Delete),
        0,
        SizerFlag::Right,
        8,
    );

    bottom_controls_sizer.add(
        &StaticText::builder(parent).with_label("Work dir").build(),
        0,
        SizerFlag::AlignCenterVertical | SizerFlag::Right,
        4,
    );
    let work_dir_text = TextCtrl::builder(parent)
        .with_size(Size::new(260, -1))
        .build();
    bottom_controls_sizer.add(&work_dir_text, 1, SizerFlag::Expand | SizerFlag::Right, 4);
    let work_dir_browse_button = Button::builder(parent)
        .with_label("Browse")
        .with_size(Size::new(92, -1))
        .build();
    set_button_icon(work_dir_browse_button, ArtId::FileOpen);
    bottom_controls_sizer.add(
        &work_dir_browse_button,
        0,
        SizerFlag::AlignCenterVertical,
        0,
    );

    BottomControlsUI {
        sizer: bottom_controls_sizer,
        add_queue_button,
        work_dir_text,
        work_dir_browse_button,
    }
}
