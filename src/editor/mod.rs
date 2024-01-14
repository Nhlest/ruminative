use bevy_app::prelude::*;
use crate::editor::ui::{inspector_ui, main_menu};

pub mod ui;

pub struct RuminativeEditorPlugin;

impl Plugin for RuminativeEditorPlugin {
  fn build(&self, app: &mut App) {
    app.add_systems(Update, inspector_ui);
    app.add_systems(Update, main_menu);
  }
}