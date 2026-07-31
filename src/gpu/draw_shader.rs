use wgpu::include_wgsl;

use crate::{
    candidate::Candidate,
    config_parse::Config,
    gpu::GpuContext,
    profiling::{Dropwatch, Metric, RuntimeStats, Stopwatch},
    utils::{self, buffer_utils},
};

pub struct DrawShader {
    pipeline: wgpu::ComputePipeline,
    bind_group_small: wgpu::BindGroup, // For the internal canvas texture
    bind_group_large: wgpu::BindGroup, // For the external canvas texture

    buffers: DrawBuffers,
}

pub struct DrawBuffers {
    /// This is the candidate that will be drawn to the canvas
    input_candidate: wgpu::Buffer,

    /// This is the small variant of the texture the shader outputs to, designed to be copied to the internal canvas buffer
    output_texture_large: wgpu::Texture,

    /// This is the larger variant and is designed to be copied to the external canvas buffer
    output_texture_small: wgpu::Texture,
}

impl DrawShader {
    pub fn init(cfg: &Config, context: &GpuContext) -> Result<Self, Box<dyn std::error::Error>> {
        let _d = Dropwatch::new("DrawShader init");

        let shader_module = context
            .device
            .create_shader_module(include_wgsl!("shaders/draw_shader.wgsl"));

        let bind_group_layout =
            context
                .device
                .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    label: Some("Draw shader: Bind group layout"),
                    entries: &[
                        // Atlas texture
                        wgpu::BindGroupLayoutEntry {
                            binding: 0,
                            visibility: wgpu::ShaderStages::COMPUTE,
                            ty: wgpu::BindingType::Texture {
                                sample_type: wgpu::TextureSampleType::Float { filterable: false },
                                view_dimension: wgpu::TextureViewDimension::D2,
                                multisampled: false,
                            },
                            count: None,
                        },
                        // Atlas entries
                        wgpu::BindGroupLayoutEntry {
                            binding: 1,
                            visibility: wgpu::ShaderStages::COMPUTE,
                            ty: wgpu::BindingType::Buffer {
                                ty: wgpu::BufferBindingType::Storage { read_only: true },
                                has_dynamic_offset: false,
                                min_binding_size: None,
                            },
                            count: None,
                        },
                        // Canvas texture
                        wgpu::BindGroupLayoutEntry {
                            binding: 2,
                            visibility: wgpu::ShaderStages::COMPUTE,
                            ty: wgpu::BindingType::Texture {
                                sample_type: wgpu::TextureSampleType::Float { filterable: false },
                                view_dimension: wgpu::TextureViewDimension::D2,
                                multisampled: false,
                            },
                            count: None,
                        },
                        // Candidate uniform
                        wgpu::BindGroupLayoutEntry {
                            binding: 3,
                            visibility: wgpu::ShaderStages::COMPUTE,
                            ty: wgpu::BindingType::Buffer {
                                ty: wgpu::BufferBindingType::Uniform,
                                has_dynamic_offset: false,
                                min_binding_size: None,
                            },
                            count: None,
                        },
                        // Output texture
                        wgpu::BindGroupLayoutEntry {
                            binding: 4,
                            visibility: wgpu::ShaderStages::COMPUTE,
                            ty: wgpu::BindingType::StorageTexture {
                                access: (wgpu::StorageTextureAccess::WriteOnly),
                                format: (wgpu::TextureFormat::Rgba8Unorm),
                                view_dimension: (wgpu::TextureViewDimension::D2),
                            },
                            count: None,
                        },
                    ],
                });

        let pipeline_layout =
            context
                .device
                .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                    label: Some("Draw shader: Pipeline layout"),
                    bind_group_layouts: &[Some(&bind_group_layout)],
                    immediate_size: 0,
                });

        let pipeline = context
            .device
            .create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                label: Some("Draw shader: Pipeline"),
                layout: Some(&pipeline_layout),
                module: &shader_module,
                entry_point: None, // One entry point for now
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                cache: None,
            });

        let buffers = DrawBuffers::new(cfg, context);

        let bind_group_small = context
            .device
            .create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("Draw shader: small bind group"),
                layout: &bind_group_layout,
                entries: &[
                    // Atlas texture
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: wgpu::BindingResource::TextureView(
                            &context
                                .buffers
                                .input_atlas_texture
                                .create_view(&wgpu::TextureViewDescriptor::default()),
                        ),
                    },
                    // Atlas entries
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: context.buffers.input_atlas_entry_buffer.as_entire_binding(),
                    },
                    // Canvas texture
                    wgpu::BindGroupEntry {
                        binding: 2,
                        resource: wgpu::BindingResource::TextureView(
                            &context
                                .buffers
                                .input_canvas_texture
                                .create_view(&wgpu::TextureViewDescriptor::default()),
                        ),
                    },
                    // Candidate uniform
                    wgpu::BindGroupEntry {
                        binding: 3,
                        resource: buffers.input_candidate.as_entire_binding(),
                    },
                    // Small output texture buffer
                    wgpu::BindGroupEntry {
                        binding: 4,
                        resource: wgpu::BindingResource::TextureView(
                            &buffers
                                .output_texture_small
                                .create_view(&wgpu::TextureViewDescriptor::default()),
                        ),
                    },
                ],
            });

        let bind_group_large = context
            .device
            .create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("Score shader: Large bind group"),
                layout: &bind_group_layout,
                entries: &[
                    // Atlas texture
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: wgpu::BindingResource::TextureView(
                            &context
                                .buffers
                                .input_atlas_texture
                                .create_view(&wgpu::TextureViewDescriptor::default()),
                        ),
                    },
                    // Atlas entries
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: context.buffers.input_atlas_entry_buffer.as_entire_binding(),
                    },
                    // Canvas texture
                    wgpu::BindGroupEntry {
                        binding: 2,
                        resource: wgpu::BindingResource::TextureView(
                            &context
                                .buffers
                                .output_canvas_texture
                                .create_view(&wgpu::TextureViewDescriptor::default()),
                        ),
                    },
                    // Candidate uniform
                    wgpu::BindGroupEntry {
                        binding: 3,
                        resource: buffers.input_candidate.as_entire_binding(),
                    },
                    // Small output texture buffer
                    wgpu::BindGroupEntry {
                        binding: 4,
                        resource: wgpu::BindingResource::TextureView(
                            &buffers
                                .output_texture_large
                                .create_view(&wgpu::TextureViewDescriptor::default()),
                        ),
                    },
                ],
            });

        return Ok(DrawShader {
            pipeline,
            bind_group_small,
            bind_group_large,
            buffers,
        });
    }

    /// Draws the candidate to the internal canvas
    pub fn run_small(
        &self,
        rs: &mut RuntimeStats,
        context: &GpuContext,
        candidate: &Candidate,
        draw_target: &wgpu::Texture,
    ) -> Result<(), Box<dyn std::error::Error>> {
        return self.run(
            rs,
            context,
            candidate,
            draw_target,
            &self.bind_group_small,
            &self.buffers.output_texture_small,
        );
    }

    /// Draws the candidate to the external canvas
    pub fn run_large(
        &self,
        rs: &mut RuntimeStats,
        context: &GpuContext,
        candidate: &Candidate,
        draw_target: &wgpu::Texture,
    ) -> Result<(), Box<dyn std::error::Error>> {
        return self.run(
            rs,
            context,
            candidate,
            draw_target,
            &self.bind_group_large,
            &self.buffers.output_texture_large,
        );
    }

    /// Draws the provided candidate to the provided texture
    fn run(
        &self,
        rs: &mut RuntimeStats,
        context: &GpuContext,
        candidate: &Candidate,
        draw_target: &wgpu::Texture,
        bind_group: &wgpu::BindGroup,
        output_texture: &wgpu::Texture,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let mut _s = Stopwatch::new();
        _s.start("DrawShader run");

        context.queue.write_buffer(
            &self.buffers.input_candidate,
            0,
            bytemuck::bytes_of(candidate),
        );

        let mut encoder = context
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Draw shader: Encoder"),
            });

        // Set up the compute pass
        // Needs its own scope because encoder.begin_compute_pass is a mutable borrow
        {
            let workgroup_size = utils::buffer_utils::compute_texture_work_group_count(
                (draw_target.width(), draw_target.height()),
                (16, 16),
            );
            // println!(
            //     "Draw shader: workgroup count: [x: {}, y: {}]",
            //     &workgroup_size.0, &workgroup_size.1
            // );
            let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("Draw shader: Compute pass"),
                timestamp_writes: None,
            });

            compute_pass.set_pipeline(&self.pipeline);
            compute_pass.set_bind_group(0, bind_group, &[]);

            compute_pass.dispatch_workgroups(workgroup_size.0, workgroup_size.1, 1);
        }

        encoder.copy_texture_to_texture(
            wgpu::TexelCopyTextureInfo {
                texture: output_texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            wgpu::TexelCopyTextureInfo {
                texture: draw_target,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            output_texture.size(),
        );

        context.queue.submit(Some(encoder.finish()));


        rs.record(Metric::DrawShader, _s.end());
        return Ok(());
    }
}

impl DrawBuffers {
    fn new(cfg: &Config, context: &GpuContext) -> Self {
        let input_candidate_buffer = context.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Draw shader: Candidate uniform"),
            size: buffer_utils::get_padded_buffer_size::<Candidate>(1),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        // This texture is sized to the smaller internal canvas texture
        let output_texture_small = context.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Draw shader: small output texture"),
            size: wgpu::Extent3d {
                width: context.buffers.input_canvas_texture.width(),
                height: context.buffers.input_canvas_texture.height(),
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::COPY_SRC | wgpu::TextureUsages::STORAGE_BINDING,
            view_formats: &[],
        });

        let output_texture_large = context.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Draw shader: small output texture"),
            size: wgpu::Extent3d {
                width: context.buffers.output_canvas_texture.width(),
                height: context.buffers.output_canvas_texture.height(),
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::COPY_SRC | wgpu::TextureUsages::STORAGE_BINDING,
            view_formats: &[],
        });

        return DrawBuffers {
            input_candidate: input_candidate_buffer,
            output_texture_large,
            output_texture_small,
        };
    }
}
