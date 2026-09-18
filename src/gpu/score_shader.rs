use std::process::exit;

use image::RgbaImage;
use wgpu::{TexelCopyBufferLayout, TextureUsages, include_wgsl};

use crate::{
    candidate::Candidate,
    config_parse::Config,
    gpu::context::GpuContext,
    profiling::{Dropwatch, Metric, RuntimeStats, Stopwatch},
    utils::buffer_utils,
};

pub struct ScoreShader {
    score_pipeline: wgpu::ComputePipeline,
    reduce_pipeline: wgpu::ComputePipeline,
    bind_group: wgpu::BindGroup,

    buffers: ScoreBuffers,
}

struct ScoreBuffers {
    /// This is the array of candidate info, for example, an entry may look like: {image_id: 0, rotation: 0.2324, scale: 1.56}.
    /// Changes after every cycle
    input_candidate_buffer: wgpu::Buffer,

    /// These hold the intermediate pixel scoring, before the reduction shader processes them and outputs the final candidate score in output_score_buffer
    /// These change after every cycle
    /// WARNING: These buffers can get extremely large. A 1920x1080 canvas with 1000 candidates will consume 8,294,400,000 (~8.2GB) bytes of VRAM!
    /// WARNING: In such case, each buffer would reach ~2GB on its own
    internal_pixel_score_buffer_0: wgpu::Buffer,
    internal_pixel_score_buffer_1: wgpu::Buffer,
    internal_pixel_score_buffer_2: wgpu::Buffer,
    internal_pixel_score_buffer_3: wgpu::Buffer,

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
                        // Internal pixel scores buffers
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
                        wgpu::BindGroupLayoutEntry {
                            binding: 6,
                            visibility: wgpu::ShaderStages::COMPUTE,
                            ty: wgpu::BindingType::Buffer {
                                ty: wgpu::BufferBindingType::Storage { read_only: false },
                                has_dynamic_offset: false,
                                min_binding_size: None,
                            },
                            count: None,
                        },
                        wgpu::BindGroupLayoutEntry {
                            binding: 7,
                            visibility: wgpu::ShaderStages::COMPUTE,
                            ty: wgpu::BindingType::Buffer {
                                ty: wgpu::BufferBindingType::Storage { read_only: false },
                                has_dynamic_offset: false,
                                min_binding_size: None,
                            },
                            count: None,
                        },
                        wgpu::BindGroupLayoutEntry {
                            binding: 8,
                            visibility: wgpu::ShaderStages::COMPUTE,
                            ty: wgpu::BindingType::Buffer {
                                ty: wgpu::BufferBindingType::Storage { read_only: false },
                                has_dynamic_offset: false,
                                min_binding_size: None,
                            },
                            count: None,
                        },
                        // Output score buffer
                        wgpu::BindGroupLayoutEntry {
                            binding: 9,
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

        let score_pipeline =
            context
                .device
                .create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                    label: Some("Score shader: Score pipeline"),
                    layout: Some(&pipeline_layout),
                    module: &shader_module,
                    entry_point: Some("score_3D"),
                    compilation_options: wgpu::PipelineCompilationOptions::default(),
                    cache: None,
                });

        let reduce_pipeline =
            context
                .device
                .create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                    label: Some("Score shader: Reduction pipeline"),
                    layout: Some(&pipeline_layout),
                    module: &shader_module,
                    entry_point: Some("reduce"),
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
                                .scaled_canvas_texture
                                .create_view(&wgpu::TextureViewDescriptor::default()),
                        ),
                    },
                    // Candidate buffer
                    wgpu::BindGroupEntry {
                        binding: 4,
                        resource: buffers.input_candidate_buffer.as_entire_binding(),
                    },
                    // Internal pixel scores buffers
                    wgpu::BindGroupEntry {
                        binding: 5,
                        resource: buffers.internal_pixel_score_buffer_0.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 6,
                        resource: buffers.internal_pixel_score_buffer_1.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 7,
                        resource: buffers.internal_pixel_score_buffer_2.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 8,
                        resource: buffers.internal_pixel_score_buffer_3.as_entire_binding(),
                    },
                    // Output score buffer
                    wgpu::BindGroupEntry {
                        binding: 9,
                        resource: buffers.output_score_buffer.as_entire_binding(),
                    },
                ],
            });

        return Ok(ScoreShader {
            score_pipeline,
            reduce_pipeline,
            bind_group,
            buffers,
        });
    }

    pub fn run(
        &self,
        rs: &mut RuntimeStats,
        cfg: &Config,
        context: &GpuContext,
        candidates: &Vec<Candidate>,
    ) -> Result<Vec<f32>, Box<dyn std::error::Error>> {
        let mut sw = Stopwatch::new();
        sw.start(None);

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

        // Set up the compute passes
        // Needs its own scope because encoder.begin_compute_pass is a mutable borrow
        {
            let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("Score shader: Compute pass"),
                timestamp_writes: None,
            });

            // Takes the candidates, and generates a score for every candidate/pixel
            compute_pass.set_pipeline(&self.score_pipeline);
            compute_pass.set_bind_group(0, &self.bind_group, &[]);
            compute_pass.dispatch_workgroups(
                cfg.candidates_per_generation.div_ceil(4) as u32,
                cfg.extra_data.target_dimensions.0.div_ceil(8),
                cfg.extra_data.target_dimensions.1.div_ceil(8),
            );

            // Takes the result fom the scoring shader, and condenses the candidate * pixel_y * pixel_y sized score array into a score for each candidate
            // Takes the candidates, and generates a score for every candidate/pixel
            compute_pass.set_pipeline(&self.reduce_pipeline);
            compute_pass.set_bind_group(0, &self.bind_group, &[]);
            compute_pass.dispatch_workgroups(cfg.candidates_per_generation as u32, 1, 1);
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

        rs.record(Metric::ScoreShader, sw.elapse());
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

        // This scales with canvas width, height, and candidate count. Thus this tends to get extremely large
        //TODO: Potentially make our owned fixed-point data type with as few bit as possible instead of using f32
        // 1920x1080x1000candidate image results in 2073600000 values
        // using f32, each value is 4 bytes, so that results in nearly 8gb of vram usage
        // the internal score value will never exceed 2, as colors are 4d where each value ranges from 0-1, so 4d pythag distance gives a maximum of 2
        // that means we could split it:
        // 2 .765625
        // 10.110001
        // if we want to use a single byte,
        // however that might not be precise enough, but using 2 bytes:
        // 2 .76568603515625
        // 10.11000100000001
        // could also just not use floats and have the pixel score be calculated as u16 or something and just map it back into floats during reduction?
        //
        //TODO:
        // ALSO TODO:
        // perhaps just make a second buffer
        // because we get limited by the adapter's maximum storage buffer size
        // but we could also just make a second/3rd buffer
        // and dynamically populate all 4 instead
        // the code isnt too far off being able to do that anwyay
        // instead of linearly just spilling into the next, we just fill like
        // buf1: [0,4,8,12,...]
        // buf2: [1,5,9,13,...]
        // buf3: [2,6,10,14,...]
        // buf4: [3,7,11,15,...]
        // just like how we manually calculate the candidate padding for the single buffer, we just change that so it splits among the 4
        let pixel_score_buffer_size = buffer_utils::get_padded_buffer_size::<f32>(
            (cfg.candidates_per_generation).div_ceil(4)
                * cfg.extra_data.target_dimensions.0 as usize
                * cfg.extra_data.target_dimensions.1 as usize,
        );

        let ttt = buffer_utils::get_padded_buffer_size::<f32>(
            (cfg.candidates_per_generation)
                * cfg.extra_data.target_dimensions.0 as usize
                * cfg.extra_data.target_dimensions.1 as usize,
        );
        println!("buffer {} bytes",ttt);

        let internal_pixel_score_buffer_0 = context.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Score shader: Internal pixel score buffer 0"),
            size: pixel_score_buffer_size,
            usage: wgpu::BufferUsages::STORAGE,
            mapped_at_creation: false,
        });
        let internal_pixel_score_buffer_1 = context.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Score shader: Internal pixel score buffer 1"),
            size: pixel_score_buffer_size,
            usage: wgpu::BufferUsages::STORAGE,
            mapped_at_creation: false,
        });
        let internal_pixel_score_buffer_2 = context.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Score shader: Internal pixel score buffer 2"),
            size: pixel_score_buffer_size,
            usage: wgpu::BufferUsages::STORAGE,
            mapped_at_creation: false,
        });
        let internal_pixel_score_buffer_3 = context.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Score shader: Internal pixel score buffer 3"),
            size: pixel_score_buffer_size,
            usage: wgpu::BufferUsages::STORAGE,
            mapped_at_creation: false,
        });

        // Pre-calculate because it's used twice
        // Yeah i know the compiler will optimise it away anyway but let me have a win ok
        let output_buffer_size =
            buffer_utils::get_padded_buffer_size::<f32>(cfg.candidates_per_generation);

        let output_score_buffer = context.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Score shader: Output buffer"),
            size: output_buffer_size,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });

        let readback_buffer = context.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Score shader: Readback buffer"),
            size: output_buffer_size,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });

        return ScoreBuffers {
            input_candidate_buffer: candidate_data_buffer,
            internal_pixel_score_buffer_0,
            internal_pixel_score_buffer_1,
            internal_pixel_score_buffer_2,
            internal_pixel_score_buffer_3,
            output_score_buffer,
            readback_buffer,
        };
    }
}
