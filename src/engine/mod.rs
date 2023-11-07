use crate::engine::imgui_pipeline::ImguiPipeline;
// use crate::engine::tilemap_pipeline::TilemapPipeline;
use crate::engine::rumigui_pipeline::RumiguiPipeline;
use crate::engine::tilemap_pipeline::TilemapPipeline;
use std::error::Error;
use std::sync::Arc;
use vulkano::command_buffer::{AutoCommandBufferBuilder, PrimaryAutoCommandBuffer};
use vulkano::device::physical::PhysicalDeviceType;
use vulkano::device::{Device, DeviceCreateInfo, DeviceExtensions, Features, Queue, QueueCreateInfo, QueueFlags};
use vulkano::image::{Image, ImageUsage};
use vulkano::instance::{Instance, InstanceCreateInfo};
use vulkano::memory::allocator::StandardMemoryAllocator;
use vulkano::pipeline::graphics::viewport::Viewport;
use vulkano::swapchain::{PresentMode, Surface, Swapchain, SwapchainCreateInfo};
use vulkano::sync::GpuFuture;
use vulkano::VulkanLibrary;
use vulkano_win::required_extensions;
use winit::event::Event;
use winit::event_loop::EventLoop;
use winit::window::{Window, WindowBuilder};

pub mod imgui_pipeline;
pub mod rumigui_pipeline;
pub mod tilemap;
pub mod tilemap_pipeline;

pub struct RuminativeInternals {
  pub memory_allocator: Arc<StandardMemoryAllocator>,
  pub device: Arc<Device>,
  pub queue: Arc<Queue>,
  pub surface: Arc<Surface>,
  pub swapchain: Arc<Swapchain>,
  pub images: Vec<Arc<Image>>,
}

impl RuminativeInternals {
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
    let window = Arc::new(WindowBuilder::new().build(&event_loop)?);
    let surface = Surface::from_window(instance.clone(), window.clone())?;

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
  ) -> Result<(Arc<Swapchain>, Vec<Arc<Image>>), Box<dyn Error>> {
    let surface_capabilities = device
      .physical_device()
      .surface_capabilities(&surface, Default::default())?;
    let image_format = device.physical_device().surface_formats(&surface, Default::default())?[0].0;
    let window = surface
      .object()
      .ok_or("No object")?
      .downcast_ref::<Window>()
      .ok_or("No window")?;
    Ok(Swapchain::new(
      device,
      surface.clone(),
      SwapchainCreateInfo {
        min_image_count: surface_capabilities.min_image_count,
        image_format,
        image_extent: window.inner_size().into(),
        image_usage: ImageUsage::COLOR_ATTACHMENT,
        present_mode: PresentMode::Fifo,
        composite_alpha: surface_capabilities
          .supported_composite_alpha
          .into_iter()
          .next()
          .ok_or("No surface capability")?,
        ..Default::default()
      },
    )?)
  }
  pub fn new(event_loop: &EventLoop<()>) -> Result<Self, Box<dyn Error>> {
    let instance = Self::instance()?;
    let (device, surface, queue) = Self::device_surface_and_queue(&event_loop, instance)?;
    let memory_allocator = Arc::new(StandardMemoryAllocator::new_default(device.clone()));
    let (swapchain, images) = Self::swapchain_and_images(device.clone(), surface.clone())?;

    Ok(Self {
      memory_allocator,
      device,
      images,
      surface,
      swapchain,
      queue,
    })
  }
}

pub trait RuminativePipeline {
  fn handle_event(&mut self, _window: &Window, _event: &Event<()>) {}
  fn update(&mut self, _ruminative_internals: &RuminativeInternals) {}
  fn bind<'a>(
    &self,
    builder: &'a mut AutoCommandBufferBuilder<PrimaryAutoCommandBuffer>,
    _window: &Window,
  ) -> Result<&'a mut AutoCommandBufferBuilder<PrimaryAutoCommandBuffer>, Box<dyn Error>> {
    Ok(builder)
  }
}

pub struct Ruminative {
  pub event_loop: EventLoop<()>,
  pub internals: RuminativeInternals,
  pub pipelines: Vec<Box<dyn RuminativePipeline>>,
  pub viewport: Viewport,
  pub previous_frame_end: Option<Box<dyn GpuFuture>>,
}

impl Ruminative {
  pub fn new() -> Result<Self, Box<dyn Error>> {
    let event_loop = EventLoop::new();
    let internals = RuminativeInternals::new(&event_loop)?;
    let (tilemap_pipeline, previous_frame_end) = TilemapPipeline::new(&internals, None)?;
    let (imgui_pipelinne, previous_frame_end) = ImguiPipeline::new(&internals, Some(previous_frame_end))?;
    let (rumigui_pipelinne, previous_frame_end) = RumiguiPipeline::new(&internals, Some(previous_frame_end))?;
    let viewport = Viewport {
      offset: [0.0, 0.0],
      depth_range: 0.0..=1.0,
      extent: [0.0, 0.0],
    };
    Ok(Self {
      event_loop,
      internals,
      pipelines: vec![
        Box::new(tilemap_pipeline),
        Box::new(imgui_pipelinne),
        Box::new(rumigui_pipelinne),
      ],
      viewport,
      previous_frame_end: Some(previous_frame_end),
    })
  }
}
