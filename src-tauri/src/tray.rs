use tauri::{
    Manager,
    menu::{Menu, MenuItem, PredefinedMenuItem},
    tray::{TrayIconBuilder, TrayIconEvent, MouseButton, MouseButtonState},
};

/// Toggle window visibility and update tray menu label
fn toggle_window(app: &tauri::AppHandle) {
    log::info!("toggle_window called");
    activate_app();

    match app.get_webview_window("main") {
        Some(window) => {
            let visible = window.is_visible().unwrap_or(false);
            log::info!("Window visible={}", visible);

            if visible {
                let _ = window.hide();
                log::info!("Window hidden");
            } else {
                let _ = window.unminimize();
                let _ = window.show();
                let _ = window.set_focus();
                log::info!("Window shown");
            }

            // Rebuild menu with updated label
            let new_label = if visible { "Show" } else { "Hide" };
            if let Ok(new_menu) = build_menu(app, new_label) {
                if let Some(tray) = app.tray_by_id("perch") {
                    let _ = tray.set_menu(Some(new_menu));
                }
            }
        }
        None => {
            log::error!("get_webview_window(\"main\") returned None");
        }
    }
}

pub fn build_menu(app: &tauri::AppHandle, show_label: &str) -> Result<Menu<tauri::Wry>, Box<dyn std::error::Error>> {
    let show_item = MenuItem::with_id(app, "show", show_label, true, None::<&str>)?;
    let separator = PredefinedMenuItem::separator(app)?;
    let quit_item = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&show_item, &separator, &quit_item])?;
    Ok(menu)
}

/// Bring the app to the foreground (needed on macOS with accessory activation policy)
fn activate_app() {
    #[cfg(target_os = "macos")]
    {
        use std::ffi::c_void;

        extern "C" {
            fn objc_getClass(name: *const std::os::raw::c_char) -> *mut c_void;
            fn sel_registerName(name: *const std::os::raw::c_char) -> *mut c_void;
        }

        type MsgSendFn = unsafe extern "C" fn(*mut c_void, *mut c_void) -> *mut c_void;
        type MsgSendFnBool = unsafe extern "C" fn(*mut c_void, *mut c_void, bool);

        unsafe {
            let raw_ptr = libc::dlsym(libc::RTLD_DEFAULT, "objc_msgSend\0".as_ptr() as *const _);
            if raw_ptr.is_null() { return; }

            let cls_name = std::ffi::CString::new("NSApplication").unwrap();
            let sel_shared = std::ffi::CString::new("sharedApplication").unwrap();
            let cls = objc_getClass(cls_name.as_ptr());
            let msg_send: MsgSendFn = std::mem::transmute(raw_ptr);
            let app = msg_send(cls, sel_registerName(sel_shared.as_ptr()));

            if !app.is_null() {
                let sel_activate = std::ffi::CString::new("activateIgnoringOtherApps:").unwrap();
                let activate_fn: MsgSendFnBool = std::mem::transmute(raw_ptr);
                activate_fn(app, sel_registerName(sel_activate.as_ptr()), true);
            }
        }
    }
}

/// Setup the system tray icon with menu
pub fn setup_tray(app: &tauri::App) -> Result<(), Box<dyn std::error::Error>> {
    let menu = build_menu(app.handle(), "Show")?;

    let icon = tauri::image::Image::from_path("icons/tray-icon.png")
        .unwrap_or_else(|_| app.default_window_icon().unwrap().clone());

    let _tray = TrayIconBuilder::with_id("perch")
        .icon(icon)
        .menu(&menu)
        .tooltip("Perch - opencode companion")
        .show_menu_on_left_click(true)
        .on_menu_event(move |app, event| {
            match event.id().as_ref() {
                "show" => {
                    log::info!("Show/Hide toggled");
                    toggle_window(app);
                }
                "quit" => {
                    log::info!("Quit menu item clicked");
                    app.exit(0);
                }
                _ => {
                    log::warn!("Unhandled menu event: {}", event.id().as_ref());
                }
            }
        })
        .on_tray_icon_event(|tray, event| {
            match event {
                TrayIconEvent::Click {
                    button: MouseButton::Left,
                    button_state: MouseButtonState::Up,
                    ..
                } => {
                    log::info!("Tray icon left-clicked");
                    toggle_window(tray.app_handle());
                }
                _ => {}
            }
        })
        .build(app)?;

    log::info!("System tray initialized");
    Ok(())
}

/// Hide dock icon on macOS (call after tray is set up)
pub fn hide_dock_icon() {
    #[cfg(target_os = "macos")]
    {
        use std::ffi::c_void;

        extern "C" {
            fn objc_getClass(name: *const std::os::raw::c_char) -> *mut c_void;
            fn sel_registerName(name: *const std::os::raw::c_char) -> *mut c_void;
        }

        type MsgSendFn = unsafe extern "C" fn(*mut c_void, *mut c_void) -> *mut c_void;
        type MsgSendFnI64 = unsafe extern "C" fn(*mut c_void, *mut c_void, i64);

        unsafe {
            let raw_ptr = libc::dlsym(libc::RTLD_DEFAULT, "objc_msgSend\0".as_ptr() as *const _);
            if raw_ptr.is_null() { return; }

            let cls_name = std::ffi::CString::new("NSApplication").unwrap();
            let sel_name = std::ffi::CString::new("sharedApplication").unwrap();
            let cls = objc_getClass(cls_name.as_ptr());
            let sel = sel_registerName(sel_name.as_ptr());
            let msg_send: MsgSendFn = std::mem::transmute(raw_ptr);
            let app = msg_send(cls, sel);

            if !app.is_null() {
                let set_sel = std::ffi::CString::new("setActivationPolicy:").unwrap();
                let set_fn: MsgSendFnI64 = std::mem::transmute(raw_ptr);
                set_fn(app, sel_registerName(set_sel.as_ptr()), 2);
                log::info!("Dock icon hidden (macOS accessory mode)");
            }
        }
    }
}
