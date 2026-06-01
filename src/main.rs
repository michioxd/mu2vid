#![cfg_attr(target_os = "windows", windows_subsystem = "windows")]

use wxdragon::prelude::*;

mod config;
mod deps;
mod media;
mod project;
mod ui;
mod youtube;

fn main() {
    config::storage::load();
    SystemOptions::set_option_by_int("msw.no-manifest-check", 1);
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("trace")).init();
    let _ = wxdragon::main(|_handle| {
        ui::main_window::show();
    });
}
