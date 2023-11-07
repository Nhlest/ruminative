use crate::engine::{RuminativeInternals, RuminativePipeline};
use imgui::{BackendFlags, ConfigFlags, Context, DrawCmd, DrawIdx, DrawVert, FontAtlasTexture, FontSource, Io, Key};
use smallvec::smallvec;
use std::cmp::Ordering;
use std::error::Error;
use std::ffi::CString;
use std::slice;
use std::sync::Arc;
use vulkano::buffer::{Buffer, BufferContents, BufferCreateInfo, BufferUsage, Subbuffer};
use vulkano::command_buffer::allocator::StandardCommandBufferAllocator;
use vulkano::command_buffer::{
  AutoCommandBufferBuilder, CommandBufferUsage, CopyBufferToImageInfo, PrimaryAutoCommandBuffer,
  PrimaryCommandBufferAbstract,
};
use vulkano::descriptor_set::allocator::{StandardDescriptorSetAllocator, StandardDescriptorSetAllocatorCreateInfo};
use vulkano::descriptor_set::{PersistentDescriptorSet, WriteDescriptorSet};
use vulkano::device::{Device, Queue};
use vulkano::format::Format;
use vulkano::image::sampler::{Filter, Sampler, SamplerAddressMode, SamplerCreateInfo};
use vulkano::image::view::ImageView;
use vulkano::image::{Image, ImageCreateInfo, ImageType, ImageUsage};
use vulkano::memory::allocator::{AllocationCreateInfo, MemoryTypeFilter, StandardMemoryAllocator};
use vulkano::pipeline::graphics::color_blend::{AttachmentBlend, ColorBlendAttachmentState, ColorBlendState};
use vulkano::pipeline::graphics::input_assembly::InputAssemblyState;
use vulkano::pipeline::graphics::multisample::MultisampleState;
use vulkano::pipeline::graphics::rasterization::RasterizationState;
use vulkano::pipeline::graphics::subpass::PipelineRenderingCreateInfo;
use vulkano::pipeline::graphics::vertex_input::{Vertex, VertexDefinition};
use vulkano::pipeline::graphics::viewport::{Scissor, ViewportState};
use vulkano::pipeline::graphics::GraphicsPipelineCreateInfo;
use vulkano::pipeline::layout::PipelineDescriptorSetLayoutCreateInfo;
use vulkano::pipeline::{
  DynamicState, GraphicsPipeline, Pipeline, PipelineBindPoint, PipelineLayout, PipelineShaderStageCreateInfo,
};
use vulkano::shader::EntryPoint;
use vulkano::swapchain::Swapchain;
use vulkano::sync::GpuFuture;
use vulkano::DeviceSize;
use winit::event::{
  DeviceEvent, ElementState, Event, KeyboardInput, MouseButton, MouseScrollDelta, TouchPhase, VirtualKeyCode,
  WindowEvent,
};
use winit::window::Window;
// use crate::engine::tilemap_pipeline::MVertex;

pub struct ImguiPipeline {
  pub pipeline: Arc<GraphicsPipeline>,
  pub descriptor_set: Arc<PersistentDescriptorSet>,
  pub vertex_buffers: Vec<Subbuffer<[DrawVertPod]>>,
  pub index_buffers: Vec<Subbuffer<[DrawIdx]>>, // u16
  pub draw_commands: Vec<(usize, u32, u32, i32, [f32; 4])>,
  pub imgui: Context,
}

fn to_imgui_mouse_button(button: MouseButton) -> Option<imgui::MouseButton> {
  match button {
    MouseButton::Left | MouseButton::Other(0) => Some(imgui::MouseButton::Left),
    MouseButton::Right | MouseButton::Other(1) => Some(imgui::MouseButton::Right),
    MouseButton::Middle | MouseButton::Other(2) => Some(imgui::MouseButton::Middle),
    MouseButton::Other(3) => Some(imgui::MouseButton::Extra1),
    MouseButton::Other(4) => Some(imgui::MouseButton::Extra2),
    _ => None,
  }
}

fn to_imgui_key(keycode: VirtualKeyCode) -> Option<Key> {
  match keycode {
    VirtualKeyCode::Tab => Some(Key::Tab),
    VirtualKeyCode::Left => Some(Key::LeftArrow),
    VirtualKeyCode::Right => Some(Key::RightArrow),
    VirtualKeyCode::Up => Some(Key::UpArrow),
    VirtualKeyCode::Down => Some(Key::DownArrow),
    VirtualKeyCode::PageUp => Some(Key::PageUp),
    VirtualKeyCode::PageDown => Some(Key::PageDown),
    VirtualKeyCode::Home => Some(Key::Home),
    VirtualKeyCode::End => Some(Key::End),
    VirtualKeyCode::Insert => Some(Key::Insert),
    VirtualKeyCode::Delete => Some(Key::Delete),
    VirtualKeyCode::Back => Some(Key::Backspace),
    VirtualKeyCode::Space => Some(Key::Space),
    VirtualKeyCode::Return => Some(Key::Enter),
    VirtualKeyCode::Escape => Some(Key::Escape),
    VirtualKeyCode::LControl => Some(Key::LeftCtrl),
    VirtualKeyCode::LShift => Some(Key::LeftShift),
    VirtualKeyCode::LAlt => Some(Key::LeftAlt),
    VirtualKeyCode::LWin => Some(Key::LeftSuper),
    VirtualKeyCode::RControl => Some(Key::RightCtrl),
    VirtualKeyCode::RShift => Some(Key::RightShift),
    VirtualKeyCode::RAlt => Some(Key::RightAlt),
    VirtualKeyCode::RWin => Some(Key::RightSuper),
    //VirtualKeyCode::Menu => Some(Key::Menu), // TODO: find out if there is a Menu key in winit
    VirtualKeyCode::Key0 => Some(Key::Alpha0),
    VirtualKeyCode::Key1 => Some(Key::Alpha1),
    VirtualKeyCode::Key2 => Some(Key::Alpha2),
    VirtualKeyCode::Key3 => Some(Key::Alpha3),
    VirtualKeyCode::Key4 => Some(Key::Alpha4),
    VirtualKeyCode::Key5 => Some(Key::Alpha5),
    VirtualKeyCode::Key6 => Some(Key::Alpha6),
    VirtualKeyCode::Key7 => Some(Key::Alpha7),
    VirtualKeyCode::Key8 => Some(Key::Alpha8),
    VirtualKeyCode::Key9 => Some(Key::Alpha9),
    VirtualKeyCode::A => Some(Key::A),
    VirtualKeyCode::B => Some(Key::B),
    VirtualKeyCode::C => Some(Key::C),
    VirtualKeyCode::D => Some(Key::D),
    VirtualKeyCode::E => Some(Key::E),
    VirtualKeyCode::F => Some(Key::F),
    VirtualKeyCode::G => Some(Key::G),
    VirtualKeyCode::H => Some(Key::H),
    VirtualKeyCode::I => Some(Key::I),
    VirtualKeyCode::J => Some(Key::J),
    VirtualKeyCode::K => Some(Key::K),
    VirtualKeyCode::L => Some(Key::L),
    VirtualKeyCode::M => Some(Key::M),
    VirtualKeyCode::N => Some(Key::N),
    VirtualKeyCode::O => Some(Key::O),
    VirtualKeyCode::P => Some(Key::P),
    VirtualKeyCode::Q => Some(Key::Q),
    VirtualKeyCode::R => Some(Key::R),
    VirtualKeyCode::S => Some(Key::S),
    VirtualKeyCode::T => Some(Key::T),
    VirtualKeyCode::U => Some(Key::U),
    VirtualKeyCode::V => Some(Key::V),
    VirtualKeyCode::W => Some(Key::W),
    VirtualKeyCode::X => Some(Key::X),
    VirtualKeyCode::Y => Some(Key::Y),
    VirtualKeyCode::Z => Some(Key::Z),
    VirtualKeyCode::F1 => Some(Key::F1),
    VirtualKeyCode::F2 => Some(Key::F2),
    VirtualKeyCode::F3 => Some(Key::F3),
    VirtualKeyCode::F4 => Some(Key::F4),
    VirtualKeyCode::F5 => Some(Key::F5),
    VirtualKeyCode::F6 => Some(Key::F6),
    VirtualKeyCode::F7 => Some(Key::F7),
    VirtualKeyCode::F8 => Some(Key::F8),
    VirtualKeyCode::F9 => Some(Key::F9),
    VirtualKeyCode::F10 => Some(Key::F10),
    VirtualKeyCode::F11 => Some(Key::F11),
    VirtualKeyCode::F12 => Some(Key::F12),
    VirtualKeyCode::Apostrophe => Some(Key::Apostrophe),
    VirtualKeyCode::Comma => Some(Key::Comma),
    VirtualKeyCode::Minus => Some(Key::Minus),
    VirtualKeyCode::Period => Some(Key::Period),
    VirtualKeyCode::Slash => Some(Key::Slash),
    VirtualKeyCode::Semicolon => Some(Key::Semicolon),
    VirtualKeyCode::Equals => Some(Key::Equal),
    VirtualKeyCode::LBracket => Some(Key::LeftBracket),
    VirtualKeyCode::Backslash => Some(Key::Backslash),
    VirtualKeyCode::RBracket => Some(Key::RightBracket),
    VirtualKeyCode::Grave => Some(Key::GraveAccent),
    VirtualKeyCode::Capital => Some(Key::CapsLock),
    VirtualKeyCode::Scroll => Some(Key::ScrollLock),
    VirtualKeyCode::Numlock => Some(Key::NumLock),
    VirtualKeyCode::Snapshot => Some(Key::PrintScreen),
    VirtualKeyCode::Pause => Some(Key::Pause),
    VirtualKeyCode::Numpad0 => Some(Key::Keypad0),
    VirtualKeyCode::Numpad1 => Some(Key::Keypad1),
    VirtualKeyCode::Numpad2 => Some(Key::Keypad2),
    VirtualKeyCode::Numpad3 => Some(Key::Keypad3),
    VirtualKeyCode::Numpad4 => Some(Key::Keypad4),
    VirtualKeyCode::Numpad5 => Some(Key::Keypad5),
    VirtualKeyCode::Numpad6 => Some(Key::Keypad6),
    VirtualKeyCode::Numpad7 => Some(Key::Keypad7),
    VirtualKeyCode::Numpad8 => Some(Key::Keypad8),
    VirtualKeyCode::Numpad9 => Some(Key::Keypad9),
    VirtualKeyCode::NumpadDecimal => Some(Key::KeypadDecimal),
    VirtualKeyCode::NumpadDivide => Some(Key::KeypadDivide),
    VirtualKeyCode::NumpadMultiply => Some(Key::KeypadMultiply),
    VirtualKeyCode::NumpadSubtract => Some(Key::KeypadSubtract),
    VirtualKeyCode::NumpadAdd => Some(Key::KeypadAdd),
    VirtualKeyCode::NumpadEnter => Some(Key::KeypadEnter),
    VirtualKeyCode::NumpadEquals => Some(Key::KeypadEqual),
    _ => None,
  }
}

fn handle_key_modifier(io: &mut Io, key: VirtualKeyCode, down: bool) {
  if key == VirtualKeyCode::LShift || key == VirtualKeyCode::RShift {
    io.add_key_event(imgui::Key::ModShift, down);
  } else if key == VirtualKeyCode::LControl || key == VirtualKeyCode::RControl {
    io.add_key_event(imgui::Key::ModCtrl, down);
  } else if key == VirtualKeyCode::LAlt || key == VirtualKeyCode::RAlt {
    io.add_key_event(imgui::Key::ModAlt, down);
  } else if key == VirtualKeyCode::LWin || key == VirtualKeyCode::RWin {
    io.add_key_event(imgui::Key::ModSuper, down);
  }
}

fn handle_window_event(io: &mut Io, window: &Window, event: &WindowEvent) {
  match *event {
    WindowEvent::Resized(physical_size) => {
      let logical_size = physical_size.to_logical(window.scale_factor());
      io.display_size = [logical_size.width, logical_size.height];
    }
    WindowEvent::ScaleFactorChanged { scale_factor, .. } => {
      let hidpi_factor = scale_factor;
      if io.mouse_pos[0].is_finite() && io.mouse_pos[1].is_finite() {
        io.mouse_pos = [
          io.mouse_pos[0] * window.scale_factor() as f32,
          io.mouse_pos[1] * window.scale_factor() as f32,
        ];
      }
      io.display_framebuffer_scale = [hidpi_factor as f32, hidpi_factor as f32];
      let logical_size = window.inner_size().to_logical(scale_factor);
      io.display_size = [logical_size.width, logical_size.height];
    }
    WindowEvent::ModifiersChanged(modifiers) => {
      io.add_key_event(Key::ModShift, modifiers.shift());
      io.add_key_event(Key::ModCtrl, modifiers.ctrl());
      io.add_key_event(Key::ModAlt, modifiers.alt());
      io.add_key_event(Key::ModSuper, modifiers.logo());
    }
    WindowEvent::KeyboardInput {
      input: KeyboardInput {
        virtual_keycode: Some(key),
        state,
        ..
      },
      ..
    } => {
      let pressed = state == ElementState::Pressed;

      handle_key_modifier(io, key, pressed);

      // Add main key event
      if let Some(key) = to_imgui_key(key) {
        io.add_key_event(key, pressed);
      }
    }
    WindowEvent::ReceivedCharacter(ch) => {
      if ch != '\u{7f}' {
        io.add_input_character(ch)
      }
    }
    WindowEvent::CursorMoved { position, .. } => {
      let position = position.to_logical(window.scale_factor());
      io.add_mouse_pos_event([position.x, position.y]);
    }
    WindowEvent::MouseWheel {
      delta,
      phase: TouchPhase::Moved,
      ..
    } => {
      let (h, v) = match delta {
        MouseScrollDelta::LineDelta(h, v) => (h, v),
        MouseScrollDelta::PixelDelta(pos) => {
          let pos = pos.to_logical::<f64>(window.scale_factor());
          let h = match pos.x.partial_cmp(&0.0) {
            Some(Ordering::Greater) => 1.0,
            Some(Ordering::Less) => -1.0,
            _ => 0.0,
          };
          let v = match pos.y.partial_cmp(&0.0) {
            Some(Ordering::Greater) => 1.0,
            Some(Ordering::Less) => -1.0,
            _ => 0.0,
          };
          (h, v)
        }
      };
      io.add_mouse_wheel_event([h, v]);
    }
    WindowEvent::MouseInput { state, button, .. } => {
      if let Some(mb) = to_imgui_mouse_button(button) {
        let pressed = state == ElementState::Pressed;
        io.add_mouse_button_event(mb, pressed);
      }
    }
    WindowEvent::Focused(newly_focused) => {
      if !newly_focused {
        // Set focus-lost to avoid stuck keys (like 'alt'
        // when alt-tabbing)
        io.app_focus_lost = true;
      }
    }
    _ => (),
  }
}

impl RuminativePipeline for ImguiPipeline {
  fn update(&mut self, ruminative_internals: &RuminativeInternals) {
    self.index_buffers.clear();
    self.vertex_buffers.clear();
    self.draw_commands.clear();
    {
      let ui = self.imgui.new_frame();
      ui.dockspace_over_main_viewport();
      ui.show_demo_window(&mut true);
      unsafe {
        let dock_id = imgui_sys::igGetID_Str(CString::new("Dockspace").unwrap().as_ptr());
        let _dock = imgui_sys::igDockBuilderGetNode(dock_id);
      }
      // imgui::Pan
      // ui.window("Bottom panel").dock
    }
    let d = self.imgui.render();
    let mut i = 0;
    for dl in d.draw_lists() {
      assert_eq!(core::mem::size_of::<DrawVertPod>(), core::mem::size_of::<DrawVert>(),);
      assert!(core::mem::align_of::<DrawVertPod>() <= core::mem::align_of::<DrawVert>());
      let vertices: &[DrawVertPod] =
        unsafe { slice::from_raw_parts(dl.vtx_buffer().as_ptr().cast(), dl.vtx_buffer().len()) };
      let vertex_buffer = Buffer::from_iter(
        ruminative_internals.memory_allocator.clone(),
        BufferCreateInfo {
          usage: BufferUsage::VERTEX_BUFFER,
          ..Default::default()
        },
        AllocationCreateInfo {
          memory_type_filter: MemoryTypeFilter::PREFER_HOST,
          ..Default::default()
        },
        vertices.iter().cloned(),
      )
      .unwrap();

      self.vertex_buffers.push(vertex_buffer);
      let indicies = dl.idx_buffer();
      let index_buffer = Buffer::from_iter(
        ruminative_internals.memory_allocator.clone(),
        BufferCreateInfo {
          usage: BufferUsage::INDEX_BUFFER,
          ..Default::default()
        },
        AllocationCreateInfo {
          memory_type_filter: MemoryTypeFilter::PREFER_HOST,
          ..Default::default()
        },
        indicies.iter().copied(),
      )
      .unwrap();
      self.index_buffers.push(index_buffer);

      for cmd in dl.commands() {
        match cmd {
          DrawCmd::Elements { count, cmd_params } => {
            self.draw_commands.push((
              i,
              count as u32,
              cmd_params.idx_offset as u32,
              cmd_params.vtx_offset as i32,
              cmd_params.clip_rect,
            ));
          }
          DrawCmd::ResetRenderState => {}
          DrawCmd::RawCallback { .. } => {}
        }
      }
      i += 1;
    }
  }
  fn bind<'a>(
    &self,
    builder: &'a mut AutoCommandBufferBuilder<PrimaryAutoCommandBuffer>,
    window: &Window,
  ) -> Result<&'a mut AutoCommandBufferBuilder<PrimaryAutoCommandBuffer>, Box<dyn Error>> {
    builder
      .bind_pipeline_graphics(self.pipeline.clone())?
      .bind_descriptor_sets(
        PipelineBindPoint::Graphics,
        self.pipeline.layout().clone(),
        0,
        self.descriptor_set.clone(),
      )?;
    for (buf, index_count, first_index, vertex_offset, clip_rect) in &self.draw_commands {
      builder
        .bind_vertex_buffers(0, self.vertex_buffers[*buf].clone())?
        .bind_index_buffer(self.index_buffers[*buf].clone())?
        .set_scissor(
          0,
          smallvec![Scissor {
            offset: [
              (f32::max(0.0, clip_rect[0]) * window.scale_factor() as f32) as u32,
              (f32::max(0.0, clip_rect[1]) * window.scale_factor() as f32) as u32,
            ],
            extent: [
              ((clip_rect[2] - clip_rect[0]).abs().ceil() * window.scale_factor() as f32) as u32,
              ((clip_rect[3] - clip_rect[1]).abs().ceil() * window.scale_factor() as f32) as u32,
            ],
          }],
        )?
        .push_constants(
          self.pipeline.layout().clone(),
          0,
          vs::PushConstants {
            window_height: window.inner_size().to_logical(window.scale_factor()).height,
            window_width: window.inner_size().to_logical(window.scale_factor()).width,
          },
        )?
        .draw_indexed(*index_count, 1, *first_index, *vertex_offset, 0)?;
    }
    Ok(builder)
  }
  fn handle_event(&mut self, window: &Window, event: &Event<()>) {
    match *event {
      Event::WindowEvent { window_id, ref event } if window_id == window.id() => {
        handle_window_event(self.imgui.io_mut(), window, event);
      }
      Event::DeviceEvent {
        event:
          DeviceEvent::Key(KeyboardInput {
            state: ElementState::Released,
            virtual_keycode: Some(key),
            ..
          }),
        ..
      } => {
        if let Some(key) = to_imgui_key(key) {
          self.imgui.io_mut().add_key_event(key, false);
        }
      }
      _ => (),
    }
  }
}

#[derive(BufferContents, Vertex, Clone)]
#[repr(C)]
pub struct DrawVertPod {
  #[format(R32G32_SFLOAT)]
  pos: [f32; 2],
  #[format(R32G32_SFLOAT)]
  uv: [f32; 2],
  #[format(R8G8B8A8_UNORM)]
  col: [u8; 4],
}

mod vs {
  vulkano_shaders::shader! {
      ty: "vertex",
      path: "assets/shaders/imgui_vertex.glsl"
  }
}

mod fs {
  vulkano_shaders::shader! {
      ty: "fragment",
      path: "assets/shaders/imgui_fragment.glsl"
  }
}

impl ImguiPipeline {
  fn shaders(device: Arc<Device>) -> Result<(EntryPoint, EntryPoint), Box<dyn Error>> {
    let vs = vs::load(device.clone())?.entry_point("main").unwrap();
    let fs = fs::load(device)?.entry_point("main").unwrap();
    Ok((vs, fs))
  }

  fn sampler(
    device: Arc<Device>,
    queue: Arc<Queue>,
    pipeline: Arc<GraphicsPipeline>,
    memory_allocator: Arc<StandardMemoryAllocator>,
    tex: FontAtlasTexture,
  ) -> Result<(Arc<PersistentDescriptorSet>, Option<Box<dyn GpuFuture>>), Box<dyn Error>> {
    let descriptor_set_allocator =
      StandardDescriptorSetAllocator::new(device.clone(), StandardDescriptorSetAllocatorCreateInfo::default());
    let command_buffer_allocator = StandardCommandBufferAllocator::new(device.clone(), Default::default());
    let mut uploads = AutoCommandBufferBuilder::primary(
      &command_buffer_allocator,
      queue.queue_family_index(),
      CommandBufferUsage::OneTimeSubmit,
    )?;

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
      (tex.width * tex.height * 4) as DeviceSize,
    )?;

    upload_buffer.write()?.copy_from_slice(tex.data);

    let texture = {
      let image = Image::new(
        memory_allocator,
        ImageCreateInfo {
          usage: ImageUsage::TRANSFER_DST | ImageUsage::SAMPLED,
          format: Format::R8G8B8A8_SRGB,
          image_type: ImageType::Dim2d,
          extent: [tex.width, tex.height, 1],
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
      [WriteDescriptorSet::image_view_sampler(0, texture, sampler)],
      [],
    )?;

    let previous_frame_end = Some(uploads.build()?.execute(queue)?.boxed());
    previous_frame_end.unwrap().flush()?;

    Ok((set, None))
  }

  fn pipeline(
    device: Arc<Device>,
    swapchain: Arc<Swapchain>,
    vs: EntryPoint,
    fs: EntryPoint,
  ) -> Result<Arc<GraphicsPipeline>, Box<dyn Error>> {
    let vertex_input_state = DrawVertPod::per_vertex().definition(&vs.info().input_interface)?;
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
        input_assembly_state: Some(InputAssemblyState::default()),
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
        dynamic_state: [DynamicState::Scissor, DynamicState::Viewport].into_iter().collect(),
        subpass: Some(subpass.into()),
        ..GraphicsPipelineCreateInfo::layout(layout)
      },
    )?;
    Ok(pipeline)
  }

  pub fn new(
    ruminative_internals: &RuminativeInternals,
    future: Option<Box<dyn GpuFuture>>,
  ) -> Result<(Self, Box<dyn GpuFuture>), Box<dyn Error>> {
    let memory_allocator = ruminative_internals.memory_allocator.clone();
    let (vs, fs) = Self::shaders(ruminative_internals.device.clone())?;
    let pipeline = Self::pipeline(
      ruminative_internals.device.clone(),
      ruminative_internals.swapchain.clone(),
      vs,
      fs,
    )?;

    let mut imgui = Context::create();
    imgui.fonts().add_font(&[FontSource::DefaultFontData { config: None }]);
    let tex = imgui.fonts().build_rgba32_texture();

    let (descriptor_set, previous_frame_end) = Self::sampler(
      ruminative_internals.device.clone(),
      ruminative_internals.queue.clone(),
      pipeline.clone(),
      memory_allocator,
      tex,
    )?;
    // let vertex_buffer = Self::vertex_buffer(&memory_allocator)?;
    // let index_buffer = Self::index_buffer(&memory_allocator)?;
    let previous_frame_end = if let Some(future) = future {
      // future.join(previous_frame_end.unwrap()).boxed()
      future
    } else {
      previous_frame_end.unwrap()
    };

    let io = imgui.io_mut();
    io.display_size = [800.0, 600.0];
    io.display_framebuffer_scale = [2.0, 2.0];
    io.backend_flags.insert(BackendFlags::HAS_MOUSE_CURSORS);
    io.backend_flags.insert(BackendFlags::HAS_SET_MOUSE_POS);
    io.backend_flags.insert(BackendFlags::RENDERER_HAS_VTX_OFFSET);
    io.config_flags.insert(ConfigFlags::DOCKING_ENABLE);
    imgui.set_platform_name(Some(format!("imgui-winit-support {}", env!("CARGO_PKG_VERSION"))));
    imgui.set_renderer_name(Some(format!("imgui-glium-renderer {}", env!("CARGO_PKG_VERSION"))));

    Ok((
      Self {
        pipeline,
        descriptor_set,
        vertex_buffers: vec![],
        index_buffers: vec![],
        draw_commands: vec![],
        imgui,
      },
      previous_frame_end,
    ))
  }
}
