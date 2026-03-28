use std::{sync::Arc, time::Instant};

use glam::Vec2;
use pollster::FutureExt;
use wgpu::util::DeviceExt;

fn main() {
	let event_loop = winit::event_loop::EventLoop::new().unwrap();
	let mut app = App::Init;
	event_loop.run_app(&mut app).unwrap();
}

#[expect(clippy::large_enum_variant)]
enum App {
	Init,
	Running(State),
}

struct State {
	window: Arc<winit::window::Window>,
	surface: wgpu::Surface<'static>,

	#[expect(unused)]
	instance: wgpu::Instance,
	adapter: wgpu::Adapter,
	device: wgpu::Device,
	queue: wgpu::Queue,

	pipeline: wgpu::RenderPipeline,
	uniform: shaders::ShaderUniform,
	uniform_buffer: wgpu::Buffer,
	uniform_bind_group: wgpu::BindGroup,
	texture_bind_group: wgpu::BindGroup,
	vertex_buffer: wgpu::Buffer,
	instance_buffer: wgpu::Buffer,

	start: Instant,
}

impl winit::application::ApplicationHandler for App {
	fn resumed(&mut self, event_loop: &winit::event_loop::ActiveEventLoop) {
		let window = event_loop
			.create_window(winit::window::Window::default_attributes())
			.unwrap();
		let window = Arc::new(window);

		let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
			backends: wgpu::Backends::PRIMARY,
			flags: wgpu::InstanceFlags::empty(),
			memory_budget_thresholds: wgpu::MemoryBudgetThresholds::default(),
			backend_options: wgpu::BackendOptions::default(),
		});

		let surface = instance.create_surface(window.clone()).unwrap();

		let adapter = instance
			.request_adapter(&wgpu::RequestAdapterOptions {
				power_preference: wgpu::PowerPreference::HighPerformance,
				force_fallback_adapter: false,
				compatible_surface: Some(&surface),
			})
			.block_on()
			.unwrap();

		let (device, queue) = adapter
			.request_device(&wgpu::DeviceDescriptor::default())
			.block_on()
			.unwrap();

		let size = window.inner_size();
		let configuration = surface
			.get_default_config(&adapter, size.width, size.height)
			.unwrap();
		surface.configure(&device, &configuration);

		let module = device.create_shader_module(wgpu::include_spirv!(env!("SHADER_SPV_PATH")));

		let bind_group_layout_0 =
			device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
				label: None,
				entries: shaders::MAIN_BIND_GROUPS[0],
			});

		let bind_group_layout_1 =
			device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
				label: None,
				entries: shaders::MAIN_BIND_GROUPS[1],
			});

		let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
			label: None,
			bind_group_layouts: &[&bind_group_layout_0, &bind_group_layout_1],
			push_constant_ranges: &[],
		});

		let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
			label: None,
			layout: Some(&pipeline_layout),
			vertex: wgpu::VertexState {
				module: &module,
				entry_point: Some(shaders::MAIN_VS_NAME),
				compilation_options: wgpu::PipelineCompilationOptions::default(),
				buffers: shaders::MAIN_VS_VERTEX_BUFFER_LAYOUTS,
			},
			primitive: wgpu::PrimitiveState {
				topology: wgpu::PrimitiveTopology::TriangleList,
				cull_mode: None,
				..wgpu::PrimitiveState::default()
			},
			depth_stencil: None,
			multisample: wgpu::MultisampleState::default(),
			fragment: Some(wgpu::FragmentState {
				module: &module,
				entry_point: Some(shaders::MAIN_FS_NAME),
				compilation_options: wgpu::PipelineCompilationOptions::default(),
				targets: &[Some(wgpu::ColorTargetState {
					format: configuration.format,
					blend: None,
					write_mask: wgpu::ColorWrites::all(),
				})],
			}),
			multiview: None,
			cache: None,
		});

		let uniform = shaders::ShaderUniform {
			time: 1.0,
			speed: 0.4,
			color_scale: 1.0,
		};

		let uniform_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
			label: None,
			contents: bytemuck::bytes_of(&uniform),
			usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
		});

		let uniform_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
			label: None,
			layout: &bind_group_layout_0,
			entries: &[wgpu::BindGroupEntry {
				binding: 0,
				resource: uniform_buffer.as_entire_binding(),
			}],
		});

		let sampler = device.create_sampler(&wgpu::wgt::SamplerDescriptor::default());
		let texture = device.create_texture_with_data(
			&queue,
			&wgpu::TextureDescriptor {
				label: None,
				dimension: wgpu::TextureDimension::D2,
				format: wgpu::TextureFormat::Rgba8Unorm,
				mip_level_count: 1,
				size: wgpu::Extent3d {
					width: 1024,
					height: 1024,
					depth_or_array_layers: 1,
				},
				sample_count: 1,
				usage: wgpu::TextureUsages::TEXTURE_BINDING,
				view_formats: &[],
			},
			wgpu::util::TextureDataOrder::LayerMajor,
			(0..1024)
				.flat_map(|x| (0..1024).map(move |y| (x, y)))
				.flat_map(|(x, y)| [(x % 256) as u8, (y % 256) as u8, 0, 255])
				.collect::<Vec<_>>()
				.as_slice(),
		);

		let texture_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
			label: None,
			layout: &bind_group_layout_1,
			entries: &[
				wgpu::BindGroupEntry {
					binding: 0,
					resource: wgpu::BindingResource::TextureView(
						&texture.create_view(&wgpu::TextureViewDescriptor::default()),
					),
				},
				wgpu::BindGroupEntry {
					binding: 1,
					resource: wgpu::BindingResource::Sampler(&sampler),
				},
			],
		});

		let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
			label: None,
			contents: bytemuck::cast_slice(&[
				shaders::Vertex {
					position: glam::Vec2 { x: -1.0, y: 0.0 },
					uv: Vec2::new(0.0, 0.0),
				},
				shaders::Vertex {
					position: glam::Vec2 { x: 1.0, y: 0.0 },
					uv: Vec2::new(1.0, 0.0),
				},
				shaders::Vertex {
					position: glam::Vec2 { x: 0.0, y: 1.0 },
					uv: Vec2::new(0.0, 1.0),
				},
			]),
			usage: wgpu::BufferUsages::VERTEX,
		});

		let instance_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
			label: None,
			contents: bytemuck::cast_slice(&[
				shaders::Instance {
					color: glam::Vec3 {
						x: 1.0,
						y: 1.0,
						z: 1.0,
					},
					offset: 0.0,
				},
				shaders::Instance {
					color: glam::Vec3 {
						x: 0.0,
						y: 1.0,
						z: 1.0,
					},
					offset: std::f32::consts::PI,
				},
			]),
			usage: wgpu::BufferUsages::VERTEX,
		});

		let state = State {
			window,
			surface,

			instance,
			adapter,
			device,
			queue,

			pipeline,
			uniform,
			uniform_buffer,
			uniform_bind_group,
			texture_bind_group,
			vertex_buffer,
			instance_buffer,

			start: Instant::now(),
		};

		*self = App::Running(state);
	}

	fn window_event(
		&mut self,
		event_loop: &winit::event_loop::ActiveEventLoop,
		_window_id: winit::window::WindowId,
		event: winit::event::WindowEvent,
	) {
		let state = match self {
			Self::Running(state) => state,
			_ => return,
		};
		match event {
			winit::event::WindowEvent::RedrawRequested => {
				state.uniform.time = state.start.elapsed().as_secs_f32();
				state.queue.write_buffer(
					&state.uniform_buffer,
					0,
					bytemuck::bytes_of(&state.uniform),
				);

				let mut encoder = state
					.device
					.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());

				let output = state.surface.get_current_texture().unwrap();
				let view = output
					.texture
					.create_view(&wgpu::TextureViewDescriptor::default());

				let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
					color_attachments: &[Some(wgpu::RenderPassColorAttachment {
						view: &view,
						depth_slice: None,
						resolve_target: None,
						ops: wgpu::Operations {
							load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
							store: wgpu::StoreOp::Store,
						},
					})],
					..wgpu::RenderPassDescriptor::default()
				});
				// maybe: keep untyped, but add constants for the indices
				// - render_pass.set_bind_group(shaders::MAIN_VS_BIND_GROUP_UNIFORM, &state.uniform_bind_group, &[]);
				render_pass.set_pipeline(&state.pipeline);
				render_pass.set_bind_group(0, &state.uniform_bind_group, &[]);
				render_pass.set_bind_group(1, &state.texture_bind_group, &[]);
				render_pass.set_vertex_buffer(0, state.vertex_buffer.slice(..));
				render_pass.set_vertex_buffer(1, state.instance_buffer.slice(..));
				render_pass.draw(0..3, 0..2);
				drop(render_pass);

				state.queue.submit([encoder.finish()]);
				output.present();

				state.window.request_redraw();
			}
			winit::event::WindowEvent::Resized(size) => {
				let configuration = state
					.surface
					.get_default_config(&state.adapter, size.width, size.height)
					.unwrap();
				state.surface.configure(&state.device, &configuration);
			}

			winit::event::WindowEvent::CloseRequested => {
				event_loop.exit();
			}
			_ => {}
		}
	}
}
