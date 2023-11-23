#![feature(adt_const_params)]


use bevy_app::{PluginGroup, PluginGroupBuilder};
use crate::editor::RuminativeEditorPlugin;
use crate::engine::engine::RuminativeEnginePlugin;
use crate::engine::rumigui_pipeline::RumiguiPipeline;

pub mod bevy {
  pub use bevy_ecs::prelude::*;
  pub use bevy_app::prelude::*;
}

pub mod winit {
  pub use winit::*;
}

pub mod imgui {
  pub use imgui::*;
  pub use imgui_sys::*;
}

pub mod engine;
pub mod editor;

pub struct Ruminative;

impl PluginGroup for Ruminative {
  fn build(self) -> PluginGroupBuilder {
    PluginGroupBuilder::start::<Self>()
      .add(RuminativeEnginePlugin)
      .add(RuminativeEditorPlugin)
  }
}