pub struct Tile {}

pub struct Tilemap {
  pub tiles: Vec<Tile>,
  pub size: (usize, usize),
}

impl Tilemap {
  pub fn new(size: (usize, usize)) -> Self {
    Self {
      tiles: (0..size.0 * size.1).map(|_x| Tile {}).collect(),
      size,
    }
  }
}
