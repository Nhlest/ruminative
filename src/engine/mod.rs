use std::sync::Arc;
use vulkano::buffer::Subbuffer;
use vulkano::descriptor_set::PersistentDescriptorSet;
use vulkano::device::{Device, Queue};
use vulkano::image::SwapchainImage;
use vulkano::pipeline::graphics::viewport::Viewport;
use vulkano::pipeline::GraphicsPipeline;
use vulkano::render_pass::RenderPass;
use vulkano::swapchain::{Surface, Swapchain};
use vulkano::sync::GpuFuture;
use winit::event_loop::EventLoop;

pub struct ZaWarudoInternals {
  pub event_loop: EventLoop<()>,
  pub device: Arc<Device>,
  pub queue: Arc<Queue>,
  pub surface: Arc<Surface>,
  pub swapchain: Arc<Swapchain>,
  pub render_pass: Arc<RenderPass>,
  pub images: Vec<Arc<SwapchainImage>>,
}

pub struct ZaWarudoPipeline<T> {
  pub pipeline: Arc<GraphicsPipeline>,
  pub descriptor_sets: Vec<Arc<PersistentDescriptorSet>>,
  pub vertex_buffer: Subbuffer<[T]>,
}

pub struct ZaWarudo<T> {
  pub internals: ZaWarudoInternals,
  pub pipelines: Vec<ZaWarudoPipeline<T>>,
  pub viewport: Viewport,
  pub previous_frame_end: Option<Box<dyn GpuFuture>>,
}
