use wxdragon::color::Colour;
use wxdragon::ffi;
use wxdragon::geometry::Size;
use wxdragon::id::{ID_ABOUT, ID_EXIT, ID_HIGHEST};
use wxdragon::prelude::*;

const ID_FILE_NEW_PROJECT: i32 = ID_HIGHEST + 1;
const ID_FILE_OPEN: i32 = ID_HIGHEST + 2;
const ID_FILE_SAVE: i32 = ID_HIGHEST + 3;
const ID_FILE_SAVE_AS: i32 = ID_HIGHEST + 4;
const WX_LEFT: ffi::wxd_Direction_t = 0x0010;

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
    pub queue_item: Panel,
    pub cover_placeholder: Panel,
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
            queue_item: queue_ui.queue_item,
            cover_placeholder: queue_ui.cover_placeholder,
            preview_panel,
        }
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
        self.queue_item
            .set_background_style(BackgroundStyle::Colour);
        self.cover_placeholder
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
        self.queue_item.set_background_color(colors.panel);
        self.cover_placeholder
            .set_background_color(colors.placeholder);
        self.preview_panel.set_background_color(colors.background);
        self.main_status.set_background_color(colors.status);

        self.main_frame.set_foreground_color(colors.foreground);
        self.main_panel.set_foreground_color(colors.foreground);
        self.main_splitter.set_foreground_color(colors.foreground);
        self.queue_panel.set_foreground_color(colors.foreground);
        self.queue_list_panel
            .set_foreground_color(colors.foreground);
        self.queue_item.set_foreground_color(colors.foreground);
        self.cover_placeholder
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
    queue_item: Panel,
    cover_placeholder: Panel,
}

struct BottomControlsUI {
    sizer: BoxSizer,
    add_queue_button: Button,
    work_dir_text: TextCtrl,
    work_dir_browse_button: Button,
}

struct UiColors {
    background: Colour,
    panel: Colour,
    placeholder: Colour,
    status: Colour,
    foreground: Colour,
}

impl UiColors {
    fn new(dark_mode: bool) -> Self {
        if dark_mode {
            Self {
                background: Colour::rgb(32, 32, 32),
                panel: Colour::rgb(43, 43, 43),
                placeholder: Colour::rgb(55, 55, 55),
                status: Colour::rgb(32, 32, 32),
                foreground: Colour::rgb(240, 240, 240),
            }
        } else {
            Self {
                background: Colour::rgb(240, 240, 240),
                panel: Colour::rgb(255, 255, 255),
                placeholder: Colour::rgb(245, 245, 245),
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

    let queue_list_sizer = BoxSizer::builder(Orientation::Vertical).build();
    let queue_item = Panel::builder(&queue_list_panel)
        .with_style(PanelStyle::BorderSimple | PanelStyle::TabTraversal)
        .build();
    let queue_item_sizer = BoxSizer::builder(Orientation::Horizontal).build();

    let cover_placeholder = Panel::builder(&queue_item)
        .with_style(PanelStyle::BorderSimple | PanelStyle::TabTraversal)
        .with_size(Size::new(96, 96))
        .build();
    queue_item_sizer.add(&cover_placeholder, 0, SizerFlag::Expand | SizerFlag::All, 6);

    let info_sizer = BoxSizer::builder(Orientation::Vertical).build();
    info_sizer.add(
        &StaticText::builder(&queue_item)
            .with_label("Queue item title")
            .build(),
        0,
        SizerFlag::Expand,
        0,
    );
    info_sizer.add(
        &StaticText::builder(&queue_item)
            .with_label("Status: waiting")
            .build(),
        0,
        SizerFlag::Expand | SizerFlag::Top,
        0,
    );
    let item_progress = Gauge::builder(&queue_item).with_range(100).build();
    item_progress.set_value(0);
    info_sizer.add(&item_progress, 0, SizerFlag::Expand | SizerFlag::Top, 0);
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
    queue_list_sizer.add(&queue_item, 0, SizerFlag::Expand | SizerFlag::All, 4);
    queue_list_panel.set_sizer(queue_list_sizer, true);
    queue_panel_sizer.add(&queue_list_panel, 1, SizerFlag::Expand | SizerFlag::All, 8);
    queue_panel.set_sizer(queue_panel_sizer, true);

    QueuePanelUI {
        queue_panel,
        queue_list_panel,
        queue_item,
        cover_placeholder,
    }
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
