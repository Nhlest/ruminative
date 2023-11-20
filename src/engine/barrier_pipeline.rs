use bevy_app::{App, Plugin};
use bevy_ecs::prelude::*;
use vulkano::command_buffer::{AutoCommandBufferBuilder, PrimaryAutoCommandBuffer};
use crate::engine::{handle_result, PipelineRunner, Resultat};

pub struct BarrierPipeline;

impl BarrierPipeline {
  fn bind<'a>(
    mut builder: NonSendMut<AutoCommandBufferBuilder<PrimaryAutoCommandBuffer>>,
  ) -> Resultat<()> {
    builder.end_rendering()?;
    Ok(())
  }
}

impl Plugin for BarrierPipeline {
  fn build(&self, app: &mut App) {
    let system_id = app.world.register_system(BarrierPipeline::bind.pipe(handle_result));
    app.world.resource_mut::<PipelineRunner>().order.push(system_id);
  }
}
