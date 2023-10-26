use std::error::Error;
use std::sync::Arc;
use vulkano::command_buffer::allocator::StandardCommandBufferAllocator;

use imgui::*;
use vulkano::command_buffer::{AutoCommandBufferBuilder, CommandBufferUsage, RenderingAttachmentInfo, RenderingInfo};
use vulkano::image::view::ImageView;
use vulkano::image::{ImageAccess, SwapchainImage};
use vulkano::pipeline::graphics::viewport::Viewport;
use vulkano::render_pass::{LoadOp, StoreOp};
use vulkano::swapchain::{
  acquire_next_image, AcquireError, SwapchainCreateInfo, SwapchainCreationError, SwapchainPresentInfo,
};
use vulkano::sync;
use vulkano::sync::{FlushError, GpuFuture};
use winit::event::{Event, WindowEvent};
use winit::event_loop::ControlFlow;
use winit::window::Window;
use crate::engine::{Ruminative};

fn window_size_dependent_setup(
  images: &[Arc<SwapchainImage>],
  viewport: &mut Viewport,
) -> Vec<Arc<ImageView<SwapchainImage>>> {
  let dimensions = images[0].dimensions().width_height();
  viewport.dimensions = [dimensions[0] as f32, dimensions[1] as f32];

  images
    .iter()
    .map(|image| ImageView::new_default(image.clone()).unwrap())
    .collect::<Vec<_>>()
}

pub fn run(mut ctx: Ruminative) -> Result<(), Box<dyn Error>> {
  // let mut imgui = Context::create();
  // imgui.fonts().add_font(&[FontSource::DefaultFontData {config: None}]);
  // let tex = imgui.fonts().build_rgba32_texture();
  // imgui.io_mut().display_size = [1024.0, 768.0];
  // {
  //   let a = imgui.new_frame();
  //   a.show_demo_window(&mut true);
  //   a.end_frame_early();
  // }
  // let d = imgui.render();
  // dbg!(&d.total_vtx_count);
  let mut images = window_size_dependent_setup(&ctx.internals.images, &mut ctx.viewport);

  let command_buffer_allocator = StandardCommandBufferAllocator::new(ctx.internals.device.clone(), Default::default());

  let mut recreate_swapchain = false;

  ctx.internals.event_loop.run(move |event, _, control_flow| match event {
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
      let window = ctx.internals.surface.object().unwrap().downcast_ref::<Window>().unwrap();
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
          Err(SwapchainCreationError::ImageExtentNotSupported { .. }) => return,
          Err(e) => panic!("failed to recreate swapchain: {e}"),
        };

        ctx.internals.swapchain = new_swapchain;

        images = window_size_dependent_setup(&new_images, &mut ctx.viewport);

        recreate_swapchain = false;
      }

      let (image_index, suboptimal, acquire_future) = match acquire_next_image(ctx.internals.swapchain.clone(), None) {
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
        ctx.internals.queue.queue_family_index(),
        CommandBufferUsage::OneTimeSubmit,
      ).unwrap();
      builder
        .begin_rendering(RenderingInfo {
          color_attachments: vec![Some(RenderingAttachmentInfo {
            load_op: LoadOp::Clear,
            store_op: StoreOp::Store,
            clear_value: Some([0.0, 0.0, 0.1, 1.0].into()),
            ..RenderingAttachmentInfo::image_view(images[image_index as usize].clone())
          })],
          ..Default::default()
        })
        .unwrap()
        .set_viewport(0, [ctx.viewport.clone()]);

      ctx.pipelines.iter().for_each(|pipeline| { pipeline.bind(&mut builder); });

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

      match future {
        Ok(future) => {
          ctx.previous_frame_end = Some(future.boxed());
        }
        Err(FlushError::OutOfDate) => {
          recreate_swapchain = true;
          ctx.previous_frame_end = Some(sync::now(ctx.internals.device.clone()).boxed());
        }
        Err(e) => {
          panic!("failed to flush future: {e}");
        }
      }
    }
    _ => (),
  });
}
