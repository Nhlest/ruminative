use crate::engine::imgui_pipeline::ImguiPipeline;
use crate::engine::internals::RuminativeInternals;
use crate::engine::rumigui_pipeline::RumiguiPipeline;
use crate::engine::tilemap_pipeline::TilemapPipeline;
use crate::engine::{ASingleton, PipelineRunner, Singleton, WinitEvent};
use bevy_app::App;
use smallvec::smallvec;
use std::error::Error;
use std::sync::Arc;
use vulkano::command_buffer::allocator::StandardCommandBufferAllocator;
use vulkano::command_buffer::{
  AutoCommandBufferBuilder, CommandBufferUsage, PrimaryAutoCommandBuffer, RenderingAttachmentInfo, RenderingInfo,
};
use vulkano::device::{Device, Queue};
use vulkano::image::view::ImageView;
use vulkano::image::Image;
use vulkano::pipeline::graphics::viewport::Viewport;
use vulkano::render_pass::{AttachmentLoadOp, AttachmentStoreOp};
use vulkano::swapchain::{acquire_next_image, Surface, Swapchain, SwapchainCreateInfo, SwapchainPresentInfo};
use vulkano::sync::GpuFuture;
use vulkano::{sync, Validated, VulkanError};
use winit::event::{Event, WindowEvent};
use winit::event_loop::{ControlFlow, EventLoop};
use winit::window::Window;

pub struct Ruminative {
  pub app: App,
}
fn window_size_dependent_setup(images: &[Arc<Image>], viewport: &mut Viewport) -> Vec<Arc<ImageView>> {
  let dimensions = images[0].extent();
  viewport.extent = [dimensions[0] as f32, dimensions[1] as f32];

  images
    .iter()
    .map(|image| ImageView::new_default(image.clone()).unwrap())
    .collect::<Vec<_>>()
}

impl Ruminative {
  pub fn new() -> Result<Self, Box<dyn Error>> {
    let mut app = App::new();

    app.add_event::<WinitEvent>();

    let event_loop = EventLoop::new();

    RuminativeInternals::new_in_app(&event_loop, &mut app)?;

    app.add_plugins(TilemapPipeline);
    app.add_plugins(RumiguiPipeline);
    app.add_plugins(ImguiPipeline);

    app.insert_non_send_resource(Singleton(event_loop));

    app.set_runner(|mut app| {
      let event_loop = app
        .world
        .remove_non_send_resource::<Singleton<EventLoop<()>>>()
        .unwrap()
        .0;
      let images = &app.world.resource::<Singleton<Vec<Arc<Image>>>>().0;
      // let viewport = &app.world.resource::<Singleton<Vec<Arc<Image>>>>().0;
      let mut viewport = Viewport::default();
      let mut images = window_size_dependent_setup(images, &mut viewport);
      let device = app.world.resource::<ASingleton<Device>>().clon();
      let queue = app.world.resource::<ASingleton<Queue>>().clon();
      let surface = app.world.resource::<ASingleton<Surface>>().clon();
      let command_buffer_allocator = StandardCommandBufferAllocator::new(device.clone(), Default::default());
      let mut recreate_swapchain = false;
      let mut previous_frame_end = Some(sync::now(device.clone()).boxed());

      event_loop.run(move |event, _a, control_flow| {
        match event {
          Event::WindowEvent {
            event: WindowEvent::CloseRequested,
            ..
          } => {
            *control_flow = ControlFlow::Exit;
          }
          Event::WindowEvent {
            event: WindowEvent::Resized(_),
            ..
          } => {
            recreate_swapchain = true;
          }
          Event::RedrawEventsCleared => {
            let window = surface.object().unwrap().downcast_ref::<Window>().unwrap();
            let dimensions = window.inner_size();
            if dimensions.width == 0 || dimensions.height == 0 {
              return;
            }

            previous_frame_end.as_mut().unwrap().cleanup_finished();

            let swapchain = app.world.resource::<ASingleton<Swapchain>>().clon();

            if recreate_swapchain {
              let (new_swapchain, new_images) = match swapchain.recreate(SwapchainCreateInfo {
                image_extent: dimensions.into(),
                ..swapchain.create_info()
              }) {
                Ok(r) => r,
                Err(e) => panic!("failed to recreate swapchain: {e}"),
              };

              app.insert_resource(ASingleton(new_swapchain));

              images = window_size_dependent_setup(&new_images, &mut viewport);

              recreate_swapchain = false;
            }

            let swapchain = app.world.resource::<ASingleton<Swapchain>>().clon();

            let (image_index, suboptimal, acquire_future) =
              match acquire_next_image(swapchain.clone(), None).map_err(Validated::unwrap) {
                Ok(r) => r,
                Err(VulkanError::OutOfDate) => {
                  recreate_swapchain = true;
                  return;
                }
                Err(e) => {
                  panic!("failed to acquire next image: {e}")
                }
              };

            if suboptimal {
              recreate_swapchain = true;
            }
            let mut builder = AutoCommandBufferBuilder::primary(
              &command_buffer_allocator,
              queue.queue_family_index(),
              CommandBufferUsage::OneTimeSubmit,
            )
            .unwrap();
            builder
              .begin_rendering(RenderingInfo {
                color_attachments: vec![Some(RenderingAttachmentInfo {
                  load_op: AttachmentLoadOp::Clear,
                  store_op: AttachmentStoreOp::Store,
                  clear_value: Some([0.0, 0.0, 0.1, 1.0].into()),
                  ..RenderingAttachmentInfo::image_view(images[image_index as usize].clone())
                })],
                ..Default::default()
              })
              .unwrap()
              .set_viewport(0, smallvec![viewport.clone()])
              .unwrap();

            app.insert_non_send_resource(builder);

            app.update();
            for i in app.world.resource::<PipelineRunner>().order.clone() {
              app.world.run_system(i).unwrap();
            }

            let mut builder = app
              .world
              .remove_non_send_resource::<AutoCommandBufferBuilder<PrimaryAutoCommandBuffer>>()
              .unwrap();

            builder.end_rendering().unwrap();

            let command_buffer = builder.build().unwrap();

            let future = previous_frame_end
              .take()
              .unwrap()
              .join(acquire_future)
              .then_execute(queue.clone(), command_buffer)
              .unwrap()
              .then_swapchain_present(
                queue.clone(),
                SwapchainPresentInfo::swapchain_image_index(swapchain.clone(), image_index),
              )
              .then_signal_fence_and_flush();

            match future.map_err(Validated::unwrap) {
              Ok(future) => {
                previous_frame_end = Some(future.boxed());
              }
              Err(VulkanError::OutOfDate) => {
                recreate_swapchain = true;
                previous_frame_end = Some(sync::now(device.clone()).boxed());
              }
              Err(e) => {
                dbg!(&e);
                panic!("failed to flush future: {e}");
              }
            }
          }
          _ => (),
        }
        if let Some(e) = event.to_static() {
          app.world.send_event(WinitEvent(e))
        }
      })
    });

    Ok(Self { app })
  }
}
