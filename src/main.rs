use linuxblaster_control::{BlasterXG6, server};
use tao::{
    event::{Event, WindowEvent},
    event_loop::{ControlFlow, EventLoopBuilder},
    window::{WindowBuilder, Icon as TaoIcon},
    dpi::LogicalSize,
    platform::unix::WindowExtUnix,
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
    // Parse CLI args
    let start_minimized = std::env::args().any(|a| a == "--minimized");

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
        .with_inner_size(LogicalSize::new(1280.0, 800.0))
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

    let _tray_icon = TrayIconBuilder::new()
        .with_menu(Box::new(tray_menu))
        .with_tooltip("Sound Blaster G6 Control")
        .with_icon(load_tray_icon())
        .build()
        .unwrap();

    // Run event loop
    event_loop.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::Wait;

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

        if let Ok(event) = MenuEvent::receiver().try_recv() {
            if event.id == open_item.id() {
                window.set_visible(true);
                window.set_focus();
            } else if event.id == quit_item.id() {
                *control_flow = ControlFlow::Exit;
            }
        }

        if let Ok(event) = TrayIconEvent::receiver().try_recv() {
             match event {
                 TrayIconEvent::Click { button: MouseButton::Left, .. } | 
                 TrayIconEvent::DoubleClick { button: MouseButton::Left, .. } => {
                     window.set_visible(true);
                     window.set_focus();
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
