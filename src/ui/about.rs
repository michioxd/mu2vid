use wxdragon::prelude::*;

pub fn show() {
    let Some(parent) = wxdragon::app::get_app_instance().and_then(|app| app.get_top_window())
    else {
        return;
    };

    let mut info = AboutDialogInfo::new();
    info.set_name("mu2vid");
    info.set_description("mu2vid");
    info.add_developer("michioxd");
    info.set_website("https://github.com/michioxd/mu2vid");
    info.set_license("GPL v3.0");

    show_about_box(&info, Some(&parent));
}
