use crate::graphics::ctx::RenderingContext;
use crate::graphics::runner::run;
use std::error::Error;

mod engine;
mod graphics;

fn main() -> Result<(), Box<dyn Error>> {
  let ctx = RenderingContext::new()?;
  run(ctx)?;
  Ok(())
}
