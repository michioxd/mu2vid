use std::cell::RefCell;
use std::path::PathBuf;
use std::rc::Rc;

pub const DEFAULT_PROJECT_TITLE: &str = "untitled";

#[derive(Clone)]
pub struct ProjectState {
    pub title: Rc<RefCell<String>>,
    pub dirty: Rc<RefCell<bool>>,
    pub path: Rc<RefCell<Option<PathBuf>>>,
    pub skip_youtube_upload: Rc<RefCell<bool>>,
}

impl ProjectState {
    pub fn new() -> Self {
        Self {
            title: Rc::new(RefCell::new(DEFAULT_PROJECT_TITLE.to_string())),
            dirty: Rc::new(RefCell::new(false)),
            path: Rc::new(RefCell::new(None)),
            skip_youtube_upload: Rc::new(RefCell::new(false)),
        }
    }

    pub fn reset(&self) {
        *self.path.borrow_mut() = None;
        *self.title.borrow_mut() = DEFAULT_PROJECT_TITLE.to_string();
        *self.skip_youtube_upload.borrow_mut() = false;
        *self.dirty.borrow_mut() = false;
    }

    pub fn set_clean_project(&self, title: String, path: PathBuf, skip_youtube_upload: bool) {
        *self.title.borrow_mut() = title;
        *self.path.borrow_mut() = Some(path);
        *self.skip_youtube_upload.borrow_mut() = skip_youtube_upload;
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

    pub fn set_skip_youtube_upload(&self, skip: bool) {
        *self.skip_youtube_upload.borrow_mut() = skip;
    }

    pub fn skip_youtube_upload(&self) -> bool {
        *self.skip_youtube_upload.borrow()
    }

    pub fn is_dirty(&self) -> bool {
        *self.dirty.borrow()
    }
}
