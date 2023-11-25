use bevy_ecs::prelude::*;
use bevy_app::prelude::*;
use crate::editor::ui::{editor_ui, inspector_ui, systems_ui, image_assets_ui, sprite_atlas_ui, sprite_atlas_assets_ui};

pub mod ui;

pub struct RuminativeEditorPlugin;

impl Plugin for RuminativeEditorPlugin {
  fn build(&self, app: &mut App) {
    app.add_systems(Update, editor_ui);
    app.add_systems(Update, inspector_ui);
    app.add_systems(Update, systems_ui);
    app.add_systems(Update, image_assets_ui);
    app.add_systems(Update, sprite_atlas_ui);
    app.add_systems(Update, sprite_atlas_assets_ui);
  }
}