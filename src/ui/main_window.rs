use crate::ui::about;
use crate::ui::app_state::{DEFAULT_PROJECT_TITLE, ProjectState};
use crate::ui::main_window_ui::{
    FrameUI, ID_FILE_NEW_PROJECT, ID_FILE_OPEN, ID_FILE_RECENT_PROJECT_START, ID_FILE_SAVE,
    ID_FILE_SAVE_AS, MAX_RECENT_PROJECT_MENU_ITEMS, QueueItemUI, prompt_project_title,
};
use crate::ui::new_queue;
use crate::{config, deps::ffmpeg, project};
use std::cell::RefCell;
use std::path::{Path, PathBuf};
use std::rc::Rc;
use wxdragon::appearance::{
    AppAppearance, Appearance, AppearanceResult, get_app as get_appearance_app, is_system_dark_mode,
};
// use wxdragon::event::EventType;
use wxdragon::geometry::{Point, Rect, Size};
use wxdragon::id::{ID_ABOUT, ID_CANCEL, ID_EXIT, ID_OK, ID_YES};
use wxdragon::prelude::*;

const DEFAULT_WINDOW_WIDTH: i32 = 950;
const DEFAULT_WINDOW_HEIGHT: i32 = 600;
const MIN_WINDOW_WIDTH: i32 = 600;
const MIN_WINDOW_HEIGHT: i32 = 500;
const FALLBACK_SCREEN_WIDTH: i32 = 1920;
const FALLBACK_SCREEN_HEIGHT: i32 = 1080;
const MIN_SPLITTER_SASH_POSITION: i32 = 240;
const ID_QUIT_WITHOUT_SAVE: i32 = project::ID_CHANGE_TITLE + 1;

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
    setup_main_controls(&frame_ui);
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

fn setup_window_state_persistence(
    frame_ui: &FrameUI,
    project_state: ProjectState,
    queue_items: Rc<RefCell<Vec<new_queue::QueueItemDraft>>>,
    status_bar: StatusBar,
) {
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
    let frame_ui = frame_ui.clone();
    let main_frame = frame_ui.main_frame;
    main_frame.on_close(move |event| {
        if !confirm_quit_if_dirty(&frame_ui, &project_state, &queue_items, status_bar) {
            event.skip(false);
            return;
        }

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

fn setup_help_menu(
    frame_ui: &FrameUI,
    project_state: ProjectState,
    queue_items: Rc<RefCell<Vec<new_queue::QueueItemDraft>>>,
    status_bar: StatusBar,
) {
    let frame_ui = frame_ui.clone();
    let main_frame = frame_ui.main_frame;
    main_frame.on_menu_selected(move |event| {
        if event.get_id() == ID_ABOUT {
            about::show();
        } else if event.get_id() == ID_EXIT {
            if confirm_quit_if_dirty(&frame_ui, &project_state, &queue_items, status_bar) {
                frame_ui.main_frame.close(false);
            } else {
                event.skip(false);
            }
        } else {
            event.skip(true);
        }
    });
}

fn setup_main_controls(frame_ui: &FrameUI) {
    let status_bar = frame_ui.main_status;
    let frame_ui_for_queue = frame_ui.clone();
    let queue_items = Rc::new(RefCell::new(Vec::<new_queue::QueueItemDraft>::new()));
    let queue_item_uis = Rc::new(RefCell::new(Vec::<QueueItemUI>::new()));
    let project_state = ProjectState::new();

    update_project_title_bar(frame_ui, &project_state);
    load_last_project(
        frame_ui,
        &queue_items,
        &queue_item_uis,
        &project_state,
        status_bar,
    );

    setup_help_menu(
        frame_ui,
        project_state.clone(),
        Rc::clone(&queue_items),
        status_bar,
    );
    setup_window_state_persistence(
        frame_ui,
        project_state.clone(),
        Rc::clone(&queue_items),
        status_bar,
    );

    setup_project_menu(
        frame_ui,
        Rc::clone(&queue_items),
        Rc::clone(&queue_item_uis),
        project_state.clone(),
        status_bar,
    );

    let project_state_for_add = project_state.clone();
    frame_ui.add_queue_button.on_click(move |_| {
        let frame_ui = frame_ui_for_queue.clone();
        let queue_items = Rc::clone(&queue_items);
        let queue_item_uis = Rc::clone(&queue_item_uis);
        let project_state = project_state_for_add.clone();
        let on_add = Rc::new(move |item: new_queue::QueueItemDraft| {
            let item_index = queue_items.borrow().len();
            let queue_item_ui = frame_ui.add_queue_item(
                &item.title,
                &item.artwork_path.to_string_lossy(),
                &item.video_quality,
                &item.audio_display_label(),
            );
            setup_queue_item_edit(
                &frame_ui,
                queue_item_ui,
                Rc::clone(&queue_items),
                Rc::clone(&queue_item_uis),
                item_index,
                project_state.clone(),
                status_bar,
            );
            frame_ui
                .main_status
                .set_status_text(&format!("Added queue: {}", item.title), 0);
            queue_items.borrow_mut().push(item);
            queue_item_uis.borrow_mut().push(queue_item_ui);
            sync_queue_display(&frame_ui, &queue_items, &queue_item_uis);
            mark_project_dirty(&frame_ui, &project_state);
        });

        new_queue::show(status_bar, on_add);
    });

    let main_frame = frame_ui.main_frame;
    let work_dir_text = frame_ui.work_dir_text;
    let frame_ui_for_work_dir = frame_ui.clone();
    let project_state_for_work_dir = project_state.clone();
    frame_ui.work_dir_browse_button.on_click(move |_| {
        if let Some(folder) = choose_work_dir(&main_frame, &work_dir_text.get_value()) {
            work_dir_text.set_value(&folder);
            mark_project_dirty(&frame_ui_for_work_dir, &project_state_for_work_dir);
        }
    });
}

fn setup_project_menu(
    frame_ui: &FrameUI,
    queue_items: Rc<RefCell<Vec<new_queue::QueueItemDraft>>>,
    queue_item_uis: Rc<RefCell<Vec<QueueItemUI>>>,
    project_state: ProjectState,
    status_bar: StatusBar,
) {
    let frame_ui = frame_ui.clone();
    let main_frame = frame_ui.main_frame;
    main_frame.on_menu_selected(move |event| {
        let id = event.get_id();

        if id == ID_FILE_NEW_PROJECT {
            new_project(&frame_ui, &queue_items, &queue_item_uis, &project_state);
        } else if id == ID_FILE_OPEN {
            if let Some(path) = choose_project_file(&frame_ui.main_frame) {
                open_project(
                    &frame_ui,
                    &queue_items,
                    &queue_item_uis,
                    &project_state,
                    status_bar,
                    &path,
                );
            }
        } else if id == ID_FILE_SAVE {
            save_project(&frame_ui, &queue_items, &project_state, false);
        } else if id == ID_FILE_SAVE_AS {
            save_project(&frame_ui, &queue_items, &project_state, true);
        } else if id == project::ID_CHANGE_TITLE {
            change_project_title(&frame_ui, &project_state);
        } else if is_recent_project_id(id) {
            let index = (id - ID_FILE_RECENT_PROJECT_START) as usize;
            let config = config::load();
            if let Some(path) = config.recent_projects.get(index).map(PathBuf::from) {
                open_project(
                    &frame_ui,
                    &queue_items,
                    &queue_item_uis,
                    &project_state,
                    status_bar,
                    &path,
                );
            }
        } else {
            event.skip(true);
        }
    });
}

fn is_recent_project_id(id: i32) -> bool {
    id >= ID_FILE_RECENT_PROJECT_START
        && id < ID_FILE_RECENT_PROJECT_START + MAX_RECENT_PROJECT_MENU_ITEMS as i32
}

fn new_project(
    frame_ui: &FrameUI,
    queue_items: &Rc<RefCell<Vec<new_queue::QueueItemDraft>>>,
    queue_item_uis: &Rc<RefCell<Vec<QueueItemUI>>>,
    project_state: &ProjectState,
) {
    clear_queue(frame_ui, queue_items, queue_item_uis);
    frame_ui.work_dir_text.set_value("");
    project_state.reset();
    clear_last_project_path();
    update_project_title_bar(frame_ui, project_state);
    sync_queue_display(frame_ui, queue_items, queue_item_uis);
    frame_ui.main_status.set_status_text("New project", 0);
}

fn open_project(
    frame_ui: &FrameUI,
    queue_items: &Rc<RefCell<Vec<new_queue::QueueItemDraft>>>,
    queue_item_uis: &Rc<RefCell<Vec<QueueItemUI>>>,
    project_state: &ProjectState,
    status_bar: StatusBar,
    path: &Path,
) {
    match project::storage::load(path) {
        Ok(project_file) => {
            clear_queue(frame_ui, queue_items, queue_item_uis);
            frame_ui.work_dir_text.set_value(&project_file.work_dir);
            let title = clean_project_title(&project_file.title);

            for album in project_file.albums {
                let item = new_queue::QueueItemDraft::from(album);
                add_queue_item_from_project(
                    frame_ui,
                    queue_items,
                    queue_item_uis,
                    item,
                    project_state.clone(),
                    status_bar,
                );
            }

            project_state.set_clean_project(title, path.to_path_buf());
            update_project_title_bar(frame_ui, project_state);
            sync_queue_display(frame_ui, queue_items, queue_item_uis);
            save_recent_project(frame_ui, path);
            frame_ui
                .main_status
                .set_status_text(&format!("Opened project: {}", path.display()), 0);
        }
        Err(err) => show_project_error(
            &frame_ui.main_frame,
            "Open project failed",
            &format!("Failed to open project.\n\n{err}"),
        ),
    }
}

fn save_project(
    frame_ui: &FrameUI,
    queue_items: &Rc<RefCell<Vec<new_queue::QueueItemDraft>>>,
    project_state: &ProjectState,
    save_as: bool,
) -> bool {
    let path = if save_as {
        choose_save_project_file(&frame_ui.main_frame, project_state.path.borrow().as_deref())
    } else {
        project_state
            .path
            .borrow()
            .clone()
            .or_else(|| choose_save_project_file(&frame_ui.main_frame, None))
    };

    let Some(path) = path else {
        return false;
    };

    let project_file = build_project_file(frame_ui, queue_items, project_state);
    match project::storage::save(&path, &project_file) {
        Ok(()) => {
            project_state.mark_clean_saved(path.clone());
            update_project_title_bar(frame_ui, project_state);
            save_recent_project(frame_ui, &path);
            frame_ui
                .main_status
                .set_status_text(&format!("Saved project: {}", path.display()), 0);
            true
        }
        Err(err) => {
            show_project_error(
                &frame_ui.main_frame,
                "Save project failed",
                &format!("Failed to save project.\n\n{err}"),
            );
            false
        }
    }
}

fn build_project_file(
    frame_ui: &FrameUI,
    queue_items: &Rc<RefCell<Vec<new_queue::QueueItemDraft>>>,
    project_state: &ProjectState,
) -> project::ProjectFile {
    let title = project_state.title();
    let albums = queue_items
        .borrow()
        .iter()
        .map(project::ProjectAlbum::from)
        .collect();

    project::ProjectFile::new(title, frame_ui.work_dir_text.get_value(), albums)
}

fn add_queue_item_from_project(
    frame_ui: &FrameUI,
    queue_items: &Rc<RefCell<Vec<new_queue::QueueItemDraft>>>,
    queue_item_uis: &Rc<RefCell<Vec<QueueItemUI>>>,
    item: new_queue::QueueItemDraft,
    project_state: ProjectState,
    status_bar: StatusBar,
) {
    let item_index = queue_items.borrow().len();
    let queue_item_ui = frame_ui.add_queue_item(
        &item.title,
        &item.artwork_path.to_string_lossy(),
        &item.video_quality,
        &item.audio_display_label(),
    );
    setup_queue_item_edit(
        frame_ui,
        queue_item_ui,
        Rc::clone(queue_items),
        Rc::clone(queue_item_uis),
        item_index,
        project_state,
        status_bar,
    );
    queue_items.borrow_mut().push(item);
    queue_item_uis.borrow_mut().push(queue_item_ui);
}

fn clear_queue(
    frame_ui: &FrameUI,
    queue_items: &Rc<RefCell<Vec<new_queue::QueueItemDraft>>>,
    queue_item_uis: &Rc<RefCell<Vec<QueueItemUI>>>,
) {
    queue_items.borrow_mut().clear();
    for item_ui in queue_item_uis.borrow_mut().drain(..) {
        frame_ui.remove_queue_item(item_ui);
    }
}

fn save_recent_project(frame_ui: &FrameUI, path: &Path) {
    let mut config = config::load();
    project::recent::add_recent_project(&mut config.recent_projects, path);
    config.last_project_path = Some(path.to_string_lossy().to_string());
    if let Err(err) = config::save(&config) {
        log::warn!("Failed to save recent projects: {err}");
    }
    frame_ui.refresh_menu_bar();
}

fn clear_last_project_path() {
    let mut config = config::load();
    config.last_project_path = None;
    if let Err(err) = config::save(&config) {
        log::warn!("Failed to clear last project path: {err}");
    }
}

fn load_last_project(
    frame_ui: &FrameUI,
    queue_items: &Rc<RefCell<Vec<new_queue::QueueItemDraft>>>,
    queue_item_uis: &Rc<RefCell<Vec<QueueItemUI>>>,
    project_state: &ProjectState,
    status_bar: StatusBar,
) {
    let Some(path) = config::load().last_project_path.map(PathBuf::from) else {
        return;
    };

    if path.is_file() {
        open_project(
            frame_ui,
            queue_items,
            queue_item_uis,
            project_state,
            status_bar,
            &path,
        );
    } else {
        clear_last_project_path();
    }
}

fn update_project_title_bar(frame_ui: &FrameUI, project_state: &ProjectState) {
    let title = project_state.title();
    let dirty_prefix = if project_state.is_dirty() { "*" } else { "" };
    frame_ui.main_frame.set_title(&format!(
        "{dirty_prefix}{title} - mu2vid v{}",
        env!("CARGO_PKG_VERSION")
    ));
}

fn mark_project_dirty(frame_ui: &FrameUI, project_state: &ProjectState) {
    if project_state.mark_dirty() {
        update_project_title_bar(frame_ui, project_state);
    }
}

fn clean_project_title(title: &str) -> String {
    let title = title.trim();
    if title.is_empty() {
        DEFAULT_PROJECT_TITLE.to_string()
    } else {
        title.to_string()
    }
}

fn change_project_title(frame_ui: &FrameUI, project_state: &ProjectState) {
    let current_title = project_state.title();
    let Some(title) = prompt_project_title(&frame_ui.main_frame, &current_title) else {
        return;
    };

    let title = clean_project_title(&title);
    if title == current_title {
        return;
    }

    project_state.set_title(title);
    mark_project_dirty(frame_ui, project_state);
}

fn choose_project_file(parent: &Frame) -> Option<PathBuf> {
    let dialog = FileDialog::builder(parent)
        .with_message("Open project")
        .with_wildcard("mu2vid project (*.toml)|*.toml")
        .with_style(FileDialogStyle::Open | FileDialogStyle::FileMustExist)
        .build();

    if dialog.show_modal() == ID_OK {
        dialog.get_path().map(PathBuf::from)
    } else {
        None
    }
}

fn choose_save_project_file(parent: &Frame, current_path: Option<&Path>) -> Option<PathBuf> {
    let mut builder = FileDialog::builder(parent)
        .with_message("Save project")
        .with_wildcard("mu2vid project (*.toml)|*.toml")
        .with_style(FileDialogStyle::Save | FileDialogStyle::OverwritePrompt);

    if let Some(path) = current_path {
        if let Some(parent) = path.parent().and_then(|value| value.to_str()) {
            builder = builder.with_default_dir(parent);
        }
        if let Some(file_name) = path.file_name().and_then(|value| value.to_str()) {
            builder = builder.with_default_file(file_name);
        }
    } else {
        builder = builder.with_default_file("untitled.toml");
    }

    let dialog = builder.build();
    if dialog.show_modal() != ID_OK {
        return None;
    }

    dialog
        .get_path()
        .map(|path| ensure_toml_extension(PathBuf::from(path)))
}

fn ensure_toml_extension(mut path: PathBuf) -> PathBuf {
    if path.extension().is_none() {
        path.set_extension("toml");
    }
    path
}

fn show_project_error(parent: &Frame, title: &str, message: &str) {
    let dialog = MessageDialog::builder(parent, message, title)
        .with_style(
            MessageDialogStyle::OK | MessageDialogStyle::IconWarning | MessageDialogStyle::Centre,
        )
        .build();

    dialog.show_modal();
}

fn confirm_quit_if_dirty(
    frame_ui: &FrameUI,
    project_state: &ProjectState,
    queue_items: &Rc<RefCell<Vec<new_queue::QueueItemDraft>>>,
    status_bar: StatusBar,
) -> bool {
    if !project_state.is_dirty() {
        return true;
    }

    match show_unsaved_project_dialog(&frame_ui.main_frame) {
        ID_CANCEL => false,
        ID_QUIT_WITHOUT_SAVE => {
            project_state.clear_dirty();
            update_project_title_bar(frame_ui, project_state);
            true
        }
        ID_OK => save_project(frame_ui, queue_items, project_state, false),
        _ => {
            status_bar.set_status_text("Quit cancelled", 0);
            false
        }
    }
}

fn show_unsaved_project_dialog(parent: &Frame) -> i32 {
    let dialog = Dialog::builder(parent, "Unsaved project").build();
    let sizer = BoxSizer::builder(Orientation::Vertical).build();

    let message = StaticText::builder(&dialog)
        .with_label("Unsaved project, do you want to quit?")
        .build();
    sizer.add(&message, 0, SizerFlag::Expand | SizerFlag::All, 12);

    let button_sizer = BoxSizer::builder(Orientation::Horizontal).build();
    let cancel_button = Button::builder(&dialog)
        .with_id(ID_CANCEL)
        .with_label("Cancel")
        .build();
    let quit_button = Button::builder(&dialog)
        .with_id(ID_QUIT_WITHOUT_SAVE)
        .with_label("Quit")
        .build();
    let save_button = Button::builder(&dialog)
        .with_id(ID_OK)
        .with_label("Save then quit")
        .build();

    let dialog_for_cancel = dialog;
    cancel_button.on_click(move |_| {
        dialog_for_cancel.end_modal(ID_CANCEL);
    });
    let dialog_for_quit = dialog;
    quit_button.on_click(move |_| {
        dialog_for_quit.end_modal(ID_QUIT_WITHOUT_SAVE);
    });
    let dialog_for_save = dialog;
    save_button.on_click(move |_| {
        dialog_for_save.end_modal(ID_OK);
    });

    button_sizer.add(&save_button, 0, SizerFlag::Right, 6);
    button_sizer.add(&quit_button, 0, SizerFlag::Right, 6);
    button_sizer.add(&cancel_button, 0, SizerFlag::Right, 6);

    sizer.add_sizer(&button_sizer, 0, SizerFlag::AlignRight | SizerFlag::All, 12);

    dialog.set_affirmative_id(ID_OK);
    dialog.set_escape_id(ID_CANCEL);
    save_button.set_default();
    dialog.set_sizer_and_fit(sizer, true);

    dialog.show_modal()
}

fn setup_queue_item_edit(
    frame_ui: &FrameUI,
    queue_item_ui: QueueItemUI,
    queue_items: Rc<RefCell<Vec<new_queue::QueueItemDraft>>>,
    queue_item_uis: Rc<RefCell<Vec<QueueItemUI>>>,
    _item_index: usize,
    project_state: ProjectState,
    status_bar: StatusBar,
) {
    let frame_ui = frame_ui.clone();
    let frame_ui_for_up = frame_ui.clone();
    let queue_items_for_up = Rc::clone(&queue_items);
    let queue_item_uis_for_up = Rc::clone(&queue_item_uis);
    let project_state_for_up = project_state.clone();
    queue_item_ui.up_button.on_click(move |_| {
        let Some(item_index) = find_queue_item_index(&queue_item_uis_for_up, queue_item_ui) else {
            return;
        };

        if item_index == 0 || item_index >= queue_items_for_up.borrow().len() {
            return;
        }

        queue_items_for_up
            .borrow_mut()
            .swap(item_index, item_index - 1);
        sync_queue_display(
            &frame_ui_for_up,
            &queue_items_for_up,
            &queue_item_uis_for_up,
        );
        frame_ui_for_up
            .main_status
            .set_status_text("Queue moved up", 0);
        mark_project_dirty(&frame_ui_for_up, &project_state_for_up);
    });

    let frame_ui_for_down = frame_ui.clone();
    let queue_items_for_down = Rc::clone(&queue_items);
    let queue_item_uis_for_down = Rc::clone(&queue_item_uis);
    let project_state_for_down = project_state.clone();
    queue_item_ui.down_button.on_click(move |_| {
        let Some(item_index) = find_queue_item_index(&queue_item_uis_for_down, queue_item_ui)
        else {
            return;
        };

        if item_index + 1 >= queue_items_for_down.borrow().len() {
            return;
        }

        queue_items_for_down
            .borrow_mut()
            .swap(item_index, item_index + 1);
        sync_queue_display(
            &frame_ui_for_down,
            &queue_items_for_down,
            &queue_item_uis_for_down,
        );
        frame_ui_for_down
            .main_status
            .set_status_text("Queue moved down", 0);
        mark_project_dirty(&frame_ui_for_down, &project_state_for_down);
    });

    let frame_ui_for_delete = frame_ui.clone();
    let queue_items_for_delete = Rc::clone(&queue_items);
    let queue_item_uis_for_delete = Rc::clone(&queue_item_uis);
    let project_state_for_delete = project_state.clone();
    queue_item_ui.delete_button.on_click(move |_| {
        let Some(item_index) = find_queue_item_index(&queue_item_uis_for_delete, queue_item_ui)
        else {
            return;
        };

        let Some(item) = queue_items_for_delete.borrow().get(item_index).cloned() else {
            return;
        };

        if !confirm_remove_queue(&frame_ui_for_delete.main_frame, &item.title) {
            return;
        }

        queue_items_for_delete.borrow_mut().remove(item_index);
        let item_ui = queue_item_uis_for_delete.borrow_mut().remove(item_index);
        frame_ui_for_delete.remove_queue_item(item_ui);
        sync_queue_display(
            &frame_ui_for_delete,
            &queue_items_for_delete,
            &queue_item_uis_for_delete,
        );
        frame_ui_for_delete
            .main_status
            .set_status_text(&format!("Removed queue: {}", item.title), 0);
        mark_project_dirty(&frame_ui_for_delete, &project_state_for_delete);
    });

    let project_state_for_edit = project_state.clone();
    queue_item_ui.edit_button.on_click(move |_| {
        let Some(item_index) = find_queue_item_index(&queue_item_uis, queue_item_ui) else {
            return;
        };

        let Some(item) = queue_items.borrow().get(item_index).cloned() else {
            return;
        };

        let queue_items = Rc::clone(&queue_items);
        let queue_item_uis = Rc::clone(&queue_item_uis);
        let queue_item_ui = queue_item_ui;
        let frame_ui = frame_ui.clone();
        let project_state = project_state_for_edit.clone();
        let on_save = Rc::new(move |updated_item: new_queue::QueueItemDraft| {
            if let Some(item) = queue_items.borrow_mut().get_mut(item_index) {
                *item = updated_item.clone();
            }

            queue_item_ui.title_text.set_label(&updated_item.title);
            queue_item_ui.quality_text.set_label(&format!(
                "Video: {} | Audio: {}",
                updated_item.video_quality,
                updated_item.audio_display_label()
            ));
            frame_ui.update_queue_item_artwork(
                queue_item_ui.cover_bitmap,
                &updated_item.artwork_path.to_string_lossy(),
            );
            frame_ui
                .main_status
                .set_status_text(&format!("Updated queue: {}", updated_item.title), 0);
            sync_queue_display(&frame_ui, &queue_items, &queue_item_uis);
            mark_project_dirty(&frame_ui, &project_state);
        });

        new_queue::show_edit(status_bar, item, on_save);
    });
}

fn find_queue_item_index(
    queue_item_uis: &Rc<RefCell<Vec<QueueItemUI>>>,
    queue_item_ui: QueueItemUI,
) -> Option<usize> {
    queue_item_uis
        .borrow()
        .iter()
        .position(|item| item.panel.handle_ptr() == queue_item_ui.panel.handle_ptr())
}

fn sync_queue_display(
    frame_ui: &FrameUI,
    queue_items: &Rc<RefCell<Vec<new_queue::QueueItemDraft>>>,
    queue_item_uis: &Rc<RefCell<Vec<QueueItemUI>>>,
) {
    let queue_items = queue_items.borrow();
    let queue_item_uis = queue_item_uis.borrow();
    let items = queue_item_uis
        .iter()
        .zip(queue_items.iter())
        .map(|(item_ui, item)| {
            (
                *item_ui,
                item.title.clone(),
                item.artwork_path.to_string_lossy().to_string(),
                item.video_quality.clone(),
                item.audio_display_label(),
            )
        })
        .collect::<Vec<_>>();

    frame_ui.sync_queue_items(&items);
}

fn confirm_remove_queue(parent: &Frame, title: &str) -> bool {
    let dialog = MessageDialog::builder(
        parent,
        &format!("Do you want to remove \"{title}\" from queue?"),
        "Remove queue",
    )
    .with_style(
        MessageDialogStyle::YesNo | MessageDialogStyle::IconQuestion | MessageDialogStyle::Centre,
    )
    .build();

    dialog.show_modal() == ID_YES
}

fn choose_work_dir(parent: &Frame, current_path: &str) -> Option<String> {
    let dialog = DirDialog::builder(parent, "Choose work dir", current_path)
        .with_style((DirDialogStyle::Default | DirDialogStyle::MustExist).bits())
        .build();

    if dialog.show_modal() == ID_OK {
        dialog.get_path()
    } else {
        None
    }
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
