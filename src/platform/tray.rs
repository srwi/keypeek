use image::load_from_memory;
use std::process;
use std::sync::Arc;
use std::thread;
use tray_icon::{
    menu::{Menu, MenuEvent, MenuItem, PredefinedMenuItem},
    Icon, TrayIcon, TrayIconBuilder,
};

/// Keeps the tray icon alive for the lifetime of the app.
///
/// On Linux the icon lives on a dedicated GTK thread: libappindicator
/// registers the StatusNotifierItem over DBus via the glib main loop, so the
/// tray only works if `gtk::main()` runs on the thread that created it.
pub struct Tray {
    #[cfg(not(target_os = "linux"))]
    _icon: TrayIcon,
}

// Windows convention is a monochrome glyph matching the taskbar theme.
#[cfg(target_os = "windows")]
fn icon_bytes() -> &'static [u8] {
    let light_theme = winreg::RegKey::predef(winreg::enums::HKEY_CURRENT_USER)
        .open_subkey(r"Software\Microsoft\Windows\CurrentVersion\Themes\Personalize")
        .and_then(|key| key.get_value::<u32, _>("SystemUsesLightTheme"))
        .map(|value| value != 0)
        .unwrap_or(false);
    if light_theme {
        include_bytes!("../../resources/tray-icon-black.png")
    } else {
        include_bytes!("../../resources/tray-icon-white.png")
    }
}

// Monochrome template image; macOS tints it to match the menu bar.
#[cfg(target_os = "macos")]
fn icon_bytes() -> &'static [u8] {
    include_bytes!("../../resources/tray-icon-template.png")
}

#[cfg(not(any(target_os = "windows", target_os = "macos")))]
fn icon_bytes() -> &'static [u8] {
    include_bytes!("../../resources/icon.ico")
}

fn create_icon() -> Icon {
    let icon = load_from_memory(icon_bytes())
        .expect("Failed to load icon.")
        .into_rgba8();

    let (width, height) = icon.dimensions();
    Icon::from_rgba(icon.into_raw(), width, height).expect("Failed to create icon.")
}

fn build_tray_icon() -> TrayIcon {
    let menu = Menu::new();
    menu.append_items(&[
        &MenuItem::with_id("settings", "Settings…", true, None),
        &PredefinedMenuItem::separator(),
        &MenuItem::with_id("quit", "Quit", true, None),
    ])
    .expect("Failed to append menu items.");

    let builder = TrayIconBuilder::new()
        .with_menu(Box::new(menu))
        .with_icon(create_icon())
        .with_tooltip("KeyPeek");

    // Left click opens the settings instead (see `create_tray_icon`).
    #[cfg(target_os = "windows")]
    let builder = builder.with_menu_on_left_click(false);

    #[cfg(target_os = "macos")]
    let builder = builder.with_icon_as_template(true);

    builder.build().unwrap()
}

pub fn create_tray_icon(on_settings: Arc<dyn Fn() + Send + Sync>) -> Tray {
    thread::spawn({
        let on_settings = on_settings.clone();
        move || {
            while let Ok(event) = MenuEvent::receiver().recv() {
                match event.id.0.as_str() {
                    "settings" => on_settings(),
                    "quit" => process::exit(0),
                    _ => {}
                }
            }
        }
    });

    // Left-clicking the icon opens the settings, as is conventional on Windows.
    #[cfg(target_os = "windows")]
    thread::spawn(move || {
        use tray_icon::{MouseButton, MouseButtonState, TrayIconEvent};
        while let Ok(event) = TrayIconEvent::receiver().recv() {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                on_settings();
            }
        }
    });

    #[cfg(target_os = "linux")]
    {
        thread::spawn(|| {
            gtk::init().expect("Failed to initialize GTK. Is a display available?");
            let _icon = build_tray_icon();
            gtk::main();
        });
        Tray {}
    }

    #[cfg(not(target_os = "linux"))]
    Tray {
        _icon: build_tray_icon(),
    }
}
