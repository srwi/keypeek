use image::load_from_memory;
use std::sync::mpsc::{self, Receiver};
use std::thread;
use tray_icon::{
    menu::Menu, menu::MenuEvent, menu::MenuId, menu::MenuItem, Icon, TrayIcon, TrayIconBuilder,
};

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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TrayCommand {
    ShowSettings,
    Quit,
}

pub fn create_tray_icon() -> (TrayIcon, Receiver<TrayCommand>) {
    #[cfg(target_os = "linux")]
    gtk::init().expect("Failed to initialize GTK. Is a display available?");

    let show_settings = MenuItem::with_id("show_settings", "Show Settings", true, None);
    let quit = MenuItem::with_id("quit", "Quit", true, None);
    let menu = Menu::new();
    menu.append(&show_settings)
        .expect("Failed to append menu item.");
    menu.append(&quit).expect("Failed to append menu item.");

    let icon = create_icon();

    let tray_icon = TrayIconBuilder::new()
        .with_menu(Box::new(menu))
        .with_icon(icon)
        .with_tooltip("QMK Layout Helper")
        .build()
        .unwrap();

    let show_settings_id: MenuId = show_settings.id().clone();
    let quit_id: MenuId = quit.id().clone();
    let (tx, rx) = mpsc::channel();

    thread::spawn(move || {
        while let Ok(event) = MenuEvent::receiver().recv() {
            let command = if event.id == show_settings_id {
                Some(TrayCommand::ShowSettings)
            } else if event.id == quit_id {
                Some(TrayCommand::Quit)
            } else {
                None
            };

            if let Some(cmd) = command {
                if tx.send(cmd).is_err() {
                    break;
                }
            }
        }
    });

    (tray_icon, rx)
}
