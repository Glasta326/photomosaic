use rand::{Rng, rngs::StdRng};
use wgpu::include_wgsl;

use crate::{
    candidate::Candidate,
    config_parse::Config,
    gpu::GpuContext,
    profiling::{Dropwatch, Metric, RuntimeStats, Stopwatch},
    utils::buffer_utils,
};

pub struct CandidateGenShader {
    pipeline: wgpu::ComputePipeline,
    bind_group: wgpu::BindGroup,

    buffers: CandidateBuffers,
}

struct CandidateBuffers {
    /// Encapsulates the current state of revelant values from the programs config and rng
    /// Changes after every cycle
    program_state_buffer: wgpu::Buffer,

    /// This is the array of candidates that survived this cycle, which is always cfg.survival_threshold
    /// survival threshold describes the top N candidates that get to live on, and thus this buffer will always contain N elements
    /// Changes after every cycle
    input_candidate_buffer: wgpu::Buffer,

    /// This is the buffer containing all the newly created candidate children, and their parents
    /// Changes after every cycle
    output_candidate_buffer: wgpu::Buffer,

    /// This buffer is for reading data back to the cpu memory
    readback_buffer: wgpu::Buffer,
}

/// Encapsulates the state of the program for the shader to be able to use
/// program settings, current state of the rng, ect
#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct ProgramState {
    survival_threshold: u32,
    child_count: u32,
    mutation_strength: f32,
    hue_enabled: u32,
    random_seed: u32,
    target_dimensions_x: u32,
    target_dimensions_y: u32,
}

impl CandidateGenShader {
    pub fn init(cfg: &Config, context: &GpuContext) -> Result<Self, Box<dyn std::error::Error>> {
        let _d = Dropwatch::new("CandidateGenShader init");

        let shader_module = context
            .device
            .create_shader_module(include_wgsl!("shaders/candidate_gen_shader.wgsl"));

        let bind_group_layout =
            context
                .device
                .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    label: Some("Candidate gen shader: Bind group layout"),
                    entries: &[
                        // Program state
                        wgpu::BindGroupLayoutEntry {
                            binding: 0,
                            visibility: wgpu::ShaderStages::COMPUTE,
                            ty: wgpu::BindingType::Buffer {
                                ty: wgpu::BufferBindingType::Uniform,
                                has_dynamic_offset: false,
                                min_binding_size: None,
                            },
                            count: None,
                        },
                        // Candidate buffer
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
                        // Output score buffer
                        wgpu::BindGroupLayoutEntry {
                            binding: 2,
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
                    label: Some("Candidate gen shader: Pipeline layout"),
                    bind_group_layouts: &[Some(&bind_group_layout)],
                    immediate_size: 0,
                });

        let pipeline = context
            .device
            .create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                label: Some("Candidate gen shader: Pipeline"),
                layout: Some(&pipeline_layout),
                module: &shader_module,
                entry_point: None, // One entry point for now
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                cache: None,
            });

        let buffers = CandidateBuffers::new(cfg, context);

        let bind_group = context
            .device
            .create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("Candidate gen shader: Bind group"),
                layout: &bind_group_layout,
                entries: &[
                    // Program state
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: buffers.program_state_buffer.as_entire_binding(),
                    },
                    // Candidate buffer
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: buffers.input_candidate_buffer.as_entire_binding(),
                    },
                    // Output score buffer
                    wgpu::BindGroupEntry {
                        binding: 2,
                        resource: buffers.output_candidate_buffer.as_entire_binding(),
                    },
                ],
            });

        return Ok(CandidateGenShader {
            pipeline,
            bind_group,
            buffers,
        });
    }

    pub fn run(
        &self,
        rs: &mut RuntimeStats,
        cfg: &Config,
        rng: &mut StdRng,
        context: &GpuContext,
        candidates: &Vec<Candidate>,
    ) -> Result<Vec<Candidate>, Box<dyn std::error::Error>> {
        let mut sw = Stopwatch::new();
        sw.start(None);

        // Write current program state into gpu memory
        context.queue.write_buffer(
            &self.buffers.program_state_buffer,
            0,
            bytemuck::bytes_of(&ProgramState::new(cfg, rng)),
        );

        // Copy data into our candidate buffer
        context.queue.write_buffer(
            &self.buffers.input_candidate_buffer,
            0,
            bytemuck::cast_slice(candidates),
        );

        let mut encoder = context
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Candidate gen shader: Encoder"),
            });

        // Set up the compute pass
        // Needs its own scope because encoder.begin_compute_pass is a mutable borrow
        {
            let workgroup_count_x = cfg.survival_threshold.div_ceil(16) as u32;
            let workgroup_count_y = cfg.extra_data.child_count.div_ceil(16) as u32;
            // println!("Score shader: true workgroup count: {}", &workgroup_count);
            let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("Score shader: Compute pass"),
                timestamp_writes: None,
            });

            compute_pass.set_pipeline(&self.pipeline);
            compute_pass.set_bind_group(0, &self.bind_group, &[]);

            compute_pass.dispatch_workgroups(workgroup_count_x, workgroup_count_y, 1);
        }

        // Get data into a mapped buffer so CPU can read it
        encoder.copy_buffer_to_buffer(
            &self.buffers.output_candidate_buffer,
            0,
            &self.buffers.readback_buffer,
            0,
            self.buffers.output_candidate_buffer.size(),
        );
        context.queue.submit(Some(encoder.finish()));

        let result_slice = self.buffers.readback_buffer.slice(std::ops::RangeFull);
        result_slice.map_async(wgpu::MapMode::Read, |_| {});
        context.device.poll(wgpu::PollType::wait_indefinitely())?;

        let result: Vec<Candidate> =
            bytemuck::allocation::pod_collect_to_vec(&result_slice.get_mapped_range());

        self.buffers.readback_buffer.unmap();

        rs.record(Metric::CandidateReproduction, sw.elapse());
        return Ok(result);
    }
}

impl CandidateBuffers {
    fn new(cfg: &Config, context: &GpuContext) -> Self {
        let program_state_buffer = context.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Candidate gen shader: Program state buffer"),
            size: buffer_utils::get_padded_buffer_size::<ProgramState>(1), // we only take in the top n candidates
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let candidate_data_buffer = context.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Candidate gen shader: Candidate data buffer"),
            size: buffer_utils::get_padded_buffer_size::<Candidate>(cfg.survival_threshold), // we only take in the top n candidates
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        // Candidates per generation its the maximum amount of active candidates at any given moment
        // So during scoring, the candidate count = CPG
        // after culling, the candidates techinically all exist but only up to N (where N is the survival threshold) candidates are actually relevent
        // after all N candidates produce M children, the candidate count will be back up to approximatley CPG
        let buffer_size =
            buffer_utils::get_padded_buffer_size::<Candidate>(cfg.candidates_per_generation);

        let output_candidate_buffer = context.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Candidate gen shader: Output buffer"),
            size: buffer_size,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });

        let readback_buffer = context.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Candidate gen shader: Readback buffer"),
            size: buffer_size,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });

        return CandidateBuffers {
            program_state_buffer,
            input_candidate_buffer: candidate_data_buffer,
            output_candidate_buffer,
            readback_buffer,
        };
    }
}

impl ProgramState {
    pub fn new(cfg: &Config, rng: &mut StdRng) -> Self {
        return ProgramState {
            survival_threshold: cfg.survival_threshold as u32,
            child_count: cfg.extra_data.child_count as u32,
            mutation_strength: cfg.mutation_strength,
            hue_enabled: if cfg.enable_hue { 1 } else { 0 },
            random_seed: rng.next_u32(),
            target_dimensions_x: cfg.extra_data.target_dimensions.0,
            target_dimensions_y: cfg.extra_data.target_dimensions.1,
        };
    }
}
