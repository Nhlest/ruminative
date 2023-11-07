use smallvec::smallvec;
use std::error::Error;
use std::sync::Arc;
use vulkano::command_buffer::allocator::StandardCommandBufferAllocator;

use crate::engine::Ruminative;
use vulkano::command_buffer::{AutoCommandBufferBuilder, CommandBufferUsage, RenderingAttachmentInfo, RenderingInfo};
use vulkano::image::view::ImageView;
use vulkano::image::Image;
use vulkano::pipeline::graphics::viewport::Viewport;
use vulkano::render_pass::{AttachmentLoadOp, AttachmentStoreOp};
use vulkano::swapchain::{acquire_next_image, SwapchainCreateInfo, SwapchainPresentInfo};
use vulkano::sync::GpuFuture;
use vulkano::{sync, Validated, VulkanError};
use winit::event::{Event, WindowEvent};
use winit::event_loop::ControlFlow;
use winit::window::Window;

fn window_size_dependent_setup(images: &[Arc<Image>], viewport: &mut Viewport) -> Vec<Arc<ImageView>> {
  let dimensions = images[0].extent();
  viewport.extent = [dimensions[0] as f32, dimensions[1] as f32];

  images
    .iter()
    .map(|image| ImageView::new_default(image.clone()).unwrap())
    .collect::<Vec<_>>()
}

pub fn run(mut ctx: Ruminative) -> Result<(), Box<dyn Error>> {
  let mut images = window_size_dependent_setup(&ctx.internals.images, &mut ctx.viewport);

  let command_buffer_allocator = StandardCommandBufferAllocator::new(ctx.internals.device.clone(), Default::default());

  let mut recreate_swapchain = false;

  ctx.previous_frame_end = Some(sync::now(ctx.internals.device.clone()).boxed());
  ctx.event_loop.run(move |event, _, control_flow| {
    let window = ctx
      .internals
      .surface
      .object()
      .unwrap()
      .downcast_ref::<Window>()
      .unwrap();
    ctx
      .pipelines
      .iter_mut()
      .for_each(|pipeline| pipeline.handle_event(window, &event));
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
        let window = ctx
          .internals
          .surface
          .object()
          .unwrap()
          .downcast_ref::<Window>()
          .unwrap();
        let dimensions = window.inner_size();
        if dimensions.width == 0 || dimensions.height == 0 {
          return;
        }

        ctx.previous_frame_end.as_mut().unwrap().cleanup_finished();

        if recreate_swapchain {
          let (new_swapchain, new_images) = match ctx.internals.swapchain.recreate(SwapchainCreateInfo {
            image_extent: dimensions.into(),
            ..ctx.internals.swapchain.create_info()
          }) {
            Ok(r) => r,
            Err(e) => panic!("failed to recreate swapchain: {e}"),
          };

          ctx.internals.swapchain = new_swapchain;

          images = window_size_dependent_setup(&new_images, &mut ctx.viewport);

          recreate_swapchain = false;
        }

        let (image_index, suboptimal, acquire_future) =
          match acquire_next_image(ctx.internals.swapchain.clone(), None).map_err(Validated::unwrap) {
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
          ctx.internals.queue.queue_family_index(),
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
          .set_viewport(0, smallvec![ctx.viewport.clone()])
          .unwrap();

        ctx.pipelines.iter_mut().for_each(|pipeline| {
          pipeline.update(&ctx.internals);
          pipeline.bind(&mut builder, window).unwrap();
        });

        builder.end_rendering().unwrap();

        let command_buffer = builder.build().unwrap();

        let future = ctx
          .previous_frame_end
          .take()
          .unwrap()
          .join(acquire_future)
          .then_execute(ctx.internals.queue.clone(), command_buffer)
          .unwrap()
          .then_swapchain_present(
            ctx.internals.queue.clone(),
            SwapchainPresentInfo::swapchain_image_index(ctx.internals.swapchain.clone(), image_index),
          )
          .then_signal_fence_and_flush();

        match future.map_err(Validated::unwrap) {
          Ok(future) => {
            ctx.previous_frame_end = Some(future.boxed());
          }
          Err(VulkanError::OutOfDate) => {
            recreate_swapchain = true;
            ctx.previous_frame_end = Some(sync::now(ctx.internals.device.clone()).boxed());
          }
          Err(e) => {
            dbg!(&e);
            panic!("failed to flush future: {e}");
          }
        }
      }
      _ => (),
    }
  });
}
