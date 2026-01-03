use crafter_core::{ImageRenderer, ImageRendererConfig, Session, SessionConfig};

#[cfg(feature = "png")]
fn main() {
    let config = SessionConfig {
        world_size: (48, 20),
        seed: Some(42),
        view_radius: 4,
        ..Default::default()
    };

    let session = Session::new(config);
    let state = session.get_state();

    // Test with TUI settings: tile_size=6 (reduced for better terminal fit)
    let renderer_tui = ImageRenderer::new(ImageRendererConfig {
        tile_size: 6,
        show_status_bars: true,
        apply_lighting: true,
    });

    let rgb_bytes = renderer_tui.render_bytes(&state);
    let view_size = state.view.as_ref().map(|v| v.size()).unwrap_or(0) as u32;
    let tile_size = 6u32;
    let expected_w = view_size * tile_size;
    let expected_h_with_bar = view_size * tile_size + tile_size;
    let expected_h_no_bar = view_size * tile_size;

    println!("=== TUI Settings (tile_size=6) ===");
    println!("View size: {}", view_size);
    println!("Expected pixel dimensions with status bar: {}x{}", expected_w, expected_h_with_bar);
    println!("Expected pixel dimensions without status bar: {}x{}", expected_w, expected_h_no_bar);
    println!("RGB bytes received: {}", rgb_bytes.len());
    println!("Expected bytes with bar: {}", expected_w * expected_h_with_bar * 3);
    println!("Expected bytes without bar: {}", expected_w * expected_h_no_bar * 3);
    println!("Has status bar: {}", rgb_bytes.len() == (expected_w * expected_h_with_bar * 3) as usize);

    match renderer_tui.save_png(&state, "/tmp/crafter_tui_size.png") {
        Ok(()) => println!("Saved to /tmp/crafter_tui_size.png"),
        Err(e) => eprintln!("Error: {}", e),
    }

    // Also test with tile_size=7 for comparison
    let renderer = ImageRenderer::new(ImageRendererConfig {
        tile_size: 7,
        show_status_bars: true,
        apply_lighting: true,
    });

    match renderer.save_png(&state, "/tmp/crafter_rust_with_status.png") {
        Ok(()) => println!("Saved to /tmp/crafter_rust_with_status.png (tile_size=7)"),
        Err(e) => eprintln!("Error: {}", e),
    }

    println!("\nHealth: {}, Food: {}, Drink: {}, Energy: {}",
             state.inventory.health, state.inventory.food,
             state.inventory.drink, state.inventory.energy);
}

#[cfg(not(feature = "png"))]
fn main() {}
