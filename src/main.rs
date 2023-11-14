use crate::engine::engine::Ruminative;
use std::error::Error;
use std::ffi::CString;
use bevy_app::Update;
use bevy_ecs::prelude::*;
use imgui::{Context, StyleVar};
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

  ui.window("viewport")
    .bg_alpha(0.0)
    .build(|| {
      let pos = ui.window_pos();
      let size = ui.window_size();
      game_viewport.pos = [pos[0] * scale_factor as f32, pos[1] * scale_factor as f32];
      game_viewport.size = [size[0] * scale_factor as f32, size[1] * scale_factor as f32];
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
