use std::error::Error;
use std::io::Cursor;
use std::sync::Arc;
use vulkano::buffer::{Buffer, BufferContents, BufferCreateInfo, BufferUsage, Subbuffer};
use vulkano::command_buffer::allocator::StandardCommandBufferAllocator;
use vulkano::command_buffer::{AutoCommandBufferBuilder, CommandBufferUsage, PrimaryCommandBufferAbstract};
use vulkano::descriptor_set::allocator::StandardDescriptorSetAllocator;
use vulkano::descriptor_set::{PersistentDescriptorSet, WriteDescriptorSet};
use vulkano::device::physical::PhysicalDeviceType;
use vulkano::device::{Device, DeviceCreateInfo, DeviceExtensions, Features, Queue, QueueCreateInfo, QueueFlags};
use vulkano::format::Format;
use vulkano::image::view::ImageView;
use vulkano::image::{ImageDimensions, ImageUsage, ImmutableImage, MipmapsCount, SwapchainImage};
use vulkano::instance::{Instance, InstanceCreateInfo};
use vulkano::memory::allocator::{AllocationCreateInfo, MemoryUsage, StandardMemoryAllocator};
use vulkano::pipeline::graphics::color_blend::ColorBlendState;
use vulkano::pipeline::graphics::input_assembly::{InputAssemblyState, PrimitiveTopology};
use vulkano::pipeline::graphics::render_pass::PipelineRenderingCreateInfo;
use vulkano::pipeline::graphics::vertex_input::Vertex;
use vulkano::pipeline::graphics::viewport::{Viewport, ViewportState};
use vulkano::pipeline::{GraphicsPipeline, Pipeline};
use vulkano::sampler::{Filter, Sampler, SamplerAddressMode, SamplerCreateInfo};
use vulkano::shader::ShaderModule;
use vulkano::swapchain::{PresentMode, Surface, Swapchain, SwapchainCreateInfo};
use vulkano::sync::GpuFuture;
use vulkano::VulkanLibrary;
use vulkano_win::{required_extensions, VkSurfaceBuild};
use winit::event_loop::EventLoop;
use winit::window::{Window, WindowBuilder};

pub struct ZaWarudoInternals {
  pub event_loop: EventLoop<()>,
  pub device: Arc<Device>,
  pub queue: Arc<Queue>,
  pub surface: Arc<Surface>,
  pub swapchain: Arc<Swapchain>,
  pub images: Vec<Arc<SwapchainImage>>,
}

impl ZaWarudoInternals {
  fn instance() -> Result<Arc<Instance>, Box<dyn Error>> {
    let library = VulkanLibrary::new()?;
    let required_extensions = required_extensions(&library);

    let instance = Instance::new(
      library,
      InstanceCreateInfo {
        enabled_extensions: required_extensions,
        ..Default::default()
      },
    )?;
    Ok(instance)
  }
  fn device_surface_and_queue(
    event_loop: &EventLoop<()>,
    instance: Arc<Instance>,
  ) -> Result<(Arc<Device>, Arc<Surface>, Arc<Queue>), Box<dyn Error>> {
    let surface = WindowBuilder::new().build_vk_surface(&event_loop, instance.clone())?;

    let device_extensions = DeviceExtensions {
      khr_swapchain: true,
      ..DeviceExtensions::empty()
    };

    let (physical_device, queue_family_index) = instance
      .enumerate_physical_devices()?
      .filter(|p| p.supported_extensions().contains(&device_extensions))
      .filter_map(|p| {
        p.queue_family_properties()
          .iter()
          .enumerate()
          .position(|(i, q)| {
            q.queue_flags.intersects(QueueFlags::GRAPHICS) && p.surface_support(i as u32, &surface).unwrap_or(false)
          })
          .map(|i| (p, i as u32))
      })
      .min_by_key(|(p, _)| match p.properties().device_type {
        PhysicalDeviceType::IntegratedGpu => 1,
        PhysicalDeviceType::DiscreteGpu => 0,
        PhysicalDeviceType::VirtualGpu => 2,
        PhysicalDeviceType::Cpu => 3,
        PhysicalDeviceType::Other => 4,
        _ => 5,
      })
      .ok_or("Can't find physical device")?;

    println!(
      "Using device: {} {:?}",
      physical_device.properties().device_name,
      physical_device.properties().device_type
    );

    let (device, mut queues) = Device::new(
      physical_device,
      DeviceCreateInfo {
        enabled_extensions: device_extensions,
        queue_create_infos: vec![QueueCreateInfo {
          queue_family_index,
          ..Default::default()
        }],
        enabled_features: Features {
          dynamic_rendering: true,
          ..Features::empty()
        },
        ..Default::default()
      },
    )?;

    let queue = queues.next().ok_or("No queue")?;
    Ok((device, surface, queue))
  }
  fn swapchain_and_images(
    device: Arc<Device>,
    surface: Arc<Surface>,
  ) -> Result<(Arc<Swapchain>, Vec<Arc<SwapchainImage>>), Box<dyn Error>> {
    let surface_capabilities = device
      .physical_device()
      .surface_capabilities(&surface, Default::default())?;
    let image_format = Some(device.physical_device().surface_formats(&surface, Default::default())?[0].0);
    let window = surface
      .object()
      .ok_or("No object")?
      .downcast_ref::<Window>()
      .ok_or("No window")?;
    Ok(Swapchain::new(
      device.clone(),
      surface.clone(),
      SwapchainCreateInfo {
        min_image_count: surface_capabilities.min_image_count,
        image_format,
        image_extent: window.inner_size().into(),
        image_usage: ImageUsage::COLOR_ATTACHMENT,
        present_mode: PresentMode::Mailbox,
        composite_alpha: surface_capabilities
          .supported_composite_alpha
          .into_iter()
          .next()
          .ok_or("No surface capability")?,
        ..Default::default()
      },
    )?)
  }
  pub fn new() -> Result<Self, Box<dyn Error>> {
    let event_loop = EventLoop::new();
    let instance = Self::instance()?;
    let (device, surface, queue) = Self::device_surface_and_queue(&event_loop, instance.clone())?;
    let (swapchain, images) = Self::swapchain_and_images(device.clone(), surface.clone())?;

    Ok(Self {
      event_loop,
      device,
      images,
      surface,
      swapchain,
      queue,
    })
  }
}

pub struct ZaWarudoPipeline {
  pub pipeline: Arc<GraphicsPipeline>,
  pub descriptor_sets: Vec<Arc<PersistentDescriptorSet>>,
  pub vertex_buffer: Subbuffer<[MVertex]>,
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

impl ZaWarudoPipeline {
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

  fn render_pass_and_pipeline(
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

  pub fn new(zawardu_internals: &ZaWarudoInternals) -> Result<(Self, Box<dyn GpuFuture>), Box<dyn Error>> {
    let memory_allocator = StandardMemoryAllocator::new_default(zawardu_internals.device.clone());
    let (vs, fs) = Self::shaders(zawardu_internals.device.clone())?;
    let pipeline = Self::render_pass_and_pipeline(zawardu_internals.device.clone(), zawardu_internals.swapchain.clone(), vs.clone(), fs.clone())?;
    let (descriptor_set, previous_frame_end) =
      Self::sampler(zawardu_internals.device.clone(), zawardu_internals.queue.clone(), pipeline.clone(), &memory_allocator)?;
    let vertex_buffer = Self::vertex_buffer(&memory_allocator)?;

    let viewport = Viewport {
      origin: [0.0, 0.0],
      dimensions: [0.0, 0.0],
      depth_range: 0.0..1.0,
    };

    Ok((Self {
      pipeline,
      descriptor_sets: vec![descriptor_set],
      vertex_buffer,
    }, previous_frame_end.unwrap()))
  }
}

pub struct ZaWarudo {
  pub internals: ZaWarudoInternals,
  pub pipelines: Vec<ZaWarudoPipeline>,
  pub viewport: Viewport,
  pub previous_frame_end: Option<Box<dyn GpuFuture>>,
}

impl ZaWarudo {
  pub fn new() -> Result<Self, Box<dyn Error>> {
    let internals = ZaWarudoInternals::new()?;
    let (pipeline, previous_frame_end) = ZaWarudoPipeline::new(&internals)?;
    let viewport = Viewport {
      origin: [0.0, 0.0],
      dimensions: [0.0, 0.0],
      depth_range: 0.0..1.0,
    };
    Ok(Self {
      internals,
      pipelines: vec![pipeline],
      viewport,
      previous_frame_end: Some(previous_frame_end),
    })
  }
}
