use std::any::TypeId;
use crate::systems::{MySystemId, SystemStorage};
use std::mem::ManuallyDrop;
use bevy_ecs::prelude::*;
use bevy_app::prelude::*;
use bevy_ecs::system::SystemId;
use imgui::{Context, DragDropFlags, StyleVar, TextureId, Ui};
use imgui_sys::ImGuiContext;
use vulkano::swapchain::Surface;
use crate::engine::ASingleton;
use crate::engine::tilemap_pipeline::{Cell, OnKeyPress, SpriteAtlas, Tile, Transform};
use crate::systems::CommandsExtension;
use rfd::FileDialog;
use vulkano::image::Image;
use crate::assets::{Asset, Assets, LoadAssetCommand};

#[derive(Resource)]
pub struct SpriteAtlasUiOpened {
  entity: Entity
}

pub fn sprite_atlas_ui(
  mut commands: Commands,
  mut imgui: NonSendMut<Context>,
  image_assets: Res<Assets<Image>>,
  mut sprite_atlas_assets: Query<&mut Asset<SpriteAtlas>>,
  sprite_atlas_ui_opened: Option<Res<SpriteAtlasUiOpened>>,
) {
  let ui = imgui.current_frame();
  if let Some(sprite_atlas_ui_opened) = sprite_atlas_ui_opened {
    let entity = sprite_atlas_ui_opened.entity;
    let mut sprite_atlas = sprite_atlas_assets.get_mut(entity).unwrap();
    let SpriteAtlas {
      image_asset, size_x, size_y
    } = &mut sprite_atlas.as_mut().data;
    ui.window("Sprite Atlas")
      .build(|| {
        ui.slider("SX", 1, 100, size_x);
        ui.slider("SY", 1, 100, size_y);
        let t = ui.push_style_var(StyleVar::WindowPadding([0.0, 0.0]));
        let t = ui.push_style_var(StyleVar::CellPadding([0.0, 0.0]));
        let t = ui.push_style_var(StyleVar::FramePadding([0.0, 0.0]));
        let t = ui.push_style_var(StyleVar::ItemSpacing([1.0, 1.0]));
        for iy in 0..*size_y {
          for ix in 0..*size_x {
            if ui.image_button_config(format!("{} {}", ix, iy), TextureId::new(image_asset.to_bits() as usize), [512.0 / *size_x as f32, 512.0 / *size_y as f32])
              .uv0([ix as f32 * 1.0 / *size_x as f32, iy as f32 * 1.0 / *size_y as f32])
              .uv1([(ix+1) as f32 * 1.0 / *size_x as f32, (iy+1) as f32 * 1.0 / *size_y as f32])
              .build() {
              commands.spawn((Cell, Transform { x: 0.5, y: 0.5 }, OnKeyPress(None), Tile {
                tile_sheet_entity: entity,
                coord: (ix as u16, iy as u16),
              }));
            }
            if ix < *size_x - 1 {
              ui.same_line();
            }
          }
        }
      });
  }
}

pub fn sprite_atlas_assets_ui(
  mut commands: Commands,
  mut imgui: NonSendMut<Context>,
  assets: Res<Assets<SpriteAtlas>>,
  sprite_atlases: Query<&Asset<SpriteAtlas>>
) {
  let ui = imgui.current_frame();
  ui.window("Sprite Atlas Assets")
    .build(|| {
      for (key, entity) in assets.iter() {
        let sprite_atlas = &sprite_atlases.get(*entity).unwrap().data;
        if let Some(s) = ui.tree_node(format!("{:?}", key)) {
          if ui.image_button(format!("{:?}", key), TextureId::new(sprite_atlas.image_asset.to_bits() as usize), [200.0, 200.0]) {
            commands.insert_resource(SpriteAtlasUiOpened {
              entity: *entity,
            })
          }
        }
      }
    });
}

pub fn image_assets_ui(
  mut commands: Commands,
  mut imgui: NonSendMut<Context>,
  assets: Res<Assets<Image>>,
  sprite_atlases: Query<&Asset<SpriteAtlas>>
) {
  let ui = imgui.current_frame();
  ui.window("Assets")
    .build(|| {
      if ui.button("Load Image") {
        let file = FileDialog::new()
          .add_filter("image", &["png"])
          .set_directory("assets")
          .pick_file();
        if let Some(file) = file {
          commands.add(LoadAssetCommand::<Image>::new(file.as_os_str().to_str().unwrap().to_string(), (TypeId::of::<Image>(), file.file_name().unwrap().to_str().unwrap().to_owned())));
        }
      }
      for (key, entity) in assets.iter() {
        if let Some(s) = ui.tree_node(format!("{:?}", key)) {
          if ui.image_button(format!("{:?}", key), TextureId::new(entity.to_bits() as usize), [200.0, 200.0]) {
            commands.add(LoadAssetCommand::<SpriteAtlas>::new(*entity, (TypeId::of::<i32>(), format!("{:?} Sprite Atlas", key).to_string())));
          }
          // imgui::Image::new(, )
          //   .uv0([0.0, 0.0])
          //   .uv1([1.0, 1.0])
          //   .build(ui);
        }
      }
    });
}

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
  // ui.show_demo_window(&mut true);

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

  // unsafe { igSetCurrentContext(ptr::null_mut()) };
  // unsafe {
  //   let dock_id = imgui_sys::igGetID_Str(CString::new("Dockspace").unwrap().as_ptr());
  //   let _dock = imgui_sys::igDockBuilderGetNode(dock_id);
  // }
  // imgui::Pan
  // ui.window("Bottom panel").dock
}
