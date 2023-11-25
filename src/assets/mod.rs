use std::any::TypeId;
use std::collections::HashMap;
use std::io::Cursor;
use std::marker::PhantomData;
use std::ops::Deref;
use std::path::Path;
use std::sync::Arc;
use bevy_app::{App, Plugin};
use bevy_derive::{Deref, DerefMut};
use bevy_ecs::prelude::*;
use bevy_ecs::system::{Command, RunSystemOnce, SystemParam, SystemParamItem, SystemState};
use bevy_ecs::system::lifetimeless::{SCommands, SQuery, SRes};
use itertools::process_results;
use vulkano::buffer::{Buffer, BufferCreateInfo, BufferUsage};
use vulkano::command_buffer::allocator::StandardCommandBufferAllocator;
use vulkano::command_buffer::{AutoCommandBufferBuilder, CommandBufferUsage, CopyBufferToImageInfo, PrimaryCommandBufferAbstract};
use vulkano::descriptor_set::allocator::{StandardDescriptorSetAllocator, StandardDescriptorSetAllocatorCreateInfo};
use vulkano::descriptor_set::layout::DescriptorSetLayout;
use vulkano::descriptor_set::{PersistentDescriptorSet, WriteDescriptorSet};
use vulkano::device::{Device, Queue};
use vulkano::DeviceSize;
use vulkano::format::Format;
use vulkano::image::{Image, ImageCreateInfo, ImageType, ImageUsage};
use vulkano::image::sampler::{Filter, Sampler, SamplerAddressMode, SamplerCreateInfo};
use vulkano::image::view::ImageView;
use vulkano::memory::allocator::{AllocationCreateInfo, MemoryTypeFilter, StandardMemoryAllocator};
use vulkano::pipeline::GraphicsPipeline;
use vulkano::sync::GpuFuture;
use crate::engine::{ANamedSingleton, ASingleton, AssociatedResource, handle_result, NamedSingleton, Resultat};
use crate::engine::tilemap_pipeline::SpriteAtlas;

#[derive(Resource, Deref, DerefMut)]
pub struct Assets<T: Assetable> {
  #[deref]
  storage: HashMap<(TypeId, String), Entity>,
  _s: PhantomData<fn() -> T>
}

impl<T: Assetable> Default for Assets<T> {
  fn default() -> Self {
    Self {
      storage: Default::default(),
      _s: Default::default(),
    }
  }
}

#[derive(Component)]
pub struct Asset<T: Assetable> {
  pub data: T::Storage
}

pub struct LoadAssetCommand<T: Assetable + 'static> {
  source_path: T::SourceType,
  key: AssetKey,
  _s: PhantomData<fn() -> T>
}

impl<T: Assetable + 'static> LoadAssetCommand<T> {
  pub fn new(source_path: T::SourceType, key: AssetKey) -> Self {
    Self {
      source_path,
      key,
      _s: Default::default(),
    }
  }
}

impl<T: Assetable + 'static> Command for LoadAssetCommand<T> {
  fn apply(self, world: &mut World) {
    // let a = IntoSystem::into_system(T::load);
    let mut state: SystemState<T::SystemInput> = SystemState::new(world);
    let v: SystemParamItem<'_, '_, T::SystemInput> = state.get_manual_mut(world);
    T::load((self.source_path, self.key), v).unwrap();
    state.apply(world);
    // world.run_system_once_with((self.file_path, self.key), T::load.pipe(handle_result));
  }
}

type AssetKey = (TypeId, String);

pub struct InsertAssetCommand<T: Assetable + 'static> {
  asset: Entity,
  key: AssetKey,
  _s: PhantomData<fn() -> T>
}

impl<T: Assetable + 'static> InsertAssetCommand<T> {
  pub fn new(asset: Entity, key: AssetKey) -> Self {
    Self {
      asset,
      key,
      _s: Default::default(),
    }
  }
}

impl<T: Assetable + 'static> Command for InsertAssetCommand<T> {
  fn apply(self, world: &mut World) {
    world.resource_mut::<Assets<T>>().insert(self.key, self.asset);
  }
}

pub trait Assetable {
  type Storage;
  type SystemInput: SystemParam + 'static;
  type SourceType : Send;
  fn load<'w>(
    name: (Self::SourceType, AssetKey),
    // commands: Commands,
    system_input: SystemParamItem<'w, '_, Self::SystemInput>,
    // system_input: Self::SystemInput
  ) -> Resultat<()>;
}

impl Assetable for SpriteAtlas {
  type Storage = Self;
  type SystemInput = (
    SCommands,
  );
  type SourceType = Entity;

  fn load(
    (path, key): (Entity, AssetKey),
    (
      mut commands,
    ) : SystemParamItem<'_, '_, Self::SystemInput>
  ) -> Resultat<()> {
    let sprite_atlas = SpriteAtlas {
      image_asset: path,
      size_x: 10,
      size_y: 10,
    };
    let e_id = commands.spawn(Asset::<SpriteAtlas> { data: sprite_atlas } ).id();
    commands.add(InsertAssetCommand::<SpriteAtlas>::new(e_id, key));
    Ok(())
  }
}

impl Assetable for Image {
  type Storage = Arc<PersistentDescriptorSet>;
  type SystemInput = (
    SCommands,
    SRes<ASingleton<Device>>,
    SRes<ASingleton<Queue>>,
    SRes<ANamedSingleton<"Sampler", DescriptorSetLayout>>,
    SRes<ASingleton<StandardDescriptorSetAllocator>>,
    SRes<ASingleton<StandardCommandBufferAllocator>>,
    SRes<ASingleton<StandardMemoryAllocator>>
  );
  type SourceType = String;

  fn load(
    name: (Self::SourceType, AssetKey),
    (
      mut commands,
      device,
      queue,
      descriptor_set_layout,
      descriptor_set_allocator,
      command_buffer_allocator,
      memory_allocator
    ): SystemParamItem<'_, '_, Self::SystemInput>
  ) -> Resultat<()> {
    let (name, key) = name;
    let mut uploads = AutoCommandBufferBuilder::primary(
      command_buffer_allocator.clon().deref(),
      queue.queue_family_index(),
      CommandBufferUsage::OneTimeSubmit,
    )?;

    let texture = {
      let file = std::fs::read(name)?;
      let png_bytes = file.as_slice();
      // let png_bytes = include_bytes!("../../assets/tiles.png").to_vec();
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
        memory_allocator.clon(),
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
      device.clon(),
      SamplerCreateInfo {
        mag_filter: Filter::Nearest,
        min_filter: Filter::Nearest,
        address_mode: [SamplerAddressMode::Repeat; 3],
        ..Default::default()
      },
    )?;

    let set = PersistentDescriptorSet::new(
      descriptor_set_allocator.clon().deref(),
      descriptor_set_layout.clon(),
      [WriteDescriptorSet::image_view_sampler(0, texture, sampler)],
      [],
    )?;

    let _future = uploads.build()?.execute(queue.clon())?.flush();

    let e_id = commands.spawn(Asset::<Image> { data: set } ).id();
    commands.add(InsertAssetCommand::<Image>::new(e_id, key));
    Ok(())
  }
}

pub struct AssetsPlugin;

impl Plugin for AssetsPlugin {
  fn build(&self, app: &mut App) {
    app.init_resource::<Assets<Image>>();
    app.init_resource::<Assets<SpriteAtlas>>();
  }
}