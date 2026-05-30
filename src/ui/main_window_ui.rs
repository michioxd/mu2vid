use wxdragon::color::Colour;
use wxdragon::ffi;
use wxdragon::geometry::Size;
use wxdragon::id::{ID_ABOUT, ID_EXIT, ID_HIGHEST, ID_OK};
use wxdragon::prelude::*;

use crate::media::artwork;
use crate::project;

pub const ID_FILE_NEW_PROJECT: i32 = ID_HIGHEST + 1;
pub const ID_FILE_OPEN: i32 = ID_HIGHEST + 2;
pub const ID_FILE_SAVE: i32 = ID_HIGHEST + 3;
pub const ID_FILE_SAVE_AS: i32 = ID_HIGHEST + 4;
pub const ID_FILE_RECENT_PROJECT_START: i32 = ID_HIGHEST + 20;
pub const MAX_RECENT_PROJECT_MENU_ITEMS: usize = 10;
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
    pub queue_list_sizer: BoxSizer,
    pub empty_queue_panel: Panel,
    pub empty_queue_text: StaticText,
    pub preview_panel: Panel,
}

#[derive(Clone, Copy)]
pub struct QueueItemUI {
    pub panel: Panel,
    pub title_text: StaticText,
    pub quality_text: StaticText,
    pub cover_bitmap: StaticBitmap,
    pub up_button: Button,
    pub down_button: Button,
    pub edit_button: Button,
    pub delete_button: Button,
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
        audio_label: &str,
    ) -> QueueItemUI {
        self.empty_queue_panel.show(false);

        let queue_item = Panel::builder(&self.queue_list_panel)
            .with_style(PanelStyle::BorderSimple | PanelStyle::TabTraversal)
            .build();
        let queue_item_sizer = BoxSizer::builder(Orientation::Horizontal).build();

        let cover_placeholder = Panel::builder(&queue_item)
            .with_style(PanelStyle::BorderSimple | PanelStyle::TabTraversal)
            .with_size(artwork::queue_cover_size())
            .build();
        let cover_sizer = BoxSizer::builder(Orientation::Vertical).build();
        let cover_bitmap = StaticBitmap::builder(&cover_placeholder)
            .with_bitmap(Some(artwork::queue_cover_placeholder()))
            .with_size(artwork::queue_cover_size())
            .with_scale_mode(Some(ScaleMode::AspectFill))
            .build();
        cover_sizer.add(&cover_bitmap, 0, SizerFlag::AlignCentre, 0);
        cover_placeholder.set_sizer(cover_sizer, true);
        queue_item_sizer.add(&cover_placeholder, 0, SizerFlag::All, 6);

        let info_sizer = BoxSizer::builder(Orientation::Vertical).build();
        let title_text = StaticText::builder(&queue_item).with_label(title).build();
        info_sizer.add(&title_text, 0, SizerFlag::Expand, 0);
        let quality_text = StaticText::builder(&queue_item)
            .with_label(&format!("Video: {video_quality} | Audio: {audio_label}"))
            .build();
        info_sizer.add(&quality_text, 0, SizerFlag::Expand | SizerFlag::Top, 4);
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
        let move_actions_sizer = BoxSizer::builder(Orientation::Horizontal).build();
        let up_button = Button::builder(&queue_item)
            .with_label("")
            .with_size(Size::new(31, -1))
            .build();
        set_button_icon(up_button, ArtId::GoUp);
        move_actions_sizer.add(&up_button, 1, SizerFlag::Expand | SizerFlag::Right, 2);
        let down_button = Button::builder(&queue_item)
            .with_label("")
            .with_size(Size::new(31, -1))
            .build();
        set_button_icon(down_button, ArtId::GoDown);
        move_actions_sizer.add(&down_button, 1, SizerFlag::Expand, 0);
        actions_sizer.add_sizer(
            &move_actions_sizer,
            0,
            SizerFlag::Expand | SizerFlag::Bottom,
            2,
        );
        let edit_button = Button::builder(&queue_item)
            .with_label("Edit")
            .with_size(Size::new(64, -1))
            .build();
        set_button_icon(edit_button, ArtId::FileOpen);
        actions_sizer.add(&edit_button, 0, SizerFlag::Expand | SizerFlag::Bottom, 2);
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

        artwork::register_queue_artwork(cover_bitmap, artwork_path);

        QueueItemUI {
            panel: queue_item,
            title_text,
            quality_text,
            cover_bitmap,
            up_button,
            down_button,
            edit_button,
            delete_button,
        }
    }

    pub fn update_queue_item_artwork(&self, cover_bitmap: StaticBitmap, artwork_path: &str) {
        artwork::update_queue_artwork(cover_bitmap, artwork_path);
    }

    pub fn remove_queue_item(&self, item_ui: QueueItemUI) {
        artwork::unregister_queue_artwork(item_ui.cover_bitmap);
        item_ui.panel.show(false);
        let panel = item_ui.panel;
        wxdragon::call_after(Box::new(move || {
            if panel.is_valid() {
                panel.destroy();
            }
        }));
        self.queue_list_panel.layout();
        self.queue_panel.layout();
        self.main_frame.layout();
    }

    pub fn sync_queue_items(&self, items: &[(QueueItemUI, String, String, String, String)]) {
        self.empty_queue_panel.show(items.is_empty());

        for (index, (item_ui, title, artwork_path, video_quality, audio_label)) in
            items.iter().enumerate()
        {
            item_ui.panel.show(true);
            item_ui.title_text.set_label(title);
            item_ui
                .quality_text
                .set_label(&format!("Video: {video_quality} | Audio: {audio_label}"));
            item_ui.up_button.enable(index > 0);
            item_ui.down_button.enable(index + 1 < items.len());
            self.update_queue_item_artwork(item_ui.cover_bitmap, artwork_path);
        }

        self.queue_list_panel.layout();
        self.queue_panel.layout();
        self.main_frame.layout();
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

    pub fn refresh_menu_bar(&self) {
        setup_menu_bar(&self.main_frame);
    }
}

pub fn prompt_project_title(parent: &Frame, current_title: &str) -> Option<String> {
    let dialog = TextEntryDialog::builder(parent, "Project title", "Change project title")
        .with_default_value(current_title)
        .build();

    if dialog.show_modal() == ID_OK {
        dialog.get_value()
    } else {
        None
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
    let config = crate::config::load();
    let recent_menu = Menu::builder().build();
    if config.recent_projects.is_empty() {
        let _ = recent_menu.append(
            ID_FILE_RECENT_PROJECT_START,
            "No recent projects",
            "",
            ItemKind::Normal,
        );
        recent_menu.enable_item(ID_FILE_RECENT_PROJECT_START, false);
    } else {
        for (index, path) in config
            .recent_projects
            .iter()
            .take(MAX_RECENT_PROJECT_MENU_ITEMS)
            .enumerate()
        {
            let label = format!("{} {}", index + 1, path);
            let _ = recent_menu.append(
                ID_FILE_RECENT_PROJECT_START + index as i32,
                &label,
                "",
                ItemKind::Normal,
            );
        }
    }

    let file_menu = Menu::builder()
        .append_item(ID_FILE_NEW_PROJECT, "New Project\tCtrl+N", "")
        .append_item(ID_FILE_OPEN, "Open\tCtrl+O", "")
        .append_item(ID_FILE_SAVE, "Save\tCtrl+S", "")
        .append_item(ID_FILE_SAVE_AS, "Save as\tCtrl+Alt+S", "")
        .build();
    let _ = file_menu.append_submenu(recent_menu, "Recent Project", "");
    file_menu.append_separator();
    let _ = file_menu.append(ID_EXIT, "Exit", "", ItemKind::Normal);
    set_menu_item_icon(&file_menu, ID_FILE_NEW_PROJECT, ArtId::New);
    set_menu_item_icon(&file_menu, ID_FILE_OPEN, ArtId::FileOpen);
    set_menu_item_icon(&file_menu, ID_FILE_SAVE, ArtId::FileSave);
    set_menu_item_icon(&file_menu, ID_FILE_SAVE_AS, ArtId::FileSaveAs);
    set_menu_item_icon(&file_menu, ID_EXIT, ArtId::Quit);

    let help_menu = Menu::builder()
        .append_item(ID_ABOUT, "About mu2vid", "")
        .build();
    set_menu_item_icon(&help_menu, ID_ABOUT, ArtId::Information);

    let project_menu = Menu::builder()
        .append_item(
            project::ID_CHANGE_TITLE,
            "Change project title",
            "Change the title of the current project",
        )
        .build();

    let menu_bar = MenuBar::builder()
        .append(file_menu, "File")
        .append(project_menu, "Project")
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
    artwork::install_queue_artwork_loader(&queue_list_panel);

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
