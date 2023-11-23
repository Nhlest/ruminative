use bevy_ecs::prelude::*;
use bevy_app::prelude::*;
use crate::editor::ui::editor_ui;

pub mod ui;

pub struct RuminativeEditorPlugin;

impl Plugin for RuminativeEditorPlugin {
  fn build(&self, app: &mut App) {
    app.add_systems(Update, editor_ui);
  }
}