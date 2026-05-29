use wxdragon::prelude::*;

mod ui;

fn main() {
    SystemOptions::set_option_by_int("msw.no-manifest-check", 1);
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("trace")).init();
    let _ = wxdragon::main(|_handle| {
        ui::main_window::show();
    });
}
