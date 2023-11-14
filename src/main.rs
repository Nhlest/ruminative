use crate::engine::engine::Ruminative;
use std::error::Error;

mod engine;

fn main() -> Result<(), Box<dyn Error>> {
  let mut ctx = Ruminative::new()?;
  ctx.app.run();
  Ok(())
}
