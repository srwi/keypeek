use image::load_from_memory;
use std::process;
use std::thread;
use tray_icon::{menu::Menu, menu::MenuEvent, menu::MenuItem, Icon, TrayIcon, TrayIconBuilder};

#[cfg(target_os = "linux")]
use gtk;

fn create_icon() -> Icon {
    const ICON_BYTES: &[u8] = include_bytes!("../resources/icon.ico");

    let icon = load_from_memory(ICON_BYTES)
        .expect("Failed to load icon.")
        .into_rgba8();

    let (width, height) = icon.dimensions();
    Icon::from_rgba(icon.into_raw(), width, height).expect("Failed to create icon.")
}

pub fn create_tray_icon() -> TrayIcon {
    #[cfg(target_os = "linux")]
    gtk::init().expect("Failed to initialize GTK. Is a display available?");

    let quit = MenuItem::new("Quit", true, None);
    let menu = Menu::new();
    menu.append(&quit).expect("Failed to append menu item.");

    let icon = create_icon();

    let tray_icon = TrayIconBuilder::new()
        .with_menu(Box::new(menu))
        .with_icon(icon)
        .with_tooltip("KeyPeek")
        .build()
        .unwrap();

    thread::spawn(move || {
        if MenuEvent::receiver().recv().is_ok() {
            process::exit(0);
        }
    });

    tray_icon
}
