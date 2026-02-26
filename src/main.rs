use linuxblaster_control::{BlasterXG6, server};
use tao::{
    event::{Event, WindowEvent},
    event_loop::{ControlFlow, EventLoopBuilder},
    window::{WindowBuilder, Icon as TaoIcon},
    dpi::LogicalSize,
    platform::unix::{WindowExtUnix,},
};
use tray_icon::{
    menu::{Menu, MenuItem, PredefinedMenuItem, MenuEvent},
    TrayIconBuilder,
    TrayIconEvent,
    MouseButton,
};
use wry::{WebViewBuilder, WebViewBuilderExtUnix};
use tracing::Level;

fn main() {
    let start_minimized = std::env::args().any(|a| a == "--minimized");

    // Try to connect to existing instance
    if std::net::TcpStream::connect("127.0.0.1:3311").is_ok() {
        // App is already running, ping it to show window
        if let Ok(mut stream) = std::net::TcpStream::connect("127.0.0.1:3311") {
            use std::io::Write;
            let _ = stream.write_all(b"POST /api/show_window HTTP/1.1\r\nHost: 127.0.0.1\r\nContent-Length: 0\r\nConnection: close\r\n\r\n");
            std::thread::sleep(std::time::Duration::from_millis(100));
        }
        println!("Application is already running. Existing instance brought to front.");
        std::process::exit(0);
    }

    // Autostart configuration checking
    std::thread::spawn(|| {
        let is_flatpak = std::env::var("FLATPAK_ID").is_ok();
        
        let autostart_script = format!(
            "mkdir -p ~/.config/autostart && echo '[Desktop Entry]\nType=Application\nName=Sound Blaster G6X Controller\nExec={} --minimized\nIcon=cc.dreamzone.SoundBlasterG6X\nTerminal=false\nStartupNotify=false\n' > ~/.config/autostart/cc.dreamzone.SoundBlasterG6X.desktop",
            if is_flatpak { "flatpak run cc.dreamzone.SoundBlasterG6X" } else { "soundblaster-g6x" }
        );
        
        if is_flatpak {
            let _ = std::process::Command::new("flatpak-spawn")
                .args(["--host", "bash", "-c", &autostart_script])
                .output();
        } else {
            let _ = std::process::Command::new("bash")
                .args(["-c", &autostart_script])
                .output();
        }
    });

    // Set up event loop first to initialize GTK on Linux
    let event_loop = EventLoopBuilder::new().build();

    tracing_subscriber::fmt()
        .with_max_level(Level::DEBUG)
        .init();

    // Initialize device
    let device = BlasterXG6::init().expect("Failed to initialize device");

    // Spawn web server in a separate thread
    std::thread::spawn(move || {
        let rt = tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .unwrap();
        
        rt.block_on(async {
            server::start_server(device).await;
        });
    });

    // Small delay to let the HTTP server start
    std::thread::sleep(std::time::Duration::from_millis(500));

    // Create the Native Window
    let window = WindowBuilder::new()
        .with_title("Sound Blaster G6X Controller")
        .with_inner_size(LogicalSize::new(800.0, 640.0))
        .with_resizable(false)
        .with_window_icon(Some(load_window_icon()))
        .with_visible(!start_minimized) // Hidden if --minimized (autostart)
        .build(&event_loop)
        .unwrap();

    // Build WebView using GTK container from tao window (Linux-specific)
    let vbox = window.default_vbox().expect("Failed to get GTK vbox from tao window");
    let _webview = WebViewBuilder::new()
        .with_url("http://127.0.0.1:3311")
        .build_gtk(vbox)
        .unwrap();

    // Create system tray menu
    let tray_menu = Menu::new();
    let open_item = MenuItem::new("Open Control Panel", true, None);
    let quit_item = MenuItem::new("Quit", true, None);
    
    tray_menu.append(&open_item).unwrap();
    tray_menu.append(&PredefinedMenuItem::separator()).unwrap();
    tray_menu.append(&quit_item).unwrap();

    let mut builder = TrayIconBuilder::new()
        .with_menu(Box::new(tray_menu))
        .with_tooltip("Sound Blaster G6 Control")
        .with_icon(load_tray_icon());

    #[cfg(target_os = "linux")]
    if std::env::var("FLATPAK_ID").is_ok() {
        if let Ok(cache_home) = std::env::var("XDG_CACHE_HOME") {
            builder = builder.with_temp_dir_path(cache_home);
        } else if let Ok(home) = std::env::var("HOME") {
            builder = builder.with_temp_dir_path(format!("{}/.cache", home));
        }
    }

    let _tray_icon = builder.build().unwrap();

    // Run event loop
    event_loop.run(move |event, _, control_flow| {
        // Wake up event loop 5 times a second to poll tray/menu events
        // otherwise it sleeps forever when the window is hidden
        *control_flow = ControlFlow::WaitUntil(std::time::Instant::now() + std::time::Duration::from_millis(200));

        if linuxblaster_control::server::SHOW_WINDOW_REQUEST.swap(false, std::sync::atomic::Ordering::Relaxed) {
             window.set_visible(true);
             window.set_focus();
             window.request_user_attention(Some(tao::window::UserAttentionType::Critical));
        }

        match event {
            Event::WindowEvent { event, window_id, .. } => {
                if window_id == window.id() {
                    if let WindowEvent::CloseRequested = event {
                        // Minimize to tray (hide the window) instead of exiting process
                        window.set_visible(false);
                    }
                }
            }
            _ => {}
        }

        while let Ok(event) = MenuEvent::receiver().try_recv() {
            if event.id == open_item.id() {
                window.set_visible(true);
                window.set_focus();
                window.request_user_attention(Some(tao::window::UserAttentionType::Critical));
            } else if event.id == quit_item.id() {
                // Immediate clean exit, avoids waiting for background tasks to gracefully shutdown
                std::process::exit(0);
            }
        }

        while let Ok(event) = TrayIconEvent::receiver().try_recv() {
             match event {
                 TrayIconEvent::Click { button: MouseButton::Left, .. } | 
                 TrayIconEvent::DoubleClick { button: MouseButton::Left, .. } => {
                     window.set_visible(true);
                     window.set_focus();
                     window.request_user_attention(Some(tao::window::UserAttentionType::Critical));
                 }
                 _ => {}
             }
        }
    });
}

// Get the raw RGBA image data from embedded assets
fn get_icon_image_data() -> (Vec<u8>, u32, u32) {
    use linuxblaster_control::server::Assets;

    let icon_file = Assets::get("icon.png").expect("Failed to load icon asset");
    let image = image::load_from_memory(&icon_file.data).expect("Failed to parse icon");
    let rgba = image.into_rgba8();
    let (width, height) = rgba.dimensions();
    let rgba = rgba.into_raw();
    
    (rgba, width, height)
}

fn load_tray_icon() -> tray_icon::Icon {
    let (rgba, width, height) = get_icon_image_data();
    tray_icon::Icon::from_rgba(rgba, width, height).expect("Failed to create tray icon")
}

fn load_window_icon() -> TaoIcon {
    let (rgba, width, height) = get_icon_image_data();
    TaoIcon::from_rgba(rgba, width, height).expect("Failed to create window icon")
}
