use crate::graphics::runner::run;
use std::error::Error;
use crate::engine::Ruminative;

mod engine;
mod graphics;

fn main() -> Result<(), Box<dyn Error>> {
  let ctx = Ruminative::new()?;
  run(ctx)?;
  Ok(())
}
