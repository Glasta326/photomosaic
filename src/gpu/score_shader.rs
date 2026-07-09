use image::RgbaImage;
use wgpu::{TexelCopyBufferLayout, TextureUsages, include_wgsl};

use crate::{
    candidate::Candidate, config_parse::Config, gpu::context::GpuContext, profiling::Dropwatch, utils::buffer_utils,
};

pub struct ScoreShader {
    pipeline: wgpu::ComputePipeline,
    bind_group: wgpu::BindGroup,

    buffers: ScoreBuffers,
}

struct ScoreBuffers {
    /// This is the array of candidate info, for example, an entry may look like: {image_id: 0, rotation: 0.2324, scale: 1.56}.
    /// Changes after every cycle
    input_candidate_buffer: wgpu::Buffer,

    /// This is the buffer the final color difference score is put into for a candiate.
    /// Changes after every cycle
    output_score_buffer: wgpu::Buffer,

    /// This buffer is for reading data back to the cpu memory
    readback_buffer: wgpu::Buffer,
}

impl ScoreShader {
    pub fn init(cfg: &Config, context: &GpuContext) -> Result<Self, Box<dyn std::error::Error>> {
        let _d = Dropwatch::new("ScoreShader init");
        
        let shader_module = context
            .device
            .create_shader_module(include_wgsl!("shaders/score_shader.wgsl"));

        // Could be created automatically from the shader file, but i've heard it's better to do it manually
        let bind_group_layout =
            context
                .device
                .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    label: Some("Score shader: Bind group layout"),
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
                        // Candidate buffer
                        wgpu::BindGroupLayoutEntry {
                            binding: 4,
                            visibility: wgpu::ShaderStages::COMPUTE,
                            ty: wgpu::BindingType::Buffer {
                                ty: wgpu::BufferBindingType::Storage { read_only: true },
                                has_dynamic_offset: false,
                                min_binding_size: None,
                            },
                            count: None,
                        },
                        // Output score buffer
                        wgpu::BindGroupLayoutEntry {
                            binding: 5,
                            visibility: wgpu::ShaderStages::COMPUTE,
                            ty: wgpu::BindingType::Buffer {
                                ty: wgpu::BufferBindingType::Storage { read_only: false },
                                has_dynamic_offset: false,
                                min_binding_size: None,
                            },
                            count: None,
                        },
                    ],
                });

        let pipeline_layout =
            context
                .device
                .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                    label: Some("Score shader: Pipeline layout"),
                    bind_group_layouts: &[Some(&bind_group_layout)],
                    immediate_size: 0,
                });

        let pipeline = context
            .device
            .create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                label: Some("Score shader: Pipeline"),
                layout: Some(&pipeline_layout),
                module: &shader_module,
                entry_point: None, // One entry point for now
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                cache: None,
            });

        let buffers = ScoreBuffers::new(cfg, context);

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
                        resource: buffers.input_candidate_buffer.as_entire_binding(),
                    },
                    // Output score buffer
                    wgpu::BindGroupEntry {
                        binding: 5,
                        resource: buffers.output_score_buffer.as_entire_binding(),
                    },
                ],
            });

        return Ok(ScoreShader {
            pipeline,
            bind_group,
            buffers,
        });
    }

    pub fn run(
        &self,
        cfg: &Config,
        context: &GpuContext,
        candidates: &Vec<Candidate>,
    ) -> Result<Vec<f32>, Box<dyn std::error::Error>> {
        //let _d = Dropwatch::new("ScoreShader run");
        
        // Copy data into our candidate buffer
        context.queue.write_buffer(
            &self.buffers.input_candidate_buffer,
            0,
            bytemuck::cast_slice(candidates),
        );

        let mut encoder = context
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Score shader: Encoder"),
            });

        // Set up the compute pass
        // Needs its own scope because encoder.begin_compute_pass is a mutable borrow
        {
            let workgroup_count = cfg.candidates_per_generation.div_ceil(64);
            // println!("Score shader: true workgroup count: {}", &workgroup_count);
            let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("Score shader: Compute pass"),
                timestamp_writes: None,
            });

            compute_pass.set_pipeline(&self.pipeline);
            compute_pass.set_bind_group(0, &self.bind_group, &[]);

            compute_pass.dispatch_workgroups(workgroup_count as u32, 1, 1);
        }

        // Get data into a mapped buffer so CPU can read it
        encoder.copy_buffer_to_buffer(
            &self.buffers.output_score_buffer,
            0,
            &self.buffers.readback_buffer,
            0,
            self.buffers.output_score_buffer.size(),
        );
        context.queue.submit(Some(encoder.finish()));

        // Collect the buffer into a float vec and return it
        let result_slice = self.buffers.readback_buffer.slice(std::ops::RangeFull);
        result_slice.map_async(wgpu::MapMode::Read, |_| {});
        context.device.poll(wgpu::PollType::wait_indefinitely())?;

        let result: Vec<f32> =
            bytemuck::allocation::pod_collect_to_vec(&result_slice.get_mapped_range());

        // Unmap readback buffer
        // Unmapping means "This is no longer being read by the CPU"
        self.buffers.readback_buffer.unmap();

        return Ok(result);
    }
}

impl ScoreBuffers {
    fn new(cfg: &Config, context: &GpuContext) -> Self {
        let candidate_data_buffer = context.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Score shader: Candidate data buffer"),
            size: buffer_utils::get_padded_buffer_size::<Candidate>(cfg.candidates_per_generation),
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        // Pre-calculate because it's used twice
        // Yeah i know the compiler will optimise it away anyway but let me have a win ok
        let buffer_size =
            buffer_utils::get_padded_buffer_size::<f32>(cfg.candidates_per_generation);

        let output_score_buffer = context.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Score shader: Output buffer"),
            size: buffer_size,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });

        let readback_buffer = context.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Score shader: Readback buffer"),
            size: buffer_size,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });

        return ScoreBuffers {
            input_candidate_buffer: candidate_data_buffer,
            output_score_buffer,
            readback_buffer,
        };
    }
}
