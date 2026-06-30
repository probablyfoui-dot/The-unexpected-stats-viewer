// `render/src/canvas/shapes/accent_box.rs` — AccentBox shape.

// ---- Imports ---- //
use image::Rgba;

use crate::canvas::{BOX_BACKGROUND, DrawContext, Shape, blend};
use crate::canvas::shapes::rect::RoundedRect;

// ---- AccentBox struct + impl ---- //
pub struct AccentBox {
    pub width: u32,
    pub height: u32,
    pub corner_radius: u32,
    pub tint: Rgba<u8>,
}

impl AccentBox {
    pub fn new(width: u32, height: u32) -> Self {
        Self {
            width,
            height,
            corner_radius: 14,
            tint: Rgba([0, 180, 160, 18]),
        }
    }
    pub fn corner_radius(mut self, r: u32) -> Self { self.corner_radius = r; self }
    pub fn accent(mut self, color: Rgba<u8>) -> Self { self.tint = color; self }
}

impl Shape for AccentBox {
    fn draw(&self, ctx: &mut DrawContext) {
        RoundedRect::new(self.width, self.height)
            .corner_radius(self.corner_radius)
            .background(BOX_BACKGROUND)
            .draw(ctx);

        let (cw, ch) = ctx.buffer.dimensions();
        for py in 0..self.height {
            for px in 0..self.width {
                let cx = (ctx.x + px as i32) as u32;
                let cy = (ctx.y + py as i32) as u32;
                if cx >= cw || cy >= ch { continue; }
                let pixel = *ctx.buffer.get_pixel(cx, cy);
                if pixel[3] > 10 {
                    ctx.buffer.put_pixel(cx, cy, blend(pixel, self.tint));
                }
            }
        }
    }
    fn size(&self) -> (u32, u32) { (self.width, self.height) }
}
