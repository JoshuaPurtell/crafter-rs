use anyhow::{Context, Result};
use crossterm::event::{
    DisableMouseCapture, EnableMouseCapture, KeyboardEnhancementFlags,
    PopKeyboardEnhancementFlags, PushKeyboardEnhancementFlags,
};
use crossterm::terminal::{disable_raw_mode, enable_raw_mode};

fn main() -> Result<()> {
    enable_raw_mode().context("enable raw mode")?;
    crossterm::execute!(std::io::stdout(), EnableMouseCapture).ok();
    crossterm::execute!(
        std::io::stdout(),
        PushKeyboardEnhancementFlags(KeyboardEnhancementFlags::REPORT_EVENT_TYPES)
    )
    .ok();

    let result = crafter_tui::app::run();

    crossterm::execute!(std::io::stdout(), PopKeyboardEnhancementFlags).ok();
    crossterm::execute!(std::io::stdout(), DisableMouseCapture).ok();
    disable_raw_mode().ok();
    result
}
