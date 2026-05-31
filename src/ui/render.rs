use crate::deps::encoder;
use crate::ui::main_window_ui::{FrameUI, QueueItemUI};
use crate::ui::new_queue;
use std::cell::RefCell;
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::sync::{
    Arc, Mutex,
    atomic::{AtomicBool, Ordering},
};
use wxdragon::prelude::*;

thread_local! {
    static ACTIVE_RENDER_UI: RefCell<Option<ActiveRenderUi>> = const { RefCell::new(None) };
}

#[derive(Clone)]
struct ActiveRenderUi {
    frame_ui: FrameUI,
    queue_items: Rc<RefCell<Vec<new_queue::QueueItemDraft>>>,
    queue_item_uis: Vec<QueueItemUI>,
    is_started: Arc<AtomicBool>,
    cancel_render: Arc<Mutex<Option<Arc<AtomicBool>>>>,
}

pub(super) fn update_start_stop_buttons(frame_ui: &FrameUI, is_started: bool) {
    frame_ui
        .start_button
        .enable(!is_started && is_valid_work_dir(&frame_ui.work_dir_text.get_value()));
    frame_ui.stop_button.enable(is_started);
}

pub(super) fn start_render_thread(
    frame_ui: &FrameUI,
    queue_items: Rc<RefCell<Vec<new_queue::QueueItemDraft>>>,
    queue_item_uis: Rc<RefCell<Vec<QueueItemUI>>>,
    is_started: Arc<AtomicBool>,
    cancel_render: Arc<Mutex<Option<Arc<AtomicBool>>>>,
) {
    let cancel = Arc::new(AtomicBool::new(false));
    if let Ok(mut guard) = cancel_render.lock() {
        *guard = Some(Arc::clone(&cancel));
    }

    for (item, item_ui) in queue_items
        .borrow()
        .iter()
        .zip(queue_item_uis.borrow().iter())
    {
        item_ui.status_text.set_label(item.status_label());
        item_ui.progress_gauge.set_value(item.progress_value());
        item_ui.edit_button.enable(false);
        item_ui.delete_button.enable(false);
        item_ui.skip_button.enable(false);
        item_ui.up_button.enable(false);
        item_ui.down_button.enable(false);
    }
    frame_ui
        .total_progress_gauge
        .set_value(total_progress_from_status(&queue_items.borrow()));

    let request = encoder::RenderRequest {
        work_dir: PathBuf::from(frame_ui.work_dir_text.get_value()),
        queues: queue_items.borrow().clone(),
    };
    let is_started_for_event = Arc::clone(&is_started);
    let cancel_render_for_event = Arc::clone(&cancel_render);

    ACTIVE_RENDER_UI.with(|state| {
        *state.borrow_mut() = Some(ActiveRenderUi {
            frame_ui: frame_ui.clone(),
            queue_items: Rc::clone(&queue_items),
            queue_item_uis: queue_item_uis.borrow().clone(),
            is_started: Arc::clone(&is_started_for_event),
            cancel_render: Arc::clone(&cancel_render_for_event),
        });
    });

    std::thread::spawn(move || {
        encoder::render_project(request, cancel, move |event| {
            wxdragon::call_after(Box::new(move || {
                let should_clear = is_terminal_render_event(&event);
                ACTIVE_RENDER_UI.with(|state| {
                    if let Some(render_ui) = state.borrow().as_ref() {
                        handle_render_event(
                            &render_ui.frame_ui,
                            &render_ui.queue_items,
                            &render_ui.queue_item_uis,
                            &render_ui.is_started,
                            &render_ui.cancel_render,
                            event,
                        );
                    }

                    if should_clear {
                        *state.borrow_mut() = None;
                    }
                });
            }));
        });
    });
}

fn is_terminal_render_event(event: &encoder::RenderEvent) -> bool {
    matches!(
        event,
        encoder::RenderEvent::Finished
            | encoder::RenderEvent::Cancelled
            | encoder::RenderEvent::Error { .. }
    )
}

fn handle_render_event(
    frame_ui: &FrameUI,
    queue_items: &Rc<RefCell<Vec<new_queue::QueueItemDraft>>>,
    queue_item_uis: &[QueueItemUI],
    is_started: &Arc<AtomicBool>,
    cancel_render: &Arc<Mutex<Option<Arc<AtomicBool>>>>,
    event: encoder::RenderEvent,
) {
    match event {
        encoder::RenderEvent::QueueStarted { index, title } => {
            if let Some(item_ui) = queue_item_uis.get(index) {
                item_ui.status_text.set_label("Status: rendering");
                item_ui.progress_gauge.set_value(0);
            }
            frame_ui
                .main_status
                .set_status_text(&format!("Rendering: {title}"), 0);
        }
        encoder::RenderEvent::Progress {
            index,
            queue_percent,
            total_percent,
        } => {
            let total_percent = total_percent.min(100);
            if let Some(item_ui) = queue_item_uis.get(index) {
                item_ui
                    .progress_gauge
                    .set_value(queue_percent.min(100) as i32);
                item_ui
                    .status_text
                    .set_label(&format!("Status: rendering {queue_percent}%"));
            }
            frame_ui
                .total_progress_gauge
                .set_value(total_percent as i32);
            frame_ui
                .main_status
                .set_status_text(&format!("Total progress: {total_percent}%"), 0);
        }
        encoder::RenderEvent::QueueFinished { index, output_path } => {
            if let Some(item) = queue_items.borrow_mut().get_mut(index) {
                item.render_status = new_queue::QueueRenderStatus::Finished;
            }
            if let Some(item_ui) = queue_item_uis.get(index) {
                item_ui.status_text.set_label("Status: finished");
                item_ui.progress_gauge.set_value(100);
            }
            frame_ui
                .main_status
                .set_status_text(&format!("Rendered: {}", output_path.display()), 0);
        }
        encoder::RenderEvent::Finished => {
            frame_ui.total_progress_gauge.set_value(100);
            finish_render(
                frame_ui,
                queue_item_uis,
                is_started,
                cancel_render,
                "Render finished",
            );
        }
        encoder::RenderEvent::Cancelled => finish_render(
            frame_ui,
            queue_item_uis,
            is_started,
            cancel_render,
            "Render cancelled",
        ),
        encoder::RenderEvent::Error { index, message } => {
            if let Some(index) = index {
                if let Some(item) = queue_items.borrow_mut().get_mut(index) {
                    item.render_status = new_queue::QueueRenderStatus::Error;
                }
                if let Some(item_ui) = queue_item_uis.get(index) {
                    item_ui.status_text.set_label("Status: error");
                }
            }
            finish_render(
                frame_ui,
                queue_item_uis,
                is_started,
                cancel_render,
                &format!("Render failed: {message}"),
            );
        }
    }
}

fn finish_render(
    frame_ui: &FrameUI,
    queue_item_uis: &[QueueItemUI],
    is_started: &Arc<AtomicBool>,
    cancel_render: &Arc<Mutex<Option<Arc<AtomicBool>>>>,
    status: &str,
) {
    is_started.store(false, Ordering::Relaxed);
    if let Ok(mut guard) = cancel_render.lock() {
        *guard = None;
    }
    update_start_stop_buttons(frame_ui, false);

    set_queue_actions_enabled(frame_ui, queue_item_uis);
    frame_ui.main_status.set_status_text(status, 0);
}

fn is_valid_work_dir(path: &str) -> bool {
    let path = path.trim();

    !path.is_empty() && Path::new(path).is_dir()
}

fn total_progress_from_status(queue_items: &[new_queue::QueueItemDraft]) -> i32 {
    if queue_items.is_empty() {
        return 0;
    }

    let finished = queue_items
        .iter()
        .filter(|item| item.render_status.is_finished())
        .count();
    ((finished * 100) / queue_items.len()) as i32
}

fn set_queue_actions_enabled(frame_ui: &FrameUI, queue_item_uis: &[QueueItemUI]) {
    for (index, item_ui) in queue_item_uis.iter().enumerate() {
        item_ui.edit_button.enable(true);
        item_ui.delete_button.enable(true);
        item_ui.skip_button.enable(true);
        item_ui.up_button.enable(index > 0);
        item_ui.down_button.enable(index + 1 < queue_item_uis.len());
    }

    frame_ui.queue_list_panel.layout();
}
