use crate::engine::{RuminativeInternals, RuminativePipeline};
use smallvec::smallvec;
use std::error::Error;
use std::io::Cursor;
use std::sync::Arc;
use vulkano::buffer::{Buffer, BufferContents, BufferCreateInfo, BufferUsage, Subbuffer};
use vulkano::command_buffer::allocator::StandardCommandBufferAllocator;
use vulkano::command_buffer::{
  AutoCommandBufferBuilder, CommandBufferUsage, CopyBufferToImageInfo, PrimaryAutoCommandBuffer,
  PrimaryCommandBufferAbstract,
};
use vulkano::descriptor_set::allocator::{StandardDescriptorSetAllocator, StandardDescriptorSetAllocatorCreateInfo};
use vulkano::descriptor_set::{PersistentDescriptorSet, WriteDescriptorSet};
use vulkano::device::{Device, Queue};
use vulkano::format::Format;
use vulkano::image::sampler::{Filter, Sampler, SamplerAddressMode, SamplerCreateInfo};
use vulkano::image::view::ImageView;
use vulkano::image::{Image, ImageCreateInfo, ImageType, ImageUsage};
use vulkano::memory::allocator::{AllocationCreateInfo, MemoryTypeFilter, StandardMemoryAllocator};
use vulkano::pipeline::graphics::color_blend::{AttachmentBlend, ColorBlendAttachmentState, ColorBlendState};
use vulkano::pipeline::graphics::input_assembly::{InputAssemblyState, PrimitiveTopology};
use vulkano::pipeline::graphics::multisample::MultisampleState;
use vulkano::pipeline::graphics::rasterization::RasterizationState;
use vulkano::pipeline::graphics::subpass::PipelineRenderingCreateInfo;
use vulkano::pipeline::graphics::vertex_input::{Vertex, VertexDefinition};
use vulkano::pipeline::graphics::viewport::ViewportState;
use vulkano::pipeline::graphics::GraphicsPipelineCreateInfo;
use vulkano::pipeline::layout::PipelineDescriptorSetLayoutCreateInfo;
use vulkano::pipeline::{
  DynamicState, GraphicsPipeline, Pipeline, PipelineBindPoint, PipelineLayout, PipelineShaderStageCreateInfo,
};
use vulkano::shader::EntryPoint;
use vulkano::swapchain::Swapchain;
use vulkano::sync::GpuFuture;
use vulkano::DeviceSize;
use winit::window::Window;

pub struct TilemapPipeline {
  pub pipeline: Arc<GraphicsPipeline>,
  pub descriptor_set: Arc<PersistentDescriptorSet>,
  pub vertex_buffer: Subbuffer<[MVertex]>,
}

impl RuminativePipeline for TilemapPipeline {
  fn bind<'a>(
    &self,
    builder: &'a mut AutoCommandBufferBuilder<PrimaryAutoCommandBuffer>,
    _window: &Window,
  ) -> Result<&'a mut AutoCommandBufferBuilder<PrimaryAutoCommandBuffer>, Box<dyn Error>> {
    Ok(
      builder
        .bind_pipeline_graphics(self.pipeline.clone())?
        .bind_descriptor_sets(
          PipelineBindPoint::Graphics,
          self.pipeline.layout().clone(),
          0,
          self.descriptor_set.clone(),
        )?
        .bind_vertex_buffers(0, self.vertex_buffer.clone())?
        .push_constants(self.pipeline.layout().clone(), 0, vs::Constants { offset: [0.2, 0.1] })?
        .draw(4, self.vertex_buffer.len() as u32, 0, 0)?,
    )
  }
}

#[derive(BufferContents, Vertex)]
#[repr(C)]
pub struct MVertex {
  #[format(R32G32_SFLOAT)]
  in_coord: [f32; 2],
  #[format(R32_UINT)]
  tile: u32,
}

mod vs {
  vulkano_shaders::shader! {
      ty: "vertex",
      path: "assets/shaders/vertex.glsl"
  }
}

mod fs {
  vulkano_shaders::shader! {
      ty: "fragment",
      path: "assets/shaders/fragment.glsl"
  }
}

impl TilemapPipeline {
  fn shaders(device: Arc<Device>) -> Result<(EntryPoint, EntryPoint), Box<dyn Error>> {
    let vs = vs::load(device.clone())?
      .specialize([(0, 49u32.into()), (1, 22u32.into())].into_iter().collect())?
      .entry_point("main")
      .unwrap();
    let fs = fs::load(device)?.entry_point("main").unwrap();
    Ok((vs, fs))
  }

  fn vertex_buffer(memory_allocator: Arc<StandardMemoryAllocator>) -> Result<Subbuffer<[MVertex]>, Box<dyn Error>> {
    let vertices = [
      MVertex {
        in_coord: [0.0, -0.1],
        tile: 1,
      },
      MVertex {
        in_coord: [0.0, -0.3],
        tile: 2,
      },
      MVertex {
        in_coord: [0.3, -0.4],
        tile: 3,
      },
    ];
    let vertex_buffer = Buffer::from_iter(
      memory_allocator,
      BufferCreateInfo {
        usage: BufferUsage::VERTEX_BUFFER,
        ..Default::default()
      },
      AllocationCreateInfo {
        memory_type_filter: MemoryTypeFilter::PREFER_HOST,
        ..Default::default()
      },
      vertices,
    )?;
    Ok(vertex_buffer)
  }

  fn sampler(
    device: Arc<Device>,
    queue: Arc<Queue>,
    pipeline: Arc<GraphicsPipeline>,
    memory_allocator: Arc<StandardMemoryAllocator>,
  ) -> Result<(Arc<PersistentDescriptorSet>, Option<Box<dyn GpuFuture>>), Box<dyn Error>> {
    let descriptor_set_allocator =
      StandardDescriptorSetAllocator::new(device.clone(), StandardDescriptorSetAllocatorCreateInfo::default());
    let command_buffer_allocator = StandardCommandBufferAllocator::new(device.clone(), Default::default());

    let mut uploads = AutoCommandBufferBuilder::primary(
      &command_buffer_allocator,
      queue.queue_family_index(),
      CommandBufferUsage::OneTimeSubmit,
    )?;

    let texture = {
      let png_bytes = include_bytes!("../../assets/tiles.png").to_vec();
      let cursor = Cursor::new(png_bytes);
      let decoder = png::Decoder::new(cursor);
      let mut reader = decoder.read_info()?;
      let info = reader.info();
      let dimensions = [info.width, info.height, 1];

      let upload_buffer = Buffer::new_slice(
        memory_allocator.clone(),
        BufferCreateInfo {
          usage: BufferUsage::TRANSFER_SRC,
          ..Default::default()
        },
        AllocationCreateInfo {
          memory_type_filter: MemoryTypeFilter::PREFER_HOST | MemoryTypeFilter::HOST_SEQUENTIAL_WRITE,
          ..Default::default()
        },
        (info.width * info.height * 4) as DeviceSize,
      )?;

      reader.next_frame(&mut upload_buffer.write().unwrap()).unwrap();

      let image = Image::new(
        memory_allocator,
        ImageCreateInfo {
          usage: ImageUsage::TRANSFER_DST | ImageUsage::SAMPLED,
          format: Format::R8G8B8A8_SRGB,
          image_type: ImageType::Dim2d,
          extent: dimensions,
          ..Default::default()
        },
        AllocationCreateInfo {
          memory_type_filter: MemoryTypeFilter::PREFER_DEVICE,
          ..Default::default()
        },
      )?;

      uploads.copy_buffer_to_image(CopyBufferToImageInfo::buffer_image(upload_buffer, image.clone()))?;
      ImageView::new_default(image)?
    };

    let sampler = Sampler::new(
      device,
      SamplerCreateInfo {
        mag_filter: Filter::Nearest,
        min_filter: Filter::Nearest,
        address_mode: [SamplerAddressMode::Repeat; 3],
        ..Default::default()
      },
    )?;

    let layout = pipeline.layout().set_layouts().get(0).unwrap();
    let set = PersistentDescriptorSet::new(
      &descriptor_set_allocator,
      layout.clone(),
      [WriteDescriptorSet::image_view_sampler(0, texture, sampler)],
      [],
    )?;

    let previous_frame_end = Some(uploads.build()?.execute(queue)?.boxed());

    Ok((set, previous_frame_end))
  }

  fn pipeline(
    device: Arc<Device>,
    swapchain: Arc<Swapchain>,
    vs: EntryPoint,
    fs: EntryPoint,
  ) -> Result<Arc<GraphicsPipeline>, Box<dyn Error>> {
    let vertex_input_state = MVertex::per_instance().definition(&vs.info().input_interface)?;
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
        input_assembly_state: Some(InputAssemblyState {
          topology: PrimitiveTopology::TriangleStrip,
          ..Default::default()
        }),
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
    let memory_allocator = Arc::new(StandardMemoryAllocator::new_default(
      ruminative_internals.device.clone(),
    ));
    let (vs, fs) = Self::shaders(ruminative_internals.device.clone())?;
    let pipeline = Self::pipeline(
      ruminative_internals.device.clone(),
      ruminative_internals.swapchain.clone(),
      vs,
      fs,
    )?;
    let (descriptor_set, previous_frame_end) = Self::sampler(
      ruminative_internals.device.clone(),
      ruminative_internals.queue.clone(),
      pipeline.clone(),
      memory_allocator.clone(),
    )?;
    let vertex_buffer = Self::vertex_buffer(memory_allocator)?;
    let previous_frame_end = if let Some(future) = future {
      future.join(previous_frame_end.unwrap()).boxed()
    } else {
      previous_frame_end.unwrap()
    };

    Ok((
      Self {
        pipeline,
        descriptor_set,
        vertex_buffer,
      },
      previous_frame_end,
    ))
  }
}
