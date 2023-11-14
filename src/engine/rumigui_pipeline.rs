use crate::engine::imgui_pipeline::DrawVertPod;
use crate::engine::{handle_result, ASingleton, AssociatedResource, PipelineRunner, Resultat};
use bevy_app::{App, Plugin};
use bevy_ecs::prelude::*;
use smallvec::smallvec;
use std::error::Error;
use std::sync::Arc;
use vulkano::command_buffer::{AutoCommandBufferBuilder, PrimaryAutoCommandBuffer};
use vulkano::device::Device;
use vulkano::pipeline::graphics::color_blend::{AttachmentBlend, ColorBlendAttachmentState, ColorBlendState};
use vulkano::pipeline::graphics::input_assembly::InputAssemblyState;
use vulkano::pipeline::graphics::multisample::MultisampleState;
use vulkano::pipeline::graphics::rasterization::RasterizationState;
use vulkano::pipeline::graphics::subpass::PipelineRenderingCreateInfo;
use vulkano::pipeline::graphics::vertex_input::{Vertex, VertexDefinition};
use vulkano::pipeline::graphics::viewport::ViewportState;
use vulkano::pipeline::graphics::GraphicsPipelineCreateInfo;
use vulkano::pipeline::layout::PipelineDescriptorSetLayoutCreateInfo;
use vulkano::pipeline::{DynamicState, GraphicsPipeline, PipelineLayout, PipelineShaderStageCreateInfo};
use vulkano::shader::EntryPoint;
use vulkano::swapchain::Swapchain;

pub struct RumiguiPipeline;

mod vs {
  vulkano_shaders::shader! {
      ty: "vertex",
      path: "assets/shaders/imgui_vertex.glsl"
  }
}

mod fs {
  vulkano_shaders::shader! {
      ty: "fragment",
      path: "assets/shaders/imgui_fragment.glsl"
  }
}

impl RumiguiPipeline {
  fn bind<'a>(
    mut builder: NonSendMut<AutoCommandBufferBuilder<PrimaryAutoCommandBuffer>>,
    pipeline: Res<AssociatedResource<Self, Arc<GraphicsPipeline>>>,
  ) -> Resultat<()> {
    builder.bind_pipeline_graphics(pipeline.clone())?;
    Ok(())
  }
  fn shaders(device: Arc<Device>) -> Result<(EntryPoint, EntryPoint), Box<dyn Error>> {
    let vs = vs::load(device.clone())?.entry_point("main").unwrap();
    let fs = fs::load(device)?.entry_point("main").unwrap();
    Ok((vs, fs))
  }

  fn pipeline(
    device: Arc<Device>,
    swapchain: Arc<Swapchain>,
    vs: EntryPoint,
    fs: EntryPoint,
  ) -> Result<Arc<GraphicsPipeline>, Box<dyn Error>> {
    let vertex_input_state = DrawVertPod::per_vertex().definition(&vs.info().input_interface)?;
    let stages = smallvec![
      PipelineShaderStageCreateInfo::new(vs),
      PipelineShaderStageCreateInfo::new(fs),
    ];
    let layout = PipelineLayout::new(
      device.clone(),
      PipelineDescriptorSetLayoutCreateInfo::from_stages(&stages).into_pipeline_layout_create_info(device.clone())?,
    )?;
    let subpass = PipelineRenderingCreateInfo {
      color_attachment_formats: vec![Some(swapchain.image_format())],
      ..Default::default()
    };
    let pipeline = GraphicsPipeline::new(
      device.clone(),
      None,
      GraphicsPipelineCreateInfo {
        stages,
        vertex_input_state: Some(vertex_input_state),
        input_assembly_state: Some(InputAssemblyState::default()),
        viewport_state: Some(ViewportState::default()),
        rasterization_state: Some(RasterizationState::default()),
        multisample_state: Some(MultisampleState::default()),
        color_blend_state: Some(ColorBlendState::with_attachment_states(
          subpass.color_attachment_formats.len() as u32,
          ColorBlendAttachmentState {
            blend: Some(AttachmentBlend::alpha()),
            ..Default::default()
          },
        )),
        dynamic_state: [DynamicState::Viewport].into_iter().collect(),
        subpass: Some(subpass.into()),
        ..GraphicsPipelineCreateInfo::layout(layout)
      },
    )?;
    Ok(pipeline)
  }

  fn init(app: &mut App) -> Resultat<()> {
    let device = app.world.resource::<ASingleton<Device>>();
    let swapchain = app.world.resource::<ASingleton<Swapchain>>();
    let (vs, fs) = Self::shaders(device.clon())?;
    let pipeline = Self::pipeline(device.clon(), swapchain.clon(), vs.clone(), fs.clone())?;

    app.insert_resource(AssociatedResource::<Self, _>::new(pipeline));

    Ok(())
  }
}

impl Plugin for RumiguiPipeline {
  fn build(&self, app: &mut App) {
    RumiguiPipeline::init(app).unwrap();
    let system_id = app.world.register_system(RumiguiPipeline::bind.pipe(handle_result));
    app.world.resource_mut::<PipelineRunner>().order.push(system_id);
  }
}
