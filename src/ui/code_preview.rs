use crate::project;
use crate::ui::code_preview_ui::CodePreviewUI;
use wxdragon::prelude::*;
use wxdragon::timer::Timer;

const REFRESH_INTERVAL_MS: i32 = 300;

pub fn show(parent: &Frame, project_file: project::ProjectFile) {
    let preview_ui = CodePreviewUI::new(parent);
    let code_text = preview_ui.code_text;
    let project_file = std::rc::Rc::new(std::cell::RefCell::new(project_file));
    let last_text = std::rc::Rc::new(std::cell::RefCell::new(String::new()));

    refresh_preview(code_text, &project_file.borrow(), &last_text);

    let timer = std::rc::Rc::new(Timer::new(&preview_ui.frame));
    let timer_for_close = std::rc::Rc::clone(&timer);
    preview_ui.frame.on_close(move |event| {
        timer_for_close.stop();
        event.skip(true);
    });

    let code_text_for_timer = code_text;
    let project_file_for_timer = std::rc::Rc::clone(&project_file);
    let last_text_for_timer = std::rc::Rc::clone(&last_text);
    timer.on_tick(move |_| {
        refresh_preview(
            code_text_for_timer,
            &project_file_for_timer.borrow(),
            &last_text_for_timer,
        );
    });
    timer.start(REFRESH_INTERVAL_MS, false);

    preview_ui.frame.show(true);
}

fn refresh_preview(
    code_text: TextCtrl,
    project_file: &project::ProjectFile,
    last_text: &std::rc::Rc<std::cell::RefCell<String>>,
) {
    let text = project::storage::to_toml(project_file)
        .unwrap_or_else(|err| format!("# Failed to render project file preview\n# {err}\n"));

    let mut last_text = last_text.borrow_mut();
    if *last_text == text {
        return;
    }

    let (selection_start, selection_end) = code_text.get_selection();
    let current_pos = code_text.get_insertion_point();

    code_text.set_value(&text);

    let length = code_text.get_last_position();
    code_text.set_selection(selection_start.min(length), selection_end.min(length));
    code_text.set_insertion_point(current_pos.min(length));

    *last_text = text;
}
