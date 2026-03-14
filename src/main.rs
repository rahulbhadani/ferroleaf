mod app;
mod dialog;
mod compiler;
mod editor;
mod file_tree;
mod icons;
mod pdf_viewer;
mod project;
mod synctex;
mod theme;

fn main() -> iced::Result {
    env_logger::Builder::from_env(
        env_logger::Env::default().default_filter_or("warn"),
    ).init();

    iced::application("Ferroleaf", app::Ferroleaf::update, app::Ferroleaf::view)
        .subscription(app::Ferroleaf::subscription)
        .theme(app::Ferroleaf::theme)
        .window(iced::window::Settings {
            size: iced::Size::new(1440.0, 900.0),
            min_size: Some(iced::Size::new(960.0, 600.0)),
            resizable: true,
            decorations: true,
            icon: window_icon(),
            ..Default::default()
        })
        // Cast to &[u8] explicitly to avoid palette crate AsRef ambiguity
        .font(include_bytes!("../assets/fonts/JetBrainsMono-Regular.ttf") as &[u8])
        .font(include_bytes!("../assets/fonts/JetBrainsMono-Bold.ttf") as &[u8])
        .default_font(iced::Font::MONOSPACE)
        .antialiasing(true)
        .run_with(app::Ferroleaf::new)
}

fn window_icon() -> Option<iced::window::Icon> {
    // Encode the SVG icon as a simple 64x64 RGBA bitmap
    // Colours match our pink-brown palette
    let size = 64u32;
    let mut pixels = vec![0u8; (size * size * 4) as usize];
    for y in 0..size {
        for x in 0..size {
            let idx = ((y * size + x) * 4) as usize;
            let cx = x as f32 - 32.0;
            let cy = y as f32 - 32.0;
            let r2 = cx * cx + cy * cy;
            if r2 > 30.0 * 30.0 { continue; } // outside circle
            // Background
            pixels[idx]     = 0x2e;  // r
            pixels[idx + 1] = 0x1f;  // g
            pixels[idx + 2] = 0x1f;  // b
            pixels[idx + 3] = 0xff;  // a
            // Leaf ellipse
            let lx = cx / 12.0;
            let ly = (cy - 2.0) / 22.0;
            if lx * lx + ly * ly < 1.0 {
                pixels[idx]     = 0xcc;
                pixels[idx + 1] = 0x6b;
                pixels[idx + 2] = 0x85;
                pixels[idx + 3] = 0xff;
            }
            // Bright spot (stem)
            if cx.abs() < 1.5 && cy > 10.0 && cy < 22.0 {
                pixels[idx]     = 0xbf;
                pixels[idx + 1] = 0x8c;
                pixels[idx + 2] = 0x7a;
                pixels[idx + 3] = 0xff;
            }
        }
    }
    iced::window::icon::from_rgba(pixels, size, size).ok()
}
