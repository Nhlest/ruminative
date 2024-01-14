use bevy_ecs::prelude::*;
use imgui::{Context};

pub fn inspector_ui(
  mut imgui: NonSendMut<Context>,
) {
  let ui = imgui.current_frame();
  ui.window("Inspector")
    .build(|| {
    });
}

pub fn main_menu(
  mut imgui: NonSendMut<Context>,
) {
  let ui = imgui.current_frame();
  ui.main_menu_bar(|| {
    if ui.button("SAVE") {
    }
  });
}