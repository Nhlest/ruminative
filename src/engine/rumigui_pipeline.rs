use crate::engine::{RuminativeInternals, RuminativePipeline};
use smallvec::smallvec;
use std::error::Error;
use std::sync::Arc;
use vulkano::buffer::BufferContents;
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
use vulkano::pipeline::{DynamicState, GraphicsPipeline, Pipeline, PipelineLayout, PipelineShaderStageCreateInfo};
use vulkano::shader::EntryPoint;
use vulkano::swapchain::Swapchain;
use vulkano::sync::GpuFuture;
use winit::window::Window;

pub struct RumiguiPipeline {
  pub pipeline: Arc<GraphicsPipeline>,
  // pub descriptor_set: Arc<PersistentDescriptorSet>,
  // pub vertex_buffer: Subbuffer<[DrawVertPod]>,
}

impl RuminativePipeline for RumiguiPipeline {
  fn update(&mut self, _ruminative_internals: &RuminativeInternals) {}
  fn bind<'a>(
    &self,
    builder: &'a mut AutoCommandBufferBuilder<PrimaryAutoCommandBuffer>,
    window: &Window,
  ) -> Result<&'a mut AutoCommandBufferBuilder<PrimaryAutoCommandBuffer>, Box<dyn Error>> {
    builder.bind_pipeline_graphics(self.pipeline.clone())?;
    // .bind_descriptor_sets(
    //   PipelineBindPoint::Graphics,
    //   self.pipeline.layout().clone(),
    //   0,
    //   self.descriptor_set.clone(),
    // );
    builder.push_constants(
      self.pipeline.layout().clone(),
      0,
      vs::PushConstants {
        window_height: window.inner_size().to_logical(window.scale_factor()).height,
        window_width: window.inner_size().to_logical(window.scale_factor()).width,
      },
    )?;
    // .unwrap();
    Ok(builder)
  }
  // fn handle_event(&mut self, _window: &Window, _event: &Event<()>) {
  // }
}

#[derive(BufferContents, Vertex, Clone)]
#[repr(C)]
pub struct DrawVertPod {
  #[format(R32G32_SFLOAT)]
  pos: [f32; 2],
  #[format(R32G32_SFLOAT)]
  uv: [f32; 2],
  #[format(R8G8B8A8_UNORM)]
  col: [u8; 4],
}

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

  pub fn new(
    ruminative_internals: &RuminativeInternals,
    future: Option<Box<dyn GpuFuture>>,
  ) -> Result<(Self, Box<dyn GpuFuture>), Box<dyn Error>> {
    let _memory_allocator = &ruminative_internals.memory_allocator;
    let (vs, fs) = Self::shaders(ruminative_internals.device.clone())?;
    let pipeline = Self::pipeline(
      ruminative_internals.device.clone(),
      ruminative_internals.swapchain.clone(),
      vs.clone(),
      fs.clone(),
    )?;

    let previous_frame_end = if let Some(future) = future {
      // future.join(previous_frame_end.unwrap()).boxed()
      future
    } else {
      panic!("!");
      // previous_frame_end.unwrap()
      // future.unwrap()
    };

    Ok((
      Self {
        pipeline,
        // descriptor_set,
        // vertex_buffer
      },
      previous_frame_end,
    ))
  }
}
