use bevy_derive::*;
use bevy_ecs::prelude::*;
use bevy_ecs::system::SystemId;
use smallvec::SmallVec;
use std::error::Error;
use std::marker::PhantomData;
use std::sync::Arc;
use winit::event::Event;

pub mod imgui_pipeline;
pub mod rumigui_pipeline;
pub mod tilemap;
pub mod tilemap_pipeline;

pub mod engine;
pub mod internals;

#[derive(Event)]
pub struct WinitEvent(Event<'static, ()>);

pub type Resultat<T> = Result<T, Box<dyn Error>>;

pub fn handle_result(r: In<Resultat<()>>) {
  if let Err(e) = r.0 {
    eprintln!("Error: {}", e)
  }
}

#[derive(Resource, Deref, DerefMut)]
pub struct AssociatedResource<P, T> {
  #[deref]
  data: T,
  _p: PhantomData<P>,
}

impl<P, T> AssociatedResource<P, T> {
  pub fn new(data: T) -> Self {
    Self {
      data,
      _p: Default::default(),
    }
  }
}

#[derive(Resource, Default)]
pub struct PipelineRunner {
  pub order: SmallVec<[SystemId; 5]>,
}

#[derive(Resource, Deref, DerefMut)]
pub struct Singleton<T>(pub T);

#[derive(Resource, Deref, DerefMut)]
pub struct ASingleton<T>(pub Arc<T>);

impl<T> ASingleton<T> {
  pub fn clon(&self) -> Arc<T> {
    self.0.clone()
  }
}
