use std::error::Error;
use std::io::Cursor;
use std::sync::Arc;
use vulkano::buffer::{Buffer, BufferContents, BufferCreateInfo, BufferUsage, Subbuffer};
use vulkano::command_buffer::allocator::StandardCommandBufferAllocator;
use vulkano::command_buffer::{AutoCommandBufferBuilder, CommandBufferUsage, PrimaryAutoCommandBuffer, PrimaryCommandBufferAbstract};
use vulkano::descriptor_set::allocator::StandardDescriptorSetAllocator;
use vulkano::descriptor_set::{PersistentDescriptorSet, WriteDescriptorSet};
use vulkano::device::{Device, Queue};
use vulkano::format::Format;
use vulkano::image::view::ImageView;
use vulkano::image::{ImageDimensions, ImmutableImage, MipmapsCount};
use vulkano::memory::allocator::{AllocationCreateInfo, MemoryUsage, StandardMemoryAllocator};
use vulkano::pipeline::graphics::color_blend::ColorBlendState;
use vulkano::pipeline::graphics::input_assembly::{InputAssemblyState, PrimitiveTopology};
use vulkano::pipeline::graphics::render_pass::PipelineRenderingCreateInfo;
use vulkano::pipeline::graphics::vertex_input::{Vertex};
use vulkano::pipeline::graphics::viewport::{ViewportState};
use vulkano::pipeline::{GraphicsPipeline, Pipeline, PipelineBindPoint};
use vulkano::sampler::{Filter, Sampler, SamplerAddressMode, SamplerCreateInfo};
use vulkano::shader::ShaderModule;
use vulkano::swapchain::{Swapchain};
use vulkano::sync::GpuFuture;
use crate::engine::{RuminativeInternals, RuminativePipeline};

pub struct TilemapPipeline {
  pub pipeline: Arc<GraphicsPipeline>,
  pub descriptor_set: Arc<PersistentDescriptorSet>,
  pub vertex_buffer: Subbuffer<[MVertex]>,
}

impl RuminativePipeline for TilemapPipeline {
  fn bind<'a>(&self, builder: &'a mut AutoCommandBufferBuilder<PrimaryAutoCommandBuffer>) -> &'a mut AutoCommandBufferBuilder<PrimaryAutoCommandBuffer> {
    builder
      .bind_pipeline_graphics(self.pipeline.clone())
      .bind_descriptor_sets(
        PipelineBindPoint::Graphics,
        self.pipeline.layout().clone(),
        0,
        self.descriptor_set.clone(),
      )
      .bind_vertex_buffers(0, self.vertex_buffer.clone())
      .draw(4, self.vertex_buffer.len() as u32, 0, 0)
      .unwrap()
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
  fn shaders(device: Arc<Device>) -> Result<(Arc<ShaderModule>, Arc<ShaderModule>), Box<dyn Error>> {
    let vs = vs::load(device.clone())?;
    let fs = fs::load(device.clone())?;
    Ok((vs, fs))
  }

  fn vertex_buffer(memory_allocator: &StandardMemoryAllocator) -> Result<Subbuffer<[MVertex]>, Box<dyn Error>> {
    let vertices = [
      MVertex {
        in_coord: [0.0, 0.0],
        tile: 1,
      },
      MVertex {
        in_coord: [0.0, 0.3],
        tile: 2,
      },
      MVertex {
        in_coord: [0.3, 0.0],
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
        usage: MemoryUsage::Upload,
        ..Default::default()
      },
      vertices,
    )
      .unwrap();
    Ok(vertex_buffer)
  }

  fn sampler(
    device: Arc<Device>,
    queue: Arc<Queue>,
    pipeline: Arc<GraphicsPipeline>,
    memory_allocator: &StandardMemoryAllocator,
  ) -> Result<(Arc<PersistentDescriptorSet>, Option<Box<dyn GpuFuture>>), Box<dyn Error>> {
    let descriptor_set_allocator = StandardDescriptorSetAllocator::new(device.clone());
    let command_buffer_allocator = StandardCommandBufferAllocator::new(device.clone(), Default::default());
    let mut uploads = AutoCommandBufferBuilder::primary(
      &command_buffer_allocator,
      queue.queue_family_index(),
      CommandBufferUsage::OneTimeSubmit,
    )
      .unwrap();

    let texture = {
      let png_bytes = include_bytes!("../../assets/tiles.png").to_vec();
      let cursor = Cursor::new(png_bytes);
      let decoder = png::Decoder::new(cursor);
      let mut reader = decoder.read_info().unwrap();
      let info = reader.info();
      let dimensions = ImageDimensions::Dim2d {
        width: info.width,
        height: info.height,
        array_layers: 1,
      };
      let mut image_data = Vec::new();
      image_data.resize((info.width * info.height * 4) as usize, 0);
      reader.next_frame(&mut image_data).unwrap();

      let image = ImmutableImage::from_iter(
        memory_allocator,
        image_data,
        dimensions,
        MipmapsCount::One,
        Format::R8G8B8A8_SRGB,
        &mut uploads,
      )
        .unwrap();
      ImageView::new_default(image).unwrap()
    };

    let sampler = Sampler::new(
      device.clone(),
      SamplerCreateInfo {
        mag_filter: Filter::Nearest,
        min_filter: Filter::Nearest,
        address_mode: [SamplerAddressMode::Repeat; 3],
        ..Default::default()
      },
    )
      .unwrap();

    let layout = pipeline.layout().set_layouts().get(0).unwrap();
    let set = PersistentDescriptorSet::new(
      &descriptor_set_allocator,
      layout.clone(),
      [WriteDescriptorSet::image_view_sampler(0, texture, sampler)],
    )
      .unwrap();

    let previous_frame_end = Some(uploads.build().unwrap().execute(queue.clone()).unwrap().boxed());

    Ok((set, previous_frame_end))
  }

  fn pipeline(
    device: Arc<Device>,
    swapchain: Arc<Swapchain>,
    vs: Arc<ShaderModule>,
    fs: Arc<ShaderModule>,
  ) -> Result<Arc<GraphicsPipeline>, Box<dyn Error>> {
    let pipeline = GraphicsPipeline::start()
      .vertex_input_state(MVertex::per_instance())
      .render_pass(PipelineRenderingCreateInfo {
        color_attachment_formats: vec![Some(swapchain.image_format())],
        ..Default::default()
      })
      .input_assembly_state(InputAssemblyState::new().topology(PrimitiveTopology::TriangleStrip))
      .vertex_shader(
        vs.entry_point("main").ok_or("No main entry point in vertex shader")?,
        vs::SpecializationConstants {
          tile_size_x: 49,
          tile_size_y: 22,
        },
      )
      .fragment_shader(
        fs.entry_point("main").ok_or("No main entry point in fragment shader")?,
        (),
      )
      .viewport_state(ViewportState::viewport_dynamic_scissor_irrelevant())
      .color_blend_state(ColorBlendState::default().blend_alpha())
      .build(device.clone())?;
    Ok(pipeline)
  }

  pub fn new(ruminative_internals: &RuminativeInternals) -> Result<(Self, Box<dyn GpuFuture>), Box<dyn Error>> {
    let memory_allocator = StandardMemoryAllocator::new_default(ruminative_internals.device.clone());
    let (vs, fs) = Self::shaders(ruminative_internals.device.clone())?;
    let pipeline = Self::pipeline(ruminative_internals.device.clone(), ruminative_internals.swapchain.clone(), vs.clone(), fs.clone())?;
    let (descriptor_set, previous_frame_end) =
      Self::sampler(ruminative_internals.device.clone(), ruminative_internals.queue.clone(), pipeline.clone(), &memory_allocator)?;
    let vertex_buffer = Self::vertex_buffer(&memory_allocator)?;

    Ok((Self {
      pipeline,
      descriptor_set,
      vertex_buffer,
    }, previous_frame_end.unwrap()))
  }
}
