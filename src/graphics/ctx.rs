use std::error::Error;
use std::sync::Arc;
use vulkano::device::physical::PhysicalDeviceType;
use vulkano::device::{Device, DeviceCreateInfo, DeviceExtensions, Queue, QueueCreateInfo, QueueFlags};
use vulkano::image::{ImageUsage, SwapchainImage};
use vulkano::instance::{Instance, InstanceCreateInfo};
use vulkano::pipeline::graphics::input_assembly::{InputAssemblyState, PrimitiveTopology};
use vulkano::pipeline::graphics::viewport::{Viewport, ViewportState};
use vulkano::pipeline::GraphicsPipeline;
use vulkano::render_pass::{RenderPass, Subpass};
use vulkano::shader::ShaderModule;
use vulkano::swapchain::{PresentMode, Surface, Swapchain, SwapchainCreateInfo};
use vulkano::VulkanLibrary;
use vulkano_win::{required_extensions, VkSurfaceBuild};
use winit::event_loop::EventLoop;
use winit::window::{Window, WindowBuilder};

pub struct RenderingContext {
  pub event_loop: EventLoop<()>,
  pub device: Arc<Device>,
  pub images: Vec<Arc<SwapchainImage>>,
  pub render_pass: Arc<RenderPass>,
  pub viewport: Viewport,
  pub surface: Arc<Surface>,
  pub swapchain: Arc<Swapchain>,
  pub queue: Arc<Queue>,
  pub pipeline: Arc<GraphicsPipeline>,
}

impl RenderingContext {
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
  fn device_surface_and_queue(event_loop: &EventLoop<()>, instance: Arc<Instance>) -> Result<(Arc<Device>, Arc<Surface>, Arc<Queue>), Box<dyn Error>> {
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
        ..Default::default()
      },
    )?;

    let queue = queues.next().ok_or("No queue")?;
    Ok((device, surface, queue))
  }
  fn swapchain_and_images(device: Arc<Device>, surface: Arc<Surface>) -> Result<(Arc<Swapchain>, Vec<Arc<SwapchainImage>>), Box<dyn Error>> {
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
  fn shaders(device: Arc<Device>) -> Result<(Arc<ShaderModule>, Arc<ShaderModule>), Box<dyn Error>> {
    mod vs {
      vulkano_shaders::shader! {
          ty: "vertex",
          src: r"
#version 450

layout(location = 0) out vec2 tex_coords;

void main() {
  vec2 pos = vec2(gl_VertexIndex / 2, gl_VertexIndex % 2);
  gl_Position = vec4(pos * 2.0 - 1.0, 0.0, 1.0);
  tex_coords = pos;
}",
      }
    }

    mod fs {
      vulkano_shaders::shader! {
          ty: "fragment",
          src: r"
#version 450

layout(location = 0) in vec2 tex_coords;
layout(location = 0) out vec4 f_color;

layout(set = 0, binding = 0) uniform sampler2D tex;

void main() {
  vec4 tex = texture(tex, tex_coords);
  f_color = tex;
}",
      }
    }

    let vs = vs::load(device.clone())?;
    let fs = fs::load(device.clone())?;
    Ok((vs, fs))
  }
  fn render_pass_and_pipeline(device: Arc<Device>, swapchain: Arc<Swapchain>, vs: Arc<ShaderModule>, fs: Arc<ShaderModule>) -> Result<(Arc<RenderPass>, Arc<GraphicsPipeline>), Box<dyn Error>> {
    let render_pass = vulkano::single_pass_renderpass!(device.clone(), attachments: {
      color: {
        load: Clear,
        store: Store,
        format: swapchain.image_format(),
        samples: 1
      }
    }, pass: {
      color: [color],
      depth_stencil: {}
    })?;

    let pipeline = GraphicsPipeline::start()
      .render_pass(Subpass::from(render_pass.clone(), 0).ok_or("Can't create subpass")?)
      .input_assembly_state(InputAssemblyState::new().topology(PrimitiveTopology::TriangleStrip))
      .vertex_shader(
        vs.entry_point("main").ok_or("No main entry point in vertex shader")?,
        (),
      )
      .fragment_shader(
        fs.entry_point("main").ok_or("No main entry point in fragment shader")?,
        (),
      )
      .viewport_state(ViewportState::viewport_dynamic_scissor_irrelevant())
      .build(device.clone())?;
    Ok((render_pass, pipeline))
  }

  pub fn new() -> Result<Self, Box<dyn Error>> {
    let event_loop = EventLoop::new();
    let instance = Self::instance()?;
    let (device, surface, queue) = Self::device_surface_and_queue(&event_loop, instance.clone())?;
    let (swapchain, images) = Self::swapchain_and_images(device.clone(), surface.clone())?;
    let (vs, fs) = Self::shaders(device.clone())?;
    let (render_pass, pipeline) = Self::render_pass_and_pipeline(device.clone(), swapchain.clone(), vs.clone(), fs.clone())?;

    let viewport = Viewport {
      origin: [0.0, 0.0],
      dimensions: [0.0, 0.0],
      depth_range: 0.0..1.0,
    };

    Ok(RenderingContext {
      event_loop,
      device,
      images,
      render_pass,
      viewport,
      surface,
      swapchain,
      queue,
      pipeline,
    })
  }
}
