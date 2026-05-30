use std::cell::RefCell;
use std::path::PathBuf;
use std::rc::Rc;

pub const DEFAULT_PROJECT_TITLE: &str = "untitled";

#[derive(Clone)]
pub struct ProjectState {
    pub title: Rc<RefCell<String>>,
    pub dirty: Rc<RefCell<bool>>,
    pub path: Rc<RefCell<Option<PathBuf>>>,
}

impl ProjectState {
    pub fn new() -> Self {
        Self {
            title: Rc::new(RefCell::new(DEFAULT_PROJECT_TITLE.to_string())),
            dirty: Rc::new(RefCell::new(false)),
            path: Rc::new(RefCell::new(None)),
        }
    }

    pub fn reset(&self) {
        *self.path.borrow_mut() = None;
        *self.title.borrow_mut() = DEFAULT_PROJECT_TITLE.to_string();
        *self.dirty.borrow_mut() = false;
    }

    pub fn set_clean_project(&self, title: String, path: PathBuf) {
        *self.title.borrow_mut() = title;
        *self.path.borrow_mut() = Some(path);
        *self.dirty.borrow_mut() = false;
    }

    pub fn mark_clean_saved(&self, path: PathBuf) {
        *self.path.borrow_mut() = Some(path);
        *self.dirty.borrow_mut() = false;
    }

    pub fn mark_dirty(&self) -> bool {
        let mut dirty = self.dirty.borrow_mut();
        if *dirty {
            false
        } else {
            *dirty = true;
            true
        }
    }

    pub fn clear_dirty(&self) {
        *self.dirty.borrow_mut() = false;
    }

    pub fn set_title(&self, title: String) {
        *self.title.borrow_mut() = title;
    }

    pub fn title(&self) -> String {
        self.title.borrow().clone()
    }

    pub fn is_dirty(&self) -> bool {
        *self.dirty.borrow()
    }
}
