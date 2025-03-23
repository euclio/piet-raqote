use std::path::Path;

use piet::{RenderContext, samples};
use piet_raqote::{Cache, RaqoteRenderContext};
use raqote::{DrawTarget, Transform};

const FILE_PREFIX: &str = "raqote-test";

fn main() {
    samples::samples_main(run_sample, FILE_PREFIX, None);
}

fn run_sample(
    number: usize,
    scale: f64,
    save_path: &Path,
) -> Result<(), Box<dyn std::error::Error>> {
    let sample = samples::get(number)?;
    let size = sample.size() * scale;

    let mut cache = Cache::new();
    let mut target = DrawTarget::new(size.width as i32, size.height as i32);
    target.set_transform(&Transform::identity().then_scale(scale as f32, scale as f32));
    let mut ctx = RaqoteRenderContext::new(&mut target, &mut cache);

    sample.draw(&mut ctx)?;

    ctx.finish()?;
    std::mem::drop(ctx);

    target.write_png(save_path)?;

    Ok(())
}
