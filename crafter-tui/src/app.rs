use anyhow::{Context, Result};
use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use std::sync::mpsc;
use std::time::{Duration, Instant};

use crate::crafter::{
    action_hint, default_recordings_dir, draw_detail, draw_list, drain_updates, handle_key,
    spawn_crafter_loop, CrafterState,
};
use crate::renderer::Renderer;

pub fn run() -> Result<()> {
    let (width, height) = crossterm::terminal::size().context("terminal size")?;
    let mut width = u32::from(width);
    let mut height = u32::from(height);
    let renderer = Renderer::new(width, height)?;

    let (cmd_tx, cmd_rx) = mpsc::channel();
    let (update_tx, update_rx) = mpsc::channel();
    spawn_crafter_loop(cmd_rx, update_tx, default_recordings_dir());

    let mut crafter_state = CrafterState::new();
    let mut show_detail = true;
    let mut last_draw = Instant::now();
    let highlight_bg = [0.0, 0.7, 0.7, 1.0];

    loop {
        let timeout = Duration::from_millis(16);
        if event::poll(timeout)? {
            match event::read()? {
                Event::Key(key) if key.kind == KeyEventKind::Press => {
                    match key.code {
                        KeyCode::Char('q') | KeyCode::Char('Q') => break,
                        KeyCode::Char('d') | KeyCode::Char('D') => {
                            show_detail = !show_detail;
                        }
                        _ => {
                            let _ = handle_key(&mut crafter_state, key, &cmd_tx);
                        }
                    }
                }
                Event::Resize(new_width, new_height) => {
                    width = u32::from(new_width);
                    height = u32::from(new_height);
                    renderer.resize(width, height);
                }
                _ => {}
            }
        }

        drain_updates(&mut crafter_state, &update_rx);

        if last_draw.elapsed() >= Duration::from_millis(16) {
            draw_frame(
                &renderer,
                &crafter_state,
                width,
                height,
                show_detail,
                highlight_bg,
            )?;
            last_draw = Instant::now();
        }
    }

    Ok(())
}

fn draw_frame(
    renderer: &Renderer,
    crafter: &CrafterState,
    width: u32,
    height: u32,
    show_detail: bool,
    highlight_bg: [f32; 4],
) -> Result<()> {
    let buffer = renderer.next_buffer();
    if buffer.is_null() {
        return Ok(());
    }

    let bg = [0.05, 0.06, 0.08, 1.0];
    unsafe { opentui_sys::bufferClear(buffer, bg.as_ptr()) };

    draw_header(buffer, width);
    let list_width = width.saturating_sub(4) / 2;
    draw_list(buffer, crafter, list_width, height, highlight_bg);
    draw_separator(buffer, list_width, height);
    draw_detail(buffer, crafter, list_width, width, height, show_detail);
    draw_action_bar(buffer, crafter, width, height);

    renderer.render();
    Ok(())
}

fn draw_header(buffer: *mut opentui_sys::OptimizedBuffer, width: u32) {
    let header_bg = [0.08, 0.10, 0.14, 1.0];
    let fg = [0.95, 0.96, 0.98, 1.0];
    let dim_fg = [0.65, 0.68, 0.72, 1.0];
    let separator = [0.20, 0.22, 0.26, 1.0];

    unsafe {
        opentui_sys::bufferFillRect(buffer, 0, 0, width, 3, header_bg.as_ptr());
        opentui_sys::bufferFillRect(buffer, 0, 2, width, 1, separator.as_ptr());
    }

    let title = "Crafter";
    let quit_label = "[Q] Quit  [D] Toggle Detail";
    let quit_x = width.saturating_sub(quit_label.len() as u32 + 2);
    unsafe {
        opentui_sys::bufferDrawText(
            buffer,
            title.as_bytes().as_ptr(),
            title.len(),
            2,
            1,
            fg.as_ptr(),
            std::ptr::null(),
            0,
        );
        opentui_sys::bufferDrawText(
            buffer,
            quit_label.as_bytes().as_ptr(),
            quit_label.len(),
            quit_x,
            1,
            dim_fg.as_ptr(),
            std::ptr::null(),
            0,
        );
    }
}

fn draw_separator(buffer: *mut opentui_sys::OptimizedBuffer, list_width: u32, height: u32) {
    let separator = [0.20, 0.22, 0.26, 1.0];
    let x = list_width.saturating_add(2);
    let y = 3u32;
    let h = height.saturating_sub(5);
    unsafe {
        opentui_sys::bufferFillRect(buffer, x, y, 1, h, separator.as_ptr());
    }
}

fn draw_action_bar(
    buffer: *mut opentui_sys::OptimizedBuffer,
    crafter: &CrafterState,
    width: u32,
    height: u32,
) {
    let bg = [0.08, 0.10, 0.14, 1.0];
    let fg = [0.82, 0.84, 0.88, 1.0];
    let y = height.saturating_sub(2);

    unsafe {
        opentui_sys::bufferFillRect(buffer, 0, y, width, 2, bg.as_ptr());
    }

    let hint = action_hint(crafter);
    let trimmed = if hint.len() > width.saturating_sub(4) as usize {
        &hint[..width.saturating_sub(4) as usize]
    } else {
        hint.as_str()
    };

    unsafe {
        opentui_sys::bufferDrawText(
            buffer,
            trimmed.as_bytes().as_ptr(),
            trimmed.len(),
            2,
            y,
            fg.as_ptr(),
            std::ptr::null(),
            0,
        );
    }
}
