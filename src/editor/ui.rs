use crate::systems::{MySystemId, SystemStorage};
use std::mem::ManuallyDrop;
use bevy_ecs::prelude::*;
use bevy_app::prelude::*;
use bevy_ecs::system::SystemId;
use imgui::{Context, DragDropFlags, Image, StyleVar, TextureId, Ui};
use imgui_sys::ImGuiContext;
use vulkano::swapchain::Surface;
use crate::engine::ASingleton;
use crate::engine::tilemap_pipeline::{Cell, OnKeyPress, Transform};
use crate::systems::CommandsExtension;

pub fn systems_ui(
  mut commands: Commands,
  mut imgui: NonSendMut<Context>,
  mut systems: ResMut<SystemStorage>
) {
  let ui = imgui.current_frame();
  ui.window("System storage")
    .build(|| {
      for (system_id, system_name) in systems.s.iter_mut() {
        if ui.button(format!("System {}", system_name)) {
          commands.my_run_system(*system_id, 5);
        }
        let drag_source = ui.drag_drop_source_config("SYSTEM");
        if let Some(d) = drag_source.begin_payload(*system_id) {
          ui.text(format!("{}", system_name))
        }
      }
    });
}

pub fn inspector_ui(
  mut imgui: NonSendMut<Context>,
  mut cells: Query<(Entity, &mut Transform, &mut OnKeyPress), With<Cell>>,
  system_storage: Res<SystemStorage>
) {
  let ui = imgui.current_frame();
  ui.window("Inspector")
    .build(|| {
      for (entity, mut transform, mut okp) in cells.iter_mut() {
        if let Some(a) = ui.tree_node(format!("Entity {:?}", entity)) {
          ui.slider("X", -1.0, 1.0, &mut transform.x);
          ui.slider("Y", -1.0, 1.0, &mut transform.y);
          ui.separator();
          ui.text("OnKeyPress");
          ui.same_line();
          if let Some(okp_id) = okp.0 {
            if ui.button(format!("{}", system_storage.get(&okp_id).unwrap())) {
              *okp = OnKeyPress(None);
            }
          } else {
            ui.button("---");
          }
          if let Some(drop) = ui.drag_drop_target() {
            if let Some(Ok(p)) = drop.accept_payload::<MySystemId, _>("SYSTEM", DragDropFlags::empty()) {
              *okp = OnKeyPress(Some(p.data));
            }
          }
        }

        // igFold
      }
    });
}

pub fn editor_ui(
  // mut game_viewport: ResMut<GameViewport>,
  // mut commands: Commands,
  mut imgui: NonSendMut<Context>,
  // mut ctx: NonSendMut<*mut ImGuiContext>,
  // mut ui: NonSendMut<*mut Ui>,
  // surface: Res<ASingleton<Surface>>,
) {
  let ui = imgui.current_frame();
  // unsafe { igSetCurrentContext(ctx.cast()) } ;

  // let window = surface.object().unwrap().downcast_ref::<Window>().unwrap();
  // let scale_factor = window.scale_factor();
  // return;

  // return;
  // let ui : &mut Ui = unsafe { &mut*ui.cast() };
  ui.show_demo_window(&mut true);

  let t = ui.push_style_var(StyleVar::WindowPadding([0.0, 0.0]));
  ui.window("viewport")
    .bg_alpha(0.0)
    .resizable(true)
    .build(|| {
      let mut min = ui.window_content_region_min();
      let mut max = ui.window_content_region_max();

      let x = max[0]-min[0];
      let y = max[1]-min[1];
      Image::new(TextureId::new(1), [x, y])
        .uv0([0.0, 0.0])
        .uv1([1.0, 1.0])
        .build(ui);
    });

  // unsafe { igSetCurrentContext(ptr::null_mut()) };
  // unsafe {
  //   let dock_id = imgui_sys::igGetID_Str(CString::new("Dockspace").unwrap().as_ptr());
  //   let _dock = imgui_sys::igDockBuilderGetNode(dock_id);
  // }
  // imgui::Pan
  // ui.window("Bottom panel").dock
}
