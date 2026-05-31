pub mod model;
pub mod recent;
pub mod storage;

pub use model::{ProjectAlbum, ProjectFile};

use wxdragon::id::ID_HIGHEST;

pub const ID_CHANGE_TITLE: i32 = ID_HIGHEST + 10;
pub const ID_RESET_QUEUE_STATUS: i32 = ID_HIGHEST + 13;
