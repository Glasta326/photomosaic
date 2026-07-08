use wgpu::include_wgsl;

use crate::{
    candidate::Candidate,
    config_parse::Config,
    gpu::GpuContext,
    profiling::Dropwatch,
    utils::{self, buffer_utils},
};

pub struct DrawShader {
    pipeline: wgpu::ComputePipeline,
    bind_group: wgpu::BindGroup,

    buffers: DrawBuffers,
}

pub struct DrawBuffers {
    /// This is the candidate that will be drawn to the canvas
    input_candidate: wgpu::Buffer,

    /// This is the texture that the shader will output that is then copied to the canvas texture
    output_texture: wgpu::Texture,
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
                        // Target texture
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
                        // Canvas texture
                        wgpu::BindGroupLayoutEntry {
                            binding: 3,
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
                            binding: 4,
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
                            binding: 5,
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

        let bind_group = context
            .device
            .create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("Score shader: Bind group"),
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
                    // Target texture
                    wgpu::BindGroupEntry {
                        binding: 2,
                        resource: wgpu::BindingResource::TextureView(
                            &context
                                .buffers
                                .input_target_texture
                                .create_view(&wgpu::TextureViewDescriptor::default()),
                        ),
                    },
                    // Canvas texture
                    wgpu::BindGroupEntry {
                        binding: 3,
                        resource: wgpu::BindingResource::TextureView(
                            &context
                                .buffers
                                .input_canvas_texture
                                .create_view(&wgpu::TextureViewDescriptor::default()),
                        ),
                    },
                    // Candidate buffer
                    wgpu::BindGroupEntry {
                        binding: 4,
                        resource: buffers.input_candidate.as_entire_binding(),
                    },
                    // Output score buffer
                    wgpu::BindGroupEntry {
                        binding: 5,
                        resource: wgpu::BindingResource::TextureView(
                            &buffers
                                .output_texture
                                .create_view(&wgpu::TextureViewDescriptor::default()),
                        ),
                    },
                ],
            });

        return Ok(DrawShader {
            pipeline,
            bind_group,
            buffers,
        });
    }

    pub fn run(
        &self,
        context: &GpuContext,
        candidate: &Candidate,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let _d = Dropwatch::new("DrawShader run");
        
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
                (
                    context.buffers.input_canvas_texture.width(),
                    context.buffers.input_canvas_texture.height(),
                ),
                (16, 16),
            );
            println!(
                "Draw shader: workgroup count: [x: {}, y: {}]",
                &workgroup_size.0, &workgroup_size.1
            );
            let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("Draw shader: Compute pass"),
                timestamp_writes: None,
            });

            compute_pass.set_pipeline(&self.pipeline);
            compute_pass.set_bind_group(0, &self.bind_group, &[]);

            compute_pass.dispatch_workgroups(workgroup_size.0, workgroup_size.1, 1);
        }

        encoder.copy_texture_to_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &self.buffers.output_texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            wgpu::TexelCopyTextureInfo {
                texture: &context.buffers.input_canvas_texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            self.buffers.output_texture.size(), // While the buffer is the same size as the canvas buffer. Just to be sure we use the output buffer
        );

        context.queue.submit(Some(encoder.finish()));

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

        let output_texture = context.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Draw shader: output texture"),
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

        return DrawBuffers {
            input_candidate: input_candidate_buffer,
            output_texture,
        };
    }
}
