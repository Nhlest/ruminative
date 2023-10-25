use crate::graphics::runner::run;
use std::error::Error;
use crate::engine::ZaWarudo;

mod engine;
mod graphics;

fn main() -> Result<(), Box<dyn Error>> {
  let ctx = ZaWarudo::new()?;
  run(ctx)?;
  Ok(())
}
