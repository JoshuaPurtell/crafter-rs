//! Convert raw RGBA file to PNG for inspection
use image::RgbaImage;
use std::fs;

fn main() {
    // Read the debug info
    let info = fs::read_to_string("/tmp/crafter_tui_frame_debug.txt")
        .expect("No debug file - run TUI first with crafter game");
    println!("Debug info: {}", info.trim());

    // Parse dimensions
    let parts: Vec<&str> = info.split_whitespace().collect();
    let mut width = 0u32;
    let mut height = 0u32;
    for part in &parts {
        if part.starts_with("frame_width=") {
            width = part.trim_start_matches("frame_width=").parse().unwrap();
        }
        if part.starts_with("frame_height=") {
            height = part.trim_start_matches("frame_height=").parse().unwrap();
        }
    }
    println!("Parsed dimensions: {}x{}", width, height);

    // Read RGBA data
    let rgba_data = fs::read("/tmp/crafter_tui_frame.rgba")
        .expect("No RGBA file - run TUI first with crafter game");
    println!("RGBA data size: {} bytes", rgba_data.len());
    println!("Expected size: {} bytes", width * height * 4);

    if rgba_data.len() != (width * height * 4) as usize {
        println!("WARNING: Size mismatch!");
    }

    // Check status bar area (last tile_size rows)
    let tile_size = 6u32;
    let status_bar_start = (height - tile_size) * width * 4;
    println!("\nStatus bar starts at byte: {}", status_bar_start);

    if status_bar_start as usize + 32 <= rgba_data.len() {
        let sample: Vec<u8> = rgba_data[status_bar_start as usize..status_bar_start as usize + 32].to_vec();
        println!("First 32 bytes of status bar row: {:?}", sample);
        let has_content = sample.iter().any(|&b| b != 0 && b != 255);
        println!("Has non-trivial content: {}", has_content);
    }

    // Convert to PNG
    if let Some(img) = RgbaImage::from_raw(width, height, rgba_data) {
        img.save("/tmp/crafter_tui_actual_frame.png").expect("Failed to save PNG");
        println!("\nSaved to /tmp/crafter_tui_actual_frame.png");
        println!("Open this file to verify the status bar is present!");
    } else {
        println!("ERROR: Failed to create image - dimension mismatch");
    }
}
