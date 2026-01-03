use anyhow::Result;
use opentui_sys as ot;

/// Thin wrapper around the OpenTUI renderer pointer.
pub struct Renderer {
    ptr: *mut ot::CliRenderer,
}

impl Renderer {
    pub fn new(width: u32, height: u32) -> Result<Self> {
        let ptr = unsafe { ot::createRenderer(width, height) };
        if ptr.is_null() {
            anyhow::bail!("Failed to create OpenTUI renderer");
        }
        Ok(Self { ptr })
    }

    pub fn resize(&self, width: u32, height: u32) {
        unsafe { ot::resizeRenderer(self.ptr, width, height) };
    }

    pub fn next_buffer(&self) -> *mut ot::OptimizedBuffer {
        unsafe { ot::getNextBuffer(self.ptr) }
    }

    pub fn render(&self) {
        unsafe { ot::render(self.ptr, false) };
    }
}

impl Drop for Renderer {
    fn drop(&mut self) {
        unsafe { ot::destroyRenderer(self.ptr, true, 0) };
    }
}
