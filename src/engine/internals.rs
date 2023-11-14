use crate::engine::{ASingleton, PipelineRunner, Singleton};
use bevy_app::App;
use std::error::Error;
use std::sync::Arc;
use vulkano::device::physical::PhysicalDeviceType;
use vulkano::device::{Device, DeviceCreateInfo, DeviceExtensions, Features, Queue, QueueCreateInfo, QueueFlags};
use vulkano::image::{Image, ImageUsage};
use vulkano::instance::{Instance, InstanceCreateInfo};
use vulkano::memory::allocator::StandardMemoryAllocator;
use vulkano::swapchain::{PresentMode, Surface, Swapchain, SwapchainCreateInfo};
use vulkano::VulkanLibrary;
use vulkano_win::required_extensions;
use winit::event_loop::EventLoop;
use winit::window::{Window, WindowBuilder};

pub struct RuminativeInternals;

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
  pub fn new_in_app(event_loop: &EventLoop<()>, world: &mut App) -> Result<(), Box<dyn Error>> {
    let instance = Self::instance()?;
    let (device, surface, queue) = Self::device_surface_and_queue(&event_loop, instance)?;
    let memory_allocator = Arc::new(StandardMemoryAllocator::new_default(device.clone()));
    let (swapchain, images) = Self::swapchain_and_images(device.clone(), surface.clone())?;

    world.init_resource::<PipelineRunner>();

    world.insert_resource(ASingleton(device));
    world.insert_resource(ASingleton(surface));
    world.insert_resource(ASingleton(queue));
    world.insert_resource(ASingleton(memory_allocator));
    world.insert_resource(ASingleton(swapchain));
    world.insert_resource(Singleton(images));
    Ok(())
  }
}
