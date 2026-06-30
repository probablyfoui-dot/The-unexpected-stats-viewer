// `render/src/canvas/shape.rs` — Shape trait and Position/Size structs.

// ---- Imports ---- //
use super::{context::DrawContext, text::TextRenderer};

// ---- Shape trait ---- //
pub trait Shape {
    fn draw(&self, ctx: &mut DrawContext);
    fn size(&self) -> (u32, u32);
    fn measure(&self, _renderer: &TextRenderer) -> (u32, u32) {
        self.size()
    }
}
