use crate::engine::{handle_result, ASingleton, AssociatedResource, PipelineRunner, Resultat, ANamedSingleton};
use bevy_app::{App, Plugin, PreUpdate};
use bevy_ecs::prelude::*;
use smallvec::smallvec;
use std::error::Error;
use std::io::Cursor;
use std::sync::Arc;
use bevy_ecs::system::SystemId;
use itertools::Itertools;
use vulkano::buffer::{Buffer, BufferContents, BufferCreateInfo, BufferUsage, Subbuffer};
use vulkano::command_buffer::allocator::StandardCommandBufferAllocator;
use vulkano::command_buffer::{AutoCommandBufferBuilder, CommandBufferUsage, CopyBufferToImageInfo, PrimaryAutoCommandBuffer, PrimaryCommandBufferAbstract, RenderingAttachmentInfo, RenderingInfo};
use vulkano::descriptor_set::allocator::{StandardDescriptorSetAllocator, StandardDescriptorSetAllocatorCreateInfo};
use vulkano::descriptor_set::{PersistentDescriptorSet, WriteDescriptorSet};
use vulkano::descriptor_set::layout::DescriptorSetLayout;
use vulkano::device::{Device, Queue};
use vulkano::format::{Format};
use vulkano::image::sampler::{Filter, Sampler, SamplerAddressMode, SamplerCreateInfo};
use vulkano::image::view::ImageView;
use vulkano::image::{Image, ImageCreateInfo, ImageType, ImageUsage};
use vulkano::memory::allocator::{AllocationCreateInfo, MemoryTypeFilter, StandardMemoryAllocator};
use vulkano::pipeline::graphics::color_blend::{AttachmentBlend, ColorBlendAttachmentState, ColorBlendState};
use vulkano::pipeline::graphics::input_assembly::{InputAssemblyState, PrimitiveTopology};
use vulkano::pipeline::graphics::multisample::MultisampleState;
use vulkano::pipeline::graphics::rasterization::RasterizationState;
use vulkano::pipeline::graphics::subpass::PipelineRenderingCreateInfo;
use vulkano::pipeline::graphics::vertex_input::{Vertex, VertexDefinition};
use vulkano::pipeline::graphics::viewport::{Viewport, ViewportState};
use vulkano::pipeline::graphics::GraphicsPipelineCreateInfo;
use vulkano::pipeline::layout::PipelineDescriptorSetLayoutCreateInfo;
use vulkano::pipeline::{
  DynamicState, GraphicsPipeline, Pipeline, PipelineBindPoint, PipelineLayout, PipelineShaderStageCreateInfo,
};
use vulkano::shader::EntryPoint;
use vulkano::swapchain::{Surface, Swapchain};
use vulkano::DeviceSize;
use vulkano::render_pass::{AttachmentLoadOp, AttachmentStoreOp};
use crate::assets::Asset;
use crate::systems::MySystemId;

pub type TileCoord = (u16, u16);

pub struct SpriteAtlas {
  pub image_asset: Entity,
  pub size_x: u16,
  pub size_y: u16
}

#[derive(Resource)]
pub struct TileDrawCommands {
  pub vec: Vec<(Entity, Subbuffer<[MVertex]>)>
}

#[derive(Component)]
pub struct Tile {
  pub tile_sheet_entity: Entity,
  pub coord: TileCoord
}

#[derive(Component)]
pub struct OnKeyPress(pub Option<MySystemId>);

#[derive(Component)]
pub struct Cell;

#[derive(Component)]
pub struct Transform {
  pub x: f32,
  pub y: f32,
}

pub struct TilemapPipeline;

#[derive(BufferContents, Vertex)]
#[repr(C)]
pub struct MVertex {
  #[format(R32G32_SFLOAT)]
  in_coord: [f32; 2],
  #[format(R16G16_UINT)]
  tile: [u16; 2],
}

mod vs {
  vulkano_shaders::shader! {
      ty: "vertex",
      path: "assets/shaders/vertex.glsl"
  }
}

mod fs {
  vulkano_shaders::shader! {
      ty: "fragment",
      path: "assets/shaders/fragment.glsl"
  }
}

impl TilemapPipeline {
  fn shaders(device: Arc<Device>) -> Result<(EntryPoint, EntryPoint), Box<dyn Error>> {
    let vs = vs::load(device.clone())?
      .specialize([(0, 49u32.into()), (1, 22u32.into())].into_iter().collect())?
      .entry_point("main")
      .unwrap();
    let fs = fs::load(device)?.entry_point("main").unwrap();
    Ok((vs, fs))
  }

  fn vertex_buffer(memory_allocator: Arc<StandardMemoryAllocator>) -> Result<Subbuffer<[MVertex]>, Box<dyn Error>> {
    let vertices = [
      MVertex {
        in_coord: [0.0, -0.1],
        tile: [11, 0],
      },
      MVertex {
        in_coord: [0.0, -0.3],
        tile: [11, 0],
      },
      MVertex {
        in_coord: [0.3, -0.4],
        tile: [11, 0],
      },
    ];
    let vertex_buffer = Buffer::from_iter(
      memory_allocator,
      BufferCreateInfo {
        usage: BufferUsage::VERTEX_BUFFER,
        ..Default::default()
      },
      AllocationCreateInfo {
        memory_type_filter: MemoryTypeFilter::HOST_RANDOM_ACCESS,
        ..Default::default()
      },
      vertices,
    )?;
    Ok(vertex_buffer)
  }

  fn sampler(
    device: Arc<Device>,
    queue: Arc<Queue>,
    pipeline: Arc<GraphicsPipeline>,
    memory_allocator: Arc<StandardMemoryAllocator>,
    command_buffer_allocator: Arc<StandardCommandBufferAllocator>,
    descriptor_set_allocator: Arc<StandardDescriptorSetAllocator>,
  ) -> Result<(Arc<DescriptorSetLayout>, Arc<PersistentDescriptorSet>), Box<dyn Error>> {
    let mut uploads = AutoCommandBufferBuilder::primary(
      &command_buffer_allocator,
      queue.queue_family_index(),
      CommandBufferUsage::OneTimeSubmit,
    )?;

    let texture = {
      let png_bytes = include_bytes!("../../assets/tiles.png").to_vec();
      let cursor = Cursor::new(png_bytes);
      let decoder = png::Decoder::new(cursor);
      let mut reader = decoder.read_info()?;
      let info = reader.info();
      let dimensions = [info.width, info.height, 1];

      let upload_buffer = Buffer::new_slice(
        memory_allocator.clone(),
        BufferCreateInfo {
          usage: BufferUsage::TRANSFER_SRC,
          ..Default::default()
        },
        AllocationCreateInfo {
          memory_type_filter: MemoryTypeFilter::PREFER_HOST | MemoryTypeFilter::HOST_SEQUENTIAL_WRITE,
          ..Default::default()
        },
        (info.width * info.height * 4) as DeviceSize,
      )?;

      reader.next_frame(&mut upload_buffer.write().unwrap()).unwrap();
      let image = Image::new(
        memory_allocator,
        ImageCreateInfo {
          usage: ImageUsage::TRANSFER_DST | ImageUsage::SAMPLED,
          format: Format::B8G8R8A8_UNORM,
          image_type: ImageType::Dim2d,
          extent: dimensions,
          ..Default::default()
        },
        AllocationCreateInfo {
          memory_type_filter: MemoryTypeFilter::PREFER_DEVICE,
          ..Default::default()
        },
      )?;

      uploads.copy_buffer_to_image(CopyBufferToImageInfo::buffer_image(upload_buffer, image.clone()))?;
      ImageView::new_default(image)?
    };

    let sampler = Sampler::new(
      device,
      SamplerCreateInfo {
        mag_filter: Filter::Nearest,
        min_filter: Filter::Nearest,
        address_mode: [SamplerAddressMode::Repeat; 3],
        ..Default::default()
      },
    )?;

    let layout = pipeline.layout().set_layouts().get(0).unwrap().clone();
    let set = PersistentDescriptorSet::new(
      &descriptor_set_allocator,
      layout.clone(),
      [WriteDescriptorSet::image_view_sampler(0, texture, sampler)],
      [],
    )?;

    let _future = uploads.build()?.execute(queue)?;

    Ok((layout, set))
  }

  fn pipeline(
    device: Arc<Device>,
    swapchain: Arc<Swapchain>,
    vs: EntryPoint,
    fs: EntryPoint,
  ) -> Result<Arc<GraphicsPipeline>, Box<dyn Error>> {
    let vertex_input_state = MVertex::per_instance().definition(&vs.info().input_interface)?;
    let stages = smallvec![
      PipelineShaderStageCreateInfo::new(vs),
      PipelineShaderStageCreateInfo::new(fs),
    ];
    let layout = PipelineLayout::new(
      device.clone(),
      PipelineDescriptorSetLayoutCreateInfo::from_stages(&stages).into_pipeline_layout_create_info(device.clone())?,
    )?;
    let subpass = PipelineRenderingCreateInfo {
      color_attachment_formats: vec![Some(swapchain.image_format())],
      ..Default::default()
    };
    let pipeline = GraphicsPipeline::new(
      device.clone(),
      None,
      GraphicsPipelineCreateInfo {
        stages,
        vertex_input_state: Some(vertex_input_state),
        input_assembly_state: Some(InputAssemblyState {
          topology: PrimitiveTopology::TriangleStrip,
          ..Default::default()
        }),
        viewport_state: Some(ViewportState::default()),
        rasterization_state: Some(RasterizationState::default()),
        multisample_state: Some(MultisampleState::default()),
        color_blend_state: Some(ColorBlendState::with_attachment_states(
          subpass.color_attachment_formats.len() as u32,
          ColorBlendAttachmentState {
            blend: Some(AttachmentBlend::alpha()),
            ..Default::default()
          },
        )),
        dynamic_state: [DynamicState::Viewport].into_iter().collect(),
        subpass: Some(subpass.into()),
        ..GraphicsPipelineCreateInfo::layout(layout)
      },
    )?;
    Ok(pipeline)
  }

  fn texture(memory_allocator: Arc<StandardMemoryAllocator>, device: Arc<Device>, pipeline: Arc<GraphicsPipeline>, swapchain: Arc<Swapchain>) -> Resultat<(Arc<ImageView>, Arc<PersistentDescriptorSet>)> {
    let descriptor_set_allocator =
      StandardDescriptorSetAllocator::new(device.clone(), StandardDescriptorSetAllocatorCreateInfo::default());
    let image = Image::new(
      memory_allocator,
      ImageCreateInfo {
        usage: ImageUsage::COLOR_ATTACHMENT | ImageUsage::SAMPLED,
        format: swapchain.image_format(),
        image_type: ImageType::Dim2d,
        extent: [800, 600, 1],
        ..Default::default()
      },
      AllocationCreateInfo {
        memory_type_filter: MemoryTypeFilter::PREFER_DEVICE,
        ..Default::default()
      },
    )?;

    let image_view = ImageView::new_default(image).unwrap();

    let sampler = Sampler::new(
      device,
      SamplerCreateInfo {
        mag_filter: Filter::Linear,
        min_filter: Filter::Linear,
        address_mode: [SamplerAddressMode::Repeat; 3],
        ..Default::default()
      },
    )?;

    let layout = pipeline.layout().set_layouts().get(0).unwrap();
    let set = PersistentDescriptorSet::new(
      &descriptor_set_allocator,
      layout.clone(),
      [WriteDescriptorSet::image_view_sampler(0, image_view.clone(), sampler)],
      [],
    )?;

    Ok((image_view, set))
  }

  fn init(app: &mut App) -> Resultat<()> {
    let device = app.world.resource::<ASingleton<Device>>();
    let swapchain = app.world.resource::<ASingleton<Swapchain>>();
    let queue = app.world.resource::<ASingleton<Queue>>();
    let descriptor_set_allocator = app.world.resource::<ASingleton<StandardDescriptorSetAllocator>>();
    let command_buffer_allocator = app.world.resource::<ASingleton<StandardCommandBufferAllocator>>();

    let memory_allocator = Arc::new(StandardMemoryAllocator::new_default(device.clon()));
    let (vs, fs) = Self::shaders(device.clon())?;
    let pipeline = Self::pipeline(device.clon(), swapchain.clon(), vs, fs)?;
    let (descriptor_set_layout, descriptor_set) = Self::sampler(device.clon(), queue.clon(), pipeline.clone(), memory_allocator.clone(), command_buffer_allocator.clon(), descriptor_set_allocator.clon())?;
    let vertex_buffer = Self::vertex_buffer(memory_allocator.clone())?;
    let (image_view, set) = Self::texture(memory_allocator, device.clon(), pipeline.clone(), swapchain.clon())?;

    app.insert_resource(ASingleton(image_view));
    app.insert_resource(ANamedSingleton::<"X", _>(set));
    app.insert_resource(ANamedSingleton::<"Sampler", _>(descriptor_set_layout));
    app.insert_resource(AssociatedResource::<Self, _>::new(pipeline));
    app.insert_resource(AssociatedResource::<Self, _>::new(vertex_buffer));
    app.insert_resource(AssociatedResource::<Self, _>::new(descriptor_set));
    Ok(())
  }

  fn regenerate_vertex_buffer(
    mut commands: Commands,
    memory_allocator: Res<ASingleton<StandardMemoryAllocator>>,
    cells: Query<(&Transform, &Tile), With<Cell>>,
  ) -> Resultat<()> {
    let mut draw_commands = vec![];
    cells
      .iter()
      .sorted_by(|a, b| a.1.tile_sheet_entity.cmp(&b.1.tile_sheet_entity))
      .group_by(|a| a.1.tile_sheet_entity)
      .into_iter()
      .map(|(entity, group)| {
        let vert = group.map(|(transform, tile)|
          MVertex {
            in_coord: [transform.x, transform.y],
            tile: [tile.coord.0, tile.coord.1]
          }).collect::<Vec<_>>();
        Buffer::from_iter(
          memory_allocator.clon(),
          BufferCreateInfo {
            usage: BufferUsage::VERTEX_BUFFER,
            ..Default::default()
          },
          AllocationCreateInfo {
            memory_type_filter: MemoryTypeFilter::HOST_RANDOM_ACCESS,
            ..Default::default()
          },
          vert,
        ).ok().map(|b| (entity, b))
      }).filter_map(|x|x).for_each(|(entity, buffer)| {
        draw_commands.push((entity, buffer))
    });
    // if verticies.len() == 0 {
    //   return Ok(());
    // }
    // commands.insert_resource(AssociatedResource::<Self, _>::new(vertex_buffer));
    commands.insert_resource(TileDrawCommands { vec: draw_commands });
    Ok(())
  }

  fn bind(
    mut builder: NonSendMut<AutoCommandBufferBuilder<PrimaryAutoCommandBuffer>>,
    pipeline: Res<AssociatedResource<Self, Arc<GraphicsPipeline>>>,
    descriptor_set: Res<AssociatedResource<Self, Arc<PersistentDescriptorSet>>>,
    tile_draw_commands: Res<TileDrawCommands>,
    // game_viewport: Res<GameViewport>,
    image_view: Res<ASingleton<ImageView>>,
    query: Query<&Asset<Image>>,
    sprite_atlases: Query<&Asset<SpriteAtlas>>
  ) -> Resultat<()> {
    let builder = builder
      .begin_rendering(RenderingInfo {
        color_attachments: vec![Some(RenderingAttachmentInfo {
          load_op: AttachmentLoadOp::Clear,
          store_op: AttachmentStoreOp::Store,
          clear_value: Some([0.0, 0.1, 0.1, 1.0].into()),
          ..RenderingAttachmentInfo::image_view(image_view.clon())
        })],
        ..Default::default()
      })?
      .bind_pipeline_graphics(pipeline.clone())?;
      // .clear_attachments(smallvec![ClearAttachment::Color {clear_value: ClearColorValue::Float([0.1, 0.0, 0.0, 1.0]), color_attachment: 0}], smallvec![ClearRect { offset: [game_viewport.pos[0] as u32, game_viewport.pos[1] as u32], extent: [game_viewport.size[0] as u32, game_viewport.size[1] as u32], array_layers: 0..1 }])?
    for (entity, buffer) in &tile_draw_commands.vec {
      let sprite_atlas = &sprite_atlases.get(*entity).unwrap().data;
      let descriptor_set = query.get(sprite_atlas.image_asset)?.data.clone();
      builder
        .bind_descriptor_sets(
          PipelineBindPoint::Graphics,
          pipeline.layout().clone(),
          0,
          descriptor_set.clone(),
        )?
        .bind_vertex_buffers(0, buffer.clone())?
        .push_constants(pipeline.layout().clone(), 0, vs::Constants { offset: [0.2, 0.1], tile_size_x: sprite_atlas.size_x as u32, tile_size_y: sprite_atlas.size_y as u32 })?
        // .set_viewport(0, smallvec![Viewport { offset: game_viewport.pos, extent: game_viewport.size, depth_range: 0.0..=1.0 }])?
        // .set_viewport(0, smallvec![viewport.0.clone()])?
        .set_viewport(0, smallvec![Viewport { offset: [0.0, 0.0], extent: [800.0, 600.0], depth_range: 0.0..=1.0 }])?
        .draw(4, buffer.len() as u32, 0, 0)?;
    }
    Ok(())
  }
}

impl Plugin for TilemapPipeline {
  fn build(&self, app: &mut App) {
    TilemapPipeline::init(app).unwrap();
    let s = app.world.register_system(TilemapPipeline::bind.pipe(handle_result)); app.world.resource_mut::<PipelineRunner>().order.push(s);
    app.add_systems(PreUpdate, TilemapPipeline::regenerate_vertex_buffer.pipe(handle_result));
    // app.world.spawn((Cell, Transform { x: 0.5, y: 0.5 }, OnKeyPress(None)));
    // app.world.spawn((Cell, Transform { x: 0.3, y: 0.2 }, OnKeyPress(None)));
  }
}
