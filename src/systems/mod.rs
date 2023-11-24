use std::collections::HashMap;
use std::error::Error;
use bevy_app::{App, Plugin, PostUpdate, PreUpdate};
use bevy_derive::{Deref, DerefMut};
use bevy_ecs::event::EventReader;
use bevy_ecs::prelude::{Bundle, Commands, Component, Entity, Query, World};
use bevy_ecs::system::{BoxedSystem, Command, IntoSystem, RegisteredSystemError, SystemId};
use winit::event::KeyboardInput;
use crate::bevy::Resource;
use crate::engine::KeyPressed;
use crate::engine::tilemap_pipeline::OnKeyPress;

#[derive(Hash, PartialEq, Eq, Copy, Clone)]
pub struct MySystemId(pub Entity);

#[derive(Component)]
pub struct MyRegisteredSystem<I> {
  initialized: bool,
  system: BoxedSystem<I, ()>
}

pub struct MyRunSystem<I> {
  system_id: MySystemId,
  arg: I
}

impl<I: Send> MyRunSystem<I> {
  pub fn new(system_id: MySystemId, arg: I) -> Self {
    Self {
      system_id,
      arg
    }
  }
}

impl<I: Send + 'static> Command for MyRunSystem<I> {
  #[inline]
  fn apply(self, world: &mut World) {
    let _ = world.my_run_system::<I>(self.system_id, self.arg);
  }
}

pub trait CommandsExtension {
  fn my_run_system<I: Send + 'static>(&mut self, system_id: MySystemId, arg: I);
}

impl<'w, 's> CommandsExtension for Commands<'w, 's> {
  fn my_run_system<I: Send + 'static>(&mut self, system_id: MySystemId, arg: I) {
    self.add(MyRunSystem::new(system_id, arg));
  }
}

pub trait WorldExtension {
  fn my_register_system<I: 'static, M, S: IntoSystem<I, (), M> + 'static>(&mut self, system: S) -> MySystemId;
  fn my_run_system<I: 'static>(&mut self, id: MySystemId, arg: I) -> Result<(), Box<dyn Error>>;
}

impl WorldExtension for World {
  fn my_register_system<I: 'static, M, S: IntoSystem<I, (), M> + 'static>(
    &mut self,
    system: S,
  ) -> MySystemId {
    MySystemId(
      self.spawn(MyRegisteredSystem {
        initialized: false,
        system: Box::new(IntoSystem::into_system(system)),
      }).id(),
    )
  }

  fn my_run_system<I: 'static>(&mut self, id: MySystemId, arg: I) -> Result<(), Box<dyn Error>> {
    let mut entity = self
      .get_entity_mut(id.0).unwrap();

    // take ownership of system trait object
    let MyRegisteredSystem {
      mut initialized,
      mut system,
    } = entity
      .take::<MyRegisteredSystem<I>>().unwrap();

    // run the system
    if !initialized {
      system.initialize(self);
      initialized = true;
    }
    system.run(arg, self);
    system.apply_deferred(self);

    // return ownership of system trait object (if entity still exists)
    if let Some(mut entity) = self.get_entity_mut(id.0) {
      entity.insert::<MyRegisteredSystem<I>>(MyRegisteredSystem {
        initialized,
        system,
      });
    }
    Ok(())
  }
}

#[derive(Resource, Default, Deref, DerefMut)]
pub struct SystemStorage {
  pub s: HashMap<MySystemId, String>
}

impl SystemStorage {
  // pub fn add_system<M, T: IntoSystem<(), (), M> + 'static>(world: &mut World, s: T) {
  //   let system_id = world.register_system(s);
  //   let mut system_storage = world.resource_mut::<SystemStorage>();
  //   system_storage.insert(system_id, std::any::type_name::<T>().to_string());
  // }
  pub fn my_add_system<I: 'static, M, T: IntoSystem<I, (), M> + 'static>(world: &mut World, s: T) {
    let system_id = world.my_register_system(s);
    let mut system_storage = world.resource_mut::<SystemStorage>();
    system_storage.insert(system_id, std::any::type_name::<T>().to_string());
  }
}

pub fn run_systems(
  mut commands: Commands,
  mut keys: EventReader<KeyPressed>,
  okp: Query<(Entity, &OnKeyPress)>
) {
  if keys.is_empty() {
    return;
  }
  for i in keys.read() {
    for (entity, okp) in okp.iter() {
      if let Some(system_id) = okp.0 {
        commands.my_run_system(system_id, (entity, i.0));
      }
    }
  }
}

pub struct SystemRunnerPlugin;

impl Plugin for SystemRunnerPlugin {
  fn build(&self, app: &mut App) {
    app.add_systems(PostUpdate, run_systems);
  }
}