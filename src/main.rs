use std::sync::Arc;
use std::time::Instant;

use wgpu::util::DeviceExt;
use wgpu::{
    BindGroup, BindGroupDescriptor, BindGroupEntry, BindingType, Buffer, BufferBindingType,
    BufferDescriptor, BufferUsages, Color, CommandEncoderDescriptor, ComputePassDescriptor,
    CurrentSurfaceTexture, DeviceDescriptor, Instance, LoadOp, Operations, PowerPreference,
    PresentMode, RenderPassColorAttachment, RenderPassDescriptor, RequestAdapterOptions,
    ShaderStages, SurfaceConfiguration, TextureUsages, TextureViewDescriptor,
};
use winit::{
    application::ApplicationHandler,
    event::WindowEvent,
    event_loop::{ActiveEventLoop, ControlFlow, EventLoop},
    window::{Window, WindowId},
};

const PARTICLE_COUNT: u32 = 2000;
const WORKGROUP_SIZE: u32 = 64;

struct App {
    window: Option<Arc<Window>>,
    surface: Option<wgpu::Surface<'static>>,
    device: Option<wgpu::Device>,
    queue: Option<wgpu::Queue>,
    config: Option<SurfaceConfiguration>,

    particle_buffer: Option<Buffer>,
    simulation_buffer: Option<Buffer>,

    compute_pipeline: Option<wgpu::ComputePipeline>,
    compute_bind_group: Option<BindGroup>,

    render_pipeline: Option<wgpu::RenderPipeline>,
    render_bind_group: Option<BindGroup>,

    trail_textures: Option<[wgpu::Texture; 2]>,
    trail_views: Option<[wgpu::TextureView; 2]>,
    trail_bind_groups: Option<[wgpu::BindGroup; 2]>,
    trail_pipeline: Option<wgpu::RenderPipeline>,
    trail_sampler: Option<wgpu::Sampler>,
    trail_bind_group_layout: Option<wgpu::BindGroupLayout>,
    trail_format: Option<wgpu::TextureFormat>,
    trail_index: usize,

    present_bind_groups: Option<[wgpu::BindGroup; 2]>,
    present_pipeline: Option<wgpu::RenderPipeline>,
    last_frame: Instant,
}

impl Default for App {
    fn default() -> Self {
        Self {
            window: None,
            surface: None,
            device: None,
            queue: None,
            config: None,
            particle_buffer: None,
            simulation_buffer: None,
            compute_pipeline: None,
            compute_bind_group: None,
            render_pipeline: None,
            render_bind_group: None,
            trail_textures: None,
            trail_views: None,
            trail_bind_groups: None,
            trail_pipeline: None,
            trail_sampler: None,
            trail_bind_group_layout: None,
            trail_format: None,
            trail_index: 0,
            present_bind_groups: None,
            present_pipeline: None,
            last_frame: Instant::now(),
        }
    }
}

impl App {
    fn render(&mut self) {
        let (
            Some(surface),
            Some(device),
            Some(queue),
            Some(config),
            Some(particle_buffer),
            Some(simulation_buffer),
            Some(compute_pipeline),
            Some(compute_bind_group),
            Some(render_pipeline),
            Some(render_bind_group),
            Some(trail_views),
            Some(trail_bind_groups),
            Some(trail_pipeline),
            Some(present_bind_groups),
            Some(present_pipeline),
        ) = (
            &self.surface,
            &self.device,
            &self.queue,
            &self.config,
            &self.particle_buffer,
            &self.simulation_buffer,
            &self.compute_pipeline,
            &self.compute_bind_group,
            &self.render_pipeline,
            &self.render_bind_group,
            &self.trail_views,
            &self.trail_bind_groups,
            &self.trail_pipeline,
            &self.present_bind_groups,
            &self.present_pipeline,
        )
        else {
            return;
        };

        let trail_index = self.trail_index;

        let now = Instant::now();
        let dt = now.duration_since(self.last_frame).as_secs_f32().min(0.033);

        self.last_frame = now;

        // Update simulation parameters.
        let simulation_data: [u32; 4] = [dt.to_bits(), PARTICLE_COUNT, 0, 0];

        queue.write_buffer(simulation_buffer, 0, bytemuck::cast_slice(&simulation_data));

        // Acquire the window's next frame.
        let frame = match surface.get_current_texture() {
            CurrentSurfaceTexture::Success(frame) | CurrentSurfaceTexture::Suboptimal(frame) => {
                frame
            }

            CurrentSurfaceTexture::Timeout | CurrentSurfaceTexture::Occluded => return,

            CurrentSurfaceTexture::Outdated
            | CurrentSurfaceTexture::Lost
            | CurrentSurfaceTexture::Validation => {
                surface.configure(device, config);
                return;
            }
        };

        let view = frame.texture.create_view(&TextureViewDescriptor::default());

        let mut encoder = device.create_command_encoder(&CommandEncoderDescriptor {
            label: Some("Particle Simulation Encoder"),
        });

        // Pass 1: Update every particle on the GPU.
        {
            let mut pass = encoder.begin_compute_pass(&ComputePassDescriptor {
                label: Some("Particle Compute Pass"),
                timestamp_writes: None,
            });

            pass.set_pipeline(compute_pipeline);
            pass.set_bind_group(0, compute_bind_group, &[]);

            let workgroups = PARTICLE_COUNT.div_ceil(WORKGROUP_SIZE);

            pass.dispatch_workgroups(workgroups, 1, 1);
        }

        // Alternate between the two trail textures.
        let read_index = trail_index;
        let write_index = 1 - read_index;

        // Pass 2: Fade the previous frame into the other texture.
        {
            let mut pass = encoder.begin_render_pass(&RenderPassDescriptor {
                label: Some("Trail Fade Pass"),
                color_attachments: &[Some(RenderPassColorAttachment {
                    view: &trail_views[write_index],
                    depth_slice: None,
                    resolve_target: None,
                    ops: Operations {
                        load: LoadOp::Clear(Color {
                            r: 0.005,
                            g: 0.008,
                            b: 0.025,
                            a: 1.0,
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });

            pass.set_pipeline(trail_pipeline);
            pass.set_bind_group(0, &trail_bind_groups[read_index], &[]);

            // Fullscreen triangle.
            pass.draw(0..3, 0..1);
        }

        // Pass 3: Draw the current particles over the faded trails.
        {
            let mut pass = encoder.begin_render_pass(&RenderPassDescriptor {
                label: Some("Particle Render Pass"),
                color_attachments: &[Some(RenderPassColorAttachment {
                    view: &trail_views[write_index],
                    depth_slice: None,
                    resolve_target: None,
                    ops: Operations {
                        load: LoadOp::Load,
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });

            pass.set_pipeline(render_pipeline);
            pass.set_bind_group(0, render_bind_group, &[]);
            pass.draw(0..6, 0..PARTICLE_COUNT);
        }

        // Pass 4: Display the completed trail texture in the window.
        {
            let mut pass = encoder.begin_render_pass(&RenderPassDescriptor {
                label: Some("Present Trails Pass"),
                color_attachments: &[Some(RenderPassColorAttachment {
                    view: &view,
                    depth_slice: None,
                    resolve_target: None,
                    ops: Operations {
                        load: LoadOp::Clear(Color {
                            r: 0.015,
                            g: 0.02,
                            b: 0.055,
                            a: 1.0,
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });

            pass.set_pipeline(present_pipeline);
            pass.set_bind_group(0, &present_bind_groups[write_index], &[]);

            // Fullscreen triangle.
            pass.draw(0..3, 0..1);
        }

        // Submit all four passes together.
        queue.submit(Some(encoder.finish()));
        queue.present(frame);

        // The newly written texture becomes the previous frame.
        self.trail_index = write_index;

        // Request the next frame.
        if let Some(window) = &self.window {
            window.request_redraw();
        }

        let _ = particle_buffer;
    }
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        event_loop.set_control_flow(ControlFlow::Poll);

        if self.window.is_some() {
            return;
        }

        let window = Arc::new(
            event_loop
                .create_window(
                    Window::default_attributes()
                        .with_title("Triangle Lab — GPU Compute Particles")
                        .with_inner_size(winit::dpi::LogicalSize::new(800.0, 600.0)),
                )
                .expect("Failed to create window"),
        );

        let instance = Instance::default();

        let surface = instance
            .create_surface(Arc::clone(&window))
            .expect("Failed to create surface");

        let adapter = pollster::block_on(instance.request_adapter(&RequestAdapterOptions {
            power_preference: PowerPreference::HighPerformance,
            compatible_surface: Some(&surface),
            force_fallback_adapter: false,
            apply_limit_buckets: false,
        }))
        .expect("Failed to find GPU adapter");

        println!("GPU adapter: {:?}", adapter.get_info());

        let (device, queue) =
            pollster::block_on(adapter.request_device(&DeviceDescriptor::default()))
                .expect("Failed to create GPU device");

        let capabilities = surface.get_capabilities(&adapter);
        let format = capabilities
            .formats
            .iter()
            .copied()
            .find(|format| format.is_srgb())
            .unwrap_or(capabilities.formats[0]);

        let size = window.inner_size();

        let config = SurfaceConfiguration {
            usage: TextureUsages::RENDER_ATTACHMENT,
            format,
            width: size.width.max(1),
            height: size.height.max(1),
            present_mode: PresentMode::Fifo,
            desired_maximum_frame_latency: 2,
            alpha_mode: capabilities.alpha_modes[0],
            view_formats: vec![],
            color_space: wgpu::SurfaceColorSpace::Auto,
        };

        surface.configure(&device, &config);

        // Initialize particle state: [position.x, position.y,
        //                              velocity.x, velocity.y].
        let initial_particles: Vec<[f32; 4]> = (0..PARTICLE_COUNT)
            .map(|i| {
                let x = ((i * 37 % 997) as f32 / 498.5) - 1.0;
                let y = ((i * 73 % 991) as f32 / 495.5) - 1.0;

                let vx = (((i * 13 % 101) as f32 / 100.0) - 0.5) * 0.45;
                let vy = (((i * 29 % 97) as f32 / 96.0) - 0.5) * 0.45;

                [x, y, vx, vy]
            })
            .collect();

        let particle_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Particle State Storage Buffer"),
            contents: bytemuck::cast_slice(&initial_particles),
            usage: BufferUsages::STORAGE,
        });

        // 16-byte simulation uniform.
        let simulation_buffer = device.create_buffer(&BufferDescriptor {
            label: Some("Simulation Parameters"),
            size: 16,
            usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        // Compute shader and its layout.
        let compute_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Particle Compute Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("compute.wgsl").into()),
        });

        let compute_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Compute Bind Group Layout"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: ShaderStages::COMPUTE,
                    ty: BindingType::Buffer {
                        ty: BufferBindingType::Storage { read_only: false },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: ShaderStages::COMPUTE,
                    ty: BindingType::Buffer {
                        ty: BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
            ],
        });

        let compute_bind_group = device.create_bind_group(&BindGroupDescriptor {
            label: Some("Compute Bind Group"),
            layout: &compute_layout,
            entries: &[
                BindGroupEntry {
                    binding: 0,
                    resource: particle_buffer.as_entire_binding(),
                },
                BindGroupEntry {
                    binding: 1,
                    resource: simulation_buffer.as_entire_binding(),
                },
            ],
        });

        let compute_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Compute Pipeline Layout"),
                bind_group_layouts: &[Some(&compute_layout)],
                immediate_size: 0,
            });

        let compute_pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("Particle Compute Pipeline"),
            layout: Some(&compute_pipeline_layout),
            module: &compute_shader,
            entry_point: Some("cs_main"),
            compilation_options: Default::default(),
            cache: None,
        });

        // Render shader reads the same storage buffer, but read-only.
        let render_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Particle Render Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("shader.wgsl").into()),
        });
        let trail_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Trail Fade Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("trail.wgsl").into()),
        });
        let trail_format = config.format;
        let trail_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("Trail Sampler"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });

        let trail_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("Trail Bind Group Layout"),
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            sample_type: wgpu::TextureSampleType::Float { filterable: true },
                            view_dimension: wgpu::TextureViewDimension::D2,
                            multisampled: false,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                        count: None,
                    },
                ],
            });

        let create_trail_texture = || {
            device.create_texture(&wgpu::TextureDescriptor {
                label: Some("Particle Trail Texture"),
                size: wgpu::Extent3d {
                    width: config.width,
                    height: config.height,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: trail_format,
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT
                    | wgpu::TextureUsages::TEXTURE_BINDING,
                view_formats: &[],
            })
        };

        let trail_textures = [create_trail_texture(), create_trail_texture()];

        let trail_views = [
            trail_textures[0].create_view(&wgpu::TextureViewDescriptor::default()),
            trail_textures[1].create_view(&wgpu::TextureViewDescriptor::default()),
        ];

        let trail_textures = [create_trail_texture(), create_trail_texture()];

        let trail_views = [
            trail_textures[0].create_view(&wgpu::TextureViewDescriptor::default()),
            trail_textures[1].create_view(&wgpu::TextureViewDescriptor::default()),
        ];

        // Initialize both trail textures before sampling them.
        let mut trail_init_encoder =
            device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Initialize Trail Textures"),
            });

        for trail_view in &trail_views {
            let _clear_pass = trail_init_encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Clear Trail Texture"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: trail_view,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: 0.015,
                            g: 0.02,
                            b: 0.055,
                            a: 1.0,
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });
        }

        queue.submit(Some(trail_init_encoder.finish()));

        let trail_bind_groups = [
            device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("Trail Bind Group 0"),
                layout: &trail_bind_group_layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: wgpu::BindingResource::TextureView(&trail_views[0]),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::Sampler(&trail_sampler),
                    },
                ],
            }),
            device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("Trail Bind Group 1"),
                layout: &trail_bind_group_layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: wgpu::BindingResource::TextureView(&trail_views[1]),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::Sampler(&trail_sampler),
                    },
                ],
            }),
        ];

        let present_bind_groups = [
            device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("Presentation Bind Group 0"),
                layout: &trail_bind_group_layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: wgpu::BindingResource::TextureView(&trail_views[0]),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::Sampler(&trail_sampler),
                    },
                ],
            }),
            device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("Presentation Bind Group 1"),
                layout: &trail_bind_group_layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: wgpu::BindingResource::TextureView(&trail_views[1]),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::Sampler(&trail_sampler),
                    },
                ],
            }),
        ];

        let trail_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Trail Pipeline Layout"),
                bind_group_layouts: &[Some(&trail_bind_group_layout)],
                immediate_size: 0,
            });

        let trail_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Trail Fade Pipeline"),
            layout: Some(&trail_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &trail_shader,
                entry_point: Some("vs_main"),
                buffers: &[],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &trail_shader,
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: trail_format,
                    blend: None,
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
            }),
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview_mask: None,
            cache: None,
        });

        let present_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Presentation Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("present.wgsl").into()),
        });

        let present_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Presentation Pipeline Layout"),
                bind_group_layouts: &[Some(&trail_bind_group_layout)],
                immediate_size: 0,
            });

        let present_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Presentation Pipeline"),
            layout: Some(&present_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &present_shader,
                entry_point: Some("vs_main"),
                buffers: &[],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &present_shader,
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: config.format,
                    blend: None,
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
            }),
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview_mask: None,
            cache: None,
        });

        let render_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Render Bind Group Layout"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: ShaderStages::VERTEX,
                ty: BindingType::Buffer {
                    ty: BufferBindingType::Storage { read_only: true },
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        });

        let render_bind_group = device.create_bind_group(&BindGroupDescriptor {
            label: Some("Render Bind Group"),
            layout: &render_layout,
            entries: &[BindGroupEntry {
                binding: 0,
                resource: particle_buffer.as_entire_binding(),
            }],
        });

        let render_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Particle Render Pipeline Layout"),
                bind_group_layouts: &[Some(&render_layout)],
                immediate_size: 0,
            });

        let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Particle Render Pipeline"),
            layout: Some(&render_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &render_shader,
                entry_point: Some("vs_main"),
                compilation_options: Default::default(),
                buffers: &[],
            },
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            fragment: Some(wgpu::FragmentState {
                module: &render_shader,
                entry_point: Some("fs_main"),
                compilation_options: Default::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format: config.format,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            multiview_mask: None,
            cache: None,
        });

        self.window = Some(window);
        self.surface = Some(surface);
        self.device = Some(device);
        self.queue = Some(queue);
        self.config = Some(config);
        self.particle_buffer = Some(particle_buffer);
        self.simulation_buffer = Some(simulation_buffer);
        self.compute_pipeline = Some(compute_pipeline);
        self.compute_bind_group = Some(compute_bind_group);
        self.render_pipeline = Some(render_pipeline);
        self.render_bind_group = Some(render_bind_group);
        self.trail_textures = Some(trail_textures);
        self.trail_views = Some(trail_views);
        self.trail_bind_groups = Some(trail_bind_groups);
        self.trail_pipeline = Some(trail_pipeline);
        self.trail_sampler = Some(trail_sampler);
        self.trail_bind_group_layout = Some(trail_bind_group_layout);
        self.trail_format = Some(trail_format);
        self.trail_index = 0;
        self.present_bind_groups = Some(present_bind_groups);
        self.present_pipeline = Some(present_pipeline);
        self.last_frame = Instant::now();

        println!("GPU particle simulation ready: {PARTICLE_COUNT} particles");

        if let Some(window) = &self.window {
            window.request_redraw();
        }
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        window_id: WindowId,
        event: WindowEvent,
    ) {
        let Some(window) = &self.window else {
            return;
        };

        if window.id() != window_id {
            return;
        }

        match event {
            WindowEvent::CloseRequested => event_loop.exit(),

            WindowEvent::RedrawRequested => self.render(),

            WindowEvent::Resized(size) if size.width > 0 && size.height > 0 => {
                if let (Some(surface), Some(device), Some(config)) =
                    (&self.surface, &self.device, &mut self.config)
                {
                    config.width = size.width;
                    config.height = size.height;
                    surface.configure(device, config);
                }

                self.render();
            }

            _ => {}
        }
    }

    fn about_to_wait(&mut self, _event_loop: &ActiveEventLoop) {
        if let Some(window) = &self.window {
            window.request_redraw();
        }
    }
}

fn main() {
    let event_loop = EventLoop::new().expect("Failed to create event loop");

    let mut app = App::default();

    event_loop
        .run_app(&mut app)
        .expect("Failed to run application");
}
