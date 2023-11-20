#![feature(adt_const_params)]

use crate::engine::engine::Ruminative;
use std::error::Error;
use std::ffi::CString;
use bevy_app::Update;
use bevy_ecs::prelude::*;
use imgui::{Context, StyleVar, TextureId};
use vulkano::swapchain::Surface;
use winit::window::Window;
use crate::engine::{ASingleton, GameViewport};

mod engine;

fn gui(
  mut game_viewport: ResMut<GameViewport>,
  mut imgui: NonSendMut<Context>,
  surface: Res<ASingleton<Surface>>
) {
  let window = surface.object().unwrap().downcast_ref::<Window>().unwrap();
  let scale_factor = window.scale_factor();

  let ui = imgui.new_frame();
  {
    ui.dockspace_over_main_viewport();
  }

  let t = ui.push_style_var(StyleVar::WindowPadding([0.0, 0.0]));
  ui.window("viewport")
    .bg_alpha(0.0)
    .resizable(true)
    .build(|| {
      let mut min = ui.window_content_region_min();
      let mut max = ui.window_content_region_max();

      let x = max[0]-min[0];
      let y = max[1]-min[1];
      imgui::Image::new(TextureId::new(1), [x, y])
        .uv0([0.0, 0.0])
        .uv1([1.0, 1.0])
        .build(ui);
  });

  ui.show_demo_window(&mut true);
  unsafe {
    let dock_id = imgui_sys::igGetID_Str(CString::new("Dockspace").unwrap().as_ptr());
    let _dock = imgui_sys::igDockBuilderGetNode(dock_id);
  }
  // imgui::Pan
  // ui.window("Bottom panel").dock
}

fn main() -> Result<(), Box<dyn Error>> {
  let mut ctx = Ruminative::new()?;
  ctx.app.add_systems(Update, gui);
  ctx.app.run();
  Ok(())
}
