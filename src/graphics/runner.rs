use std::error::Error;
use std::sync::Arc;
use vulkano::command_buffer::allocator::StandardCommandBufferAllocator;

use crate::RenderingContext;
use vulkano::command_buffer::{AutoCommandBufferBuilder, CommandBufferUsage, RenderPassBeginInfo, SubpassContents};
use vulkano::image::view::ImageView;
use vulkano::image::{ImageAccess, SwapchainImage};
use vulkano::pipeline::graphics::viewport::Viewport;
use vulkano::render_pass::{Framebuffer, FramebufferCreateInfo, RenderPass};
use vulkano::swapchain::{
  acquire_next_image, AcquireError, SwapchainCreateInfo, SwapchainCreationError, SwapchainPresentInfo,
};
use vulkano::sync;
use vulkano::sync::{FlushError, GpuFuture};
use winit::event::{Event, WindowEvent};
use winit::event_loop::ControlFlow;
use winit::window::Window;

fn window_size_dependent_setup(
  images: &[Arc<SwapchainImage>],
  render_pass: Arc<RenderPass>,
  viewport: &mut Viewport,
) -> Vec<Arc<Framebuffer>> {
  let dimensions = images[0].dimensions().width_height();
  viewport.dimensions = [dimensions[0] as f32, dimensions[1] as f32];

  images
    .iter()
    .map(|image| {
      let view = ImageView::new_default(image.clone()).unwrap();
      Framebuffer::new(
        render_pass.clone(),
        FramebufferCreateInfo {
          attachments: vec![view],
          ..Default::default()
        },
      )
      .unwrap()
    })
    .collect::<Vec<_>>()
}

pub fn run(mut ctx: RenderingContext) -> Result<(), Box<dyn Error>> {
  let mut framebuffers = window_size_dependent_setup(&ctx.images, ctx.render_pass.clone(), &mut ctx.viewport);

  let command_buffer_allocator = StandardCommandBufferAllocator::new(ctx.device.clone(), Default::default());

  let mut recreate_swapchain = false;

  let mut previous_frame_end = Some(sync::now(ctx.device.clone()).boxed());

  ctx.event_loop.run(move |event, _, control_flow| match event {
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
      // runner.run(control_flow).unwrap();
      let window = ctx.surface.object().unwrap().downcast_ref::<Window>().unwrap();
      let dimensions = window.inner_size();
      if dimensions.width == 0 || dimensions.height == 0 {
        return;
      }

      previous_frame_end.as_mut().unwrap().cleanup_finished();

      if recreate_swapchain {
        let (new_swapchain, new_images) = match ctx.swapchain.recreate(SwapchainCreateInfo {
          image_extent: dimensions.into(),
          ..ctx.swapchain.create_info()
        }) {
          Ok(r) => r,
          Err(SwapchainCreationError::ImageExtentNotSupported { .. }) => return,
          Err(e) => panic!("failed to recreate swapchain: {e}"),
        };

        ctx.swapchain = new_swapchain;

        framebuffers = window_size_dependent_setup(&new_images, ctx.render_pass.clone(), &mut ctx.viewport);

        recreate_swapchain = false;
      }

      let (image_index, suboptimal, acquire_future) = match acquire_next_image(ctx.swapchain.clone(), None) {
        Ok(r) => r,
        Err(AcquireError::OutOfDate) => {
          recreate_swapchain = true;
          return;
        }
        Err(e) => panic!("failed to acquire next image: {e}"),
      };

      if suboptimal {
        recreate_swapchain = true;
      }

      let mut builder = AutoCommandBufferBuilder::primary(
        &command_buffer_allocator,
        ctx.queue.queue_family_index(),
        CommandBufferUsage::OneTimeSubmit,
      )
      .unwrap();

      builder
        .begin_render_pass(
          RenderPassBeginInfo {
            clear_values: vec![Some([0.0, 0.0, 1.0, 1.0].into())],

            ..RenderPassBeginInfo::framebuffer(framebuffers[image_index as usize].clone())
          },
          SubpassContents::Inline,
        )
        .unwrap()
        .set_viewport(0, [ctx.viewport.clone()])
        .bind_pipeline_graphics(ctx.pipeline.clone())
        // .bind_descriptor_sets(
        //   PipelineBindPoint::Graphics,
        //   ctx.pipeline.layout().clone(),
        //   0,
        //   ctx.descriptor_set.clone(),
        // )
        // .draw(4, 1, 0, 0)
        // .unwrap()
        .end_render_pass()
        .unwrap();

      let command_buffer = builder.build().unwrap();

      let future = previous_frame_end
        .take()
        .unwrap()
        .join(acquire_future)
        .then_execute(ctx.queue.clone(), command_buffer)
        .unwrap()
        .then_swapchain_present(
          ctx.queue.clone(),
          SwapchainPresentInfo::swapchain_image_index(ctx.swapchain.clone(), image_index),
        )
        .then_signal_fence_and_flush();

      match future {
        Ok(future) => {
          previous_frame_end = Some(future.boxed());
        }
        Err(FlushError::OutOfDate) => {
          recreate_swapchain = true;
          previous_frame_end = Some(sync::now(ctx.device.clone()).boxed());
        }
        Err(e) => {
          panic!("failed to flush future: {e}");
        }
      }
    }
    _ => (),
  });
}
