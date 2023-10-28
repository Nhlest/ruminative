use crate::engine::Ruminative;
use crate::graphics::runner::run;
use std::error::Error;

mod engine;
mod graphics;

fn main() -> Result<(), Box<dyn Error>> {
  let ctx = Ruminative::new()?;
  run(ctx)?;
  Ok(())
}
