use std::{iter, sync::Arc, time::Instant};
use winit::application::ApplicationHandler;
use winit::event::{WindowEvent, KeyEvent};
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::keyboard::{KeyCode, PhysicalKey};
use winit::window::{Window, WindowAttributes, WindowId};

// Наш интерфейс для игры
pub trait Game {
    fn on_create(&mut self, _window: &Arc<Window>) {}
    fn on_update(&mut self, _dt: f32) {}
    fn on_render(&mut self) {}
    fn on_mouse_move(&mut self, _x: f64, _y: f64) {}
    fn on_key_press(&mut self, _code: KeyCode, _is_pressed: bool) {}
}

// Графическое состояние движка
struct EngineState {
    surface: wgpu::Surface<'static>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
    window: Arc<Window>,
    pipeline: Option<wgpu::RenderPipeline>,
}

impl EngineState {
    async fn new(window: Arc<Window>) -> anyhow::Result<Self> {
        let size = window.inner_size();

        // Явно прописываем все поля, так как Default больше не реализован
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            #[cfg(not(target_arch = "wasm32"))]
            backends: wgpu::Backends::PRIMARY,
            #[cfg(target_arch = "wasm32")]
            backends: wgpu::Backends::GL,
            flags: Default::default(),
            memory_budget_thresholds: Default::default(),
            backend_options: Default::default(),
            display: None,
        });

        // Используем ? вместо unwrap()
        let surface = instance.create_surface(window.clone())?;

        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::default(),
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
                apply_limit_buckets: true,
            })
            .await?;

        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor {
                label: None,
                required_features: wgpu::Features::empty(),
                experimental_features: wgpu::ExperimentalFeatures::disabled(),
                required_limits: wgpu::Limits::default(),
                memory_hints: Default::default(),
                trace: wgpu::Trace::Off,
            })
            .await?;

        let surface_caps = surface.get_capabilities(&adapter);
        let surface_format = surface_caps
            .formats
            .iter()
            .copied()
            .find(|f| f.is_srgb())
            .unwrap_or(surface_caps.formats[0]);

        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: surface_format,
            width: size.width,
            height: size.height,
            present_mode: surface_caps.present_modes[0],
            alpha_mode: surface_caps.alpha_modes[0],
            desired_maximum_frame_latency: 2,
            view_formats: vec![],
            color_space: wgpu::SurfaceColorSpace::Auto,
        };

        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Triangle Shader"),
            source: wgpu::ShaderSource::Wgsl(r#"
                @vertex
                fn vs_main(@builtin(vertex_index) in_vertex_index: u32) -> @builtin(position) vec4<f32> {
                    let x = f32(1 - i32(in_vertex_index)) * 0.5;
                    let y = f32(i32(in_vertex_index & 1u) * 2 - 1) * 0.5;
                    return vec4<f32>(x, y, 0.0, 1.0);
                }
                @fragment
                fn fs_main() -> @location(0) vec4<f32> {
                    return vec4<f32>(0.8, 0.2, 0.3, 1.0); // Оранжевый цвет
                }
            "#.into()),
        });

        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Render Pipeline"),
            layout: None,
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                buffers: &[],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: config.format,
                    blend: Some(wgpu::BlendState::REPLACE),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
            }),
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview_mask: None, // Возвращаем как ты и исправил
            cache: None,
        });

        Ok(Self {
            surface,
            device,
            queue,
            config,
            window,
            pipeline: Some(pipeline),
        })
    }

    pub fn resize(&mut self, width: u32, height: u32) {
        if width > 0 && height > 0 {
            self.config.width = width;
            self.config.height = height;
            self.surface.configure(&self.device, &self.config);
        }
    }

    fn render(&mut self) -> anyhow::Result<()> {
        let output = match self.surface.get_current_texture() {
            wgpu::CurrentSurfaceTexture::Success(surface_texture) => surface_texture,
            wgpu::CurrentSurfaceTexture::Suboptimal(surface_texture) => surface_texture,
            wgpu::CurrentSurfaceTexture::Timeout
            | wgpu::CurrentSurfaceTexture::Occluded
            | wgpu::CurrentSurfaceTexture::Validation => return Ok(()),
            wgpu::CurrentSurfaceTexture::Outdated => {
                self.surface.configure(&self.device, &self.config);
                return Ok(());
            }
            wgpu::CurrentSurfaceTexture::Lost => {
                anyhow::bail!("Lost device");
            }
        };

        let view = output.texture.create_view(&wgpu::TextureViewDescriptor::default());

        let mut encoder = self.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Render Encoder"),
        });

        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Render Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color { r: 0.02, g: 0.02, b: 0.05, a: 1.0 }),
                        store: wgpu::StoreOp::Store,
                    },
                    depth_slice: None,
                })],
                depth_stencil_attachment: None,
                occlusion_query_set: None,
                timestamp_writes: None,
                multiview_mask: None,
            });

            if let Some(pipeline) = &self.pipeline {
                render_pass.set_pipeline(pipeline);
                render_pass.draw(0..3, 0..1);
            }
        }

        self.queue.submit(iter::once(encoder.finish()));
        self.queue.present(output);
        Ok(())
    }
}

struct EngineApp<G: Game> {
    game: G,
    state: Option<EngineState>,
    last_time: Instant,
}

impl<G: Game + 'static> ApplicationHandler for EngineApp<G> {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.state.is_none() {
            let window_attrs = WindowAttributes::default()
                .with_title("Wingman Protocol EX")
                .with_inner_size(winit::dpi::LogicalSize::new(800, 600));

            let window = Arc::new(event_loop.create_window(window_attrs).expect("Failed to create window"));

            self.state = Some(pollster::block_on(EngineState::new(window.clone())).expect("Failed to init graphics state"));
            self.game.on_create(&window);
        }
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        let state = match &mut self.state {
            Some(s) => s,
            None => return,
        };

        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::Resized(size) => state.resize(size.width, size.height),
            WindowEvent::RedrawRequested => {
                let now = Instant::now();
                let dt = now.duration_since(self.last_time).as_secs_f32();
                self.last_time = now;

                self.game.on_update(dt);
                self.game.on_render();

                match state.render() {
                    Ok(_) => {}
                    Err(e) => {
                        eprintln!("Render error: {}", e);
                        event_loop.exit();
                    }
                }
            }
            WindowEvent::CursorMoved { position, .. } => {
                self.game.on_mouse_move(position.x, position.y);
            }
            WindowEvent::KeyboardInput {
                event: KeyEvent {
                    physical_key: PhysicalKey::Code(code),
                    state: key_state,
                    ..
                },
                ..
            } => {
                self.game.on_key_press(code, key_state.is_pressed());

                if code == KeyCode::Escape && key_state.is_pressed() {
                    event_loop.exit();
                }
            }
            _ => {}
        }
    }

    fn about_to_wait(&mut self, _event_loop: &ActiveEventLoop) {
        if let Some(state) = &self.state {
            state.window.request_redraw();
        }
    }
}

// Убрали mut у game, чтобы линтер не ругался на неиспользуемую мутабельность
pub fn run<G: Game + 'static>(game: G) -> Result<(), Box<dyn std::error::Error>> {
    let event_loop = EventLoop::new()?;
    event_loop.set_control_flow(ControlFlow::Poll);

    let mut app = EngineApp {
        game,
        state: None,
        last_time: Instant::now(),
    };

    event_loop.run_app(&mut app)?;
    Ok(())
}