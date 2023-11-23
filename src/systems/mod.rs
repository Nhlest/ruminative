use std::collections::HashMap;
use bevy_app::{App, Plugin, PostUpdate, PreUpdate};
use bevy_derive::{Deref, DerefMut};
use bevy_ecs::event::EventReader;
use bevy_ecs::prelude::{Commands, Query, World};
use bevy_ecs::system::{IntoSystem, SystemId};
use winit::event::KeyboardInput;
use crate::bevy::Resource;
use crate::engine::KeyPressed;
use crate::engine::tilemap_pipeline::OnKeyPress;

#[derive(Resource, Default, Deref, DerefMut)]
pub struct SystemStorage {
  pub s: HashMap<SystemId, String>
}

impl SystemStorage {
  pub fn add_system<M, T: IntoSystem<(), (), M> + 'static>(world: &mut World, s: T) {
    let system_id = world.register_system(s);
    let mut system_storage = world.resource_mut::<SystemStorage>();
    system_storage.insert(system_id, std::any::type_name::<T>().to_string());
  }
}

pub fn run_systems(
  mut commands: Commands,
  keys: EventReader<KeyPressed>,
  okp: Query<&OnKeyPress>
) {
  if keys.is_empty() {
    return;
  }
  for okp in okp.iter() {
    if let Some(system_id) = okp.0 {
      commands.run_system(system_id);
    }
  }
}

pub struct SystemRunnerPlugin;

impl Plugin for SystemRunnerPlugin {
  fn build(&self, app: &mut App) {
    app.add_systems(PostUpdate, run_systems);
  }
}