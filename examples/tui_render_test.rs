//! Test that renders exactly what the TUI would render and saves it as PNG
use crafter_core::{Session, SessionConfig};
use crafter_core::image_renderer::{ImageRenderer, ImageRendererConfig};
use image::{RgbaImage, Rgba};

fn main() {
    let config = SessionConfig {
        world_size: (48, 20),
        seed: Some(42),
        view_radius: 4,
        ..Default::default()
    };

    let session = Session::new(config);
    let state = session.get_state();

    // EXACTLY what the TUI does in render_state_graphics()
    let view = state.view.as_ref().expect("No view");
    let view_size = view.size() as u32;
    let tile_size = 6u32; // TUI uses tile_size=6

    let renderer_config = ImageRendererConfig {
        tile_size,
        show_status_bars: true,
        apply_lighting: true,
    };

    let renderer = ImageRenderer::new(renderer_config);
    let rgb_bytes = renderer.render_bytes(&state);

    // Dimensions
    let pixel_w = view_size * tile_size;
    let status_bar_height = tile_size;
    let pixel_h = view_size * tile_size + status_bar_height;

    println!("=== TUI Render Test ===");
    println!("view_size: {}", view_size);
    println!("tile_size: {}", tile_size);
    println!("pixel_w: {}, pixel_h: {}", pixel_w, pixel_h);
    println!("RGB bytes: {} (expected: {})", rgb_bytes.len(), pixel_w * pixel_h * 3);

    // Convert RGB to RGBA (same as TUI)
    let rgba_bytes: Vec<u8> = rgb_bytes
        .chunks_exact(3)
        .flat_map(|rgb| [rgb[0], rgb[1], rgb[2], 255])
        .collect();

    println!("RGBA bytes: {} (expected: {})", rgba_bytes.len(), pixel_w * pixel_h * 4);

    // Terminal cell dimensions (super-sampling: 2 pixels per cell)
    let cells_w = pixel_w / 2;
    let cells_h = pixel_h / 2;
    println!("Terminal cells: {}x{}", cells_w, cells_h);
    println!("Total terminal rows needed (with y_start=5): {}", 5 + cells_h);

    // Check status bar row data
    let status_bar_start_row = view_size * tile_size; // Row where status bar starts
    let status_bar_byte_offset = (status_bar_start_row * pixel_w * 3) as usize;
    println!("\nStatus bar starts at pixel row: {}", status_bar_start_row);
    println!("Status bar byte offset in RGB: {}", status_bar_byte_offset);

    if status_bar_byte_offset < rgb_bytes.len() {
        let sample: Vec<u8> = rgb_bytes[status_bar_byte_offset..status_bar_byte_offset.min(rgb_bytes.len()) + 24].to_vec();
        println!("First 24 bytes of status bar row: {:?}", sample);
        println!("(Non-zero values indicate status bar content is present)");
    } else {
        println!("ERROR: Status bar byte offset exceeds RGB data length!");
    }

    // Save as PNG to verify
    if let Some(img) = RgbaImage::from_raw(pixel_w, pixel_h, rgba_bytes.clone()) {
        img.save("/tmp/tui_exact_render.png").expect("Failed to save PNG");
        println!("\nSaved to /tmp/tui_exact_render.png");
    } else {
        println!("ERROR: Failed to create image from RGBA data");
    }

    // Also verify by checking the last few rows
    println!("\n=== Checking last 6 pixel rows (status bar area) ===");
    for row in (pixel_h - 6)..pixel_h {
        let row_start = (row * pixel_w * 3) as usize;
        let row_end = row_start + 18; // First 6 pixels (18 bytes)
        if row_end <= rgb_bytes.len() {
            let sample: Vec<u8> = rgb_bytes[row_start..row_end].to_vec();
            let has_content = sample.iter().any(|&b| b != 0);
            println!("Row {}: {:?} {}", row, sample, if has_content { "✓" } else { "empty" });
        }
    }

    println!("\nHealth: {}, Food: {}, Drink: {}, Energy: {}",
             state.inventory.health, state.inventory.food,
             state.inventory.drink, state.inventory.energy);
}
