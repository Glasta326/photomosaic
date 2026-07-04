use image::RgbaImage;
use wgpu::{TextureUsages, include_wgsl, util::DeviceExt};

use crate::{
    candidate::{self, Candidate},
    config_parse::{self, Config},
    data_reader::{self, AtlasEntry},
};

pub struct Compute {
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
    pub pipeline: wgpu::ComputePipeline,

    instance: wgpu::Instance,
    adapter: wgpu::Adapter,
    bind_group_layout: wgpu::BindGroupLayout,

    pub buffers: Buffers,
}

struct Buffers {
    /// This is the texture containing all of the images the user wants to be in the pallette.
    /// Never changes after initalisation
    pub input_atlas_texture: wgpu::Texture,

    /// This is the reference data for the atlas, so when the shader sees {image_id:0,...},
    /// it looks into this buffer at id[0] to get posiiton and height information for the correct image in the atlas.
    /// Never changes after initalisation
    pub input_atlas_entry_buffer: wgpu::Buffer,

    /// This is the image the user wants to re-create from the pallette.
    /// Never changes after initalisation
    pub input_target_texture: wgpu::Texture,

    /// This is the canvas that is drawn to over time.
    /// Changes after every cycle
    pub input_canvas_texture: wgpu::Texture,

    /// This is the array of candidate info, for example, an entry may look like: {image_id: 0, rotation: 0.2324, scale: 1.56}.
    /// Changes after every cycle
    pub input_candidate_buffer: wgpu::Buffer,

    /// This is the buffer the final color difference score is put into for a candiate.
    /// Changes after every cycle
    pub output_score_buffer: wgpu::Buffer,

    /// This buffer is for reading data back to the cpu memory
    pub readback_buffer: wgpu::Buffer,
}

impl Compute {
    pub fn new_init(
        cfg: &Config,
        atlas_texture: RgbaImage,
        atlas_entries: Vec<AtlasEntry>,
        target_texture: RgbaImage,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let _descriptor = wgpu::InstanceDescriptor::new_without_display_handle();
        let _instance = wgpu::Instance::new(_descriptor);
        let _adapter =
            pollster::block_on(_instance.request_adapter(&wgpu::RequestAdapterOptions::default()))?;
        println!("Created adapter: \n{:#?}", _adapter.get_info());

        // Ensure adapter supports compute shaders
        let supports_compute = _adapter
            .get_downlevel_capabilities()
            .flags
            .contains(wgpu::DownlevelFlags::COMPUTE_SHADERS);
        if !supports_compute {
            return Err("Gpu does not support compute shaders".into());
        }

        let (_device, _queue) =
            pollster::block_on(_adapter.request_device(&wgpu::DeviceDescriptor::default()))?;

        // Once all the device and hardware stuff has been attained, we create all the constant elements that don't change like BGL and shader ect

        // Load in the shader
        let shader_module =
            _device.create_shader_module(include_wgsl!("../shaders/score_shader.wgsl"));

        // Could be created automatically from the shader file, but i've heard it's better to do it manually
        let bind_group_layout =
            _device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("Bind group layout"),
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

        let pipeline_layout = _device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Pipeline layout"),
            bind_group_layouts: &[Some(&bind_group_layout)],
            immediate_size: 0,
        });

        let pipeline = _device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("Pipeline"),
            layout: Some(&pipeline_layout),
            module: &shader_module,
            entry_point: None, // One entry point for now
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            cache: None,
        });

        let buffers = Self::buffer_init(
            &_device,
            &_queue,
            &cfg,
            atlas_texture,
            atlas_entries,
            target_texture,
        );

        // TODO: this kind of sucks
        // mayb put all the buffers into their own struct?
        return Ok(Compute {
            device: _device,
            queue: _queue,
            pipeline: pipeline,
            instance: _instance,
            adapter: _adapter,
            bind_group_layout: bind_group_layout,
            buffers,
        });
    }

    fn buffer_init(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        cfg: &Config,
        atlas_texture: RgbaImage,
        atlas_entries: Vec<AtlasEntry>,
        target_texture: RgbaImage,
    ) -> Buffers {
        let atlas_texture_size = wgpu::Extent3d {
            width: atlas_texture.width(),
            height: atlas_texture.height(),
            depth_or_array_layers: 1,
        };

        // Our output texture, canvas texture and target texture are all the same size, so we use this for everything
        let canvas_texture_size = wgpu::Extent3d {
            width: target_texture.width(),
            height: target_texture.height(),
            depth_or_array_layers: 1,
        };

        // Modified from .create_buffer_init()'s logic
        let align_mask = wgpu::COPY_BUFFER_ALIGNMENT - 1;
        let padded_candidate_buffer_size = ((((cfg.candidates_per_generation
            * std::mem::size_of::<candidate::Candidate>())
            as u64)
            + align_mask)
            & !align_mask)
            .max(wgpu::COPY_BUFFER_ALIGNMENT);
        // Worth noting we'll need the same number of these as we do candidates, so the count is re-used
        let padded_output_score_buffer_size =
            ((((cfg.candidates_per_generation * std::mem::size_of::<f32>()) as u64) + align_mask)
                & !align_mask)
                .max(wgpu::COPY_BUFFER_ALIGNMENT);

        let atlas_texture_buffer = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Atlas texture buffer"),
            size: atlas_texture_size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: TextureUsages::TEXTURE_BINDING | TextureUsages::COPY_DST, // COPY_DST because we need to copy the image file onto it during init
            view_formats: &[],
        });

        let atlas_entry_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Atlas entry buffer"),
            contents: bytemuck::cast_slice(&atlas_entries),
            usage: wgpu::BufferUsages::STORAGE,
        });

        let target_texture_buffer = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Target texture buffer"),
            size: canvas_texture_size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: TextureUsages::TEXTURE_BINDING | TextureUsages::COPY_DST, // Ditto
            view_formats: &[],
        });

        // Upload both static textures
        queue.write_texture(
            atlas_texture_buffer.as_image_copy(),
            bytemuck::cast_slice(atlas_texture.as_raw()),
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(4 * atlas_texture.width()),
                rows_per_image: None, // Only one texture so unneeded
            },
            atlas_texture_size,
        );
        queue.write_texture(
            target_texture_buffer.as_image_copy(),
            bytemuck::cast_slice(target_texture.as_raw()),
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(4 * target_texture.width()),
                rows_per_image: None, // Only one texture so unneeded
            },
            canvas_texture_size,
        );

        let canvas_texture_buffer = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Canvas texture buffer"),
            size: canvas_texture_size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: TextureUsages::TEXTURE_BINDING
                | TextureUsages::STORAGE_BINDING
                | TextureUsages::COPY_DST, // Ditto
            view_formats: &[],
        });

        let candidate_data_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Candidate data buffer"),
            size: padded_candidate_buffer_size,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let output_score_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Output score buffer"),
            size: padded_output_score_buffer_size,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });

        let readback_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Readback buffer"),
            size: padded_output_score_buffer_size,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });

        return Buffers {
            input_atlas_texture: atlas_texture_buffer,
            input_atlas_entry_buffer: atlas_entry_buffer,
            input_target_texture: target_texture_buffer,
            input_canvas_texture: canvas_texture_buffer,
            input_candidate_buffer: candidate_data_buffer,
            output_score_buffer,
            readback_buffer,
        };
    }

    pub fn run(
        &self,
        cfg: &Config,
        canvas: RgbaImage,
        candidates: Vec<Candidate>,
    ) -> Result<Vec<f32>, Box<dyn std::error::Error>> {
        let bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Bind group"),
            layout: &self.bind_group_layout,
            entries: &[
                // Atlas texture
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(
                        &self
                            .buffers
                            .input_atlas_texture
                            .create_view(&wgpu::TextureViewDescriptor::default()),
                    ),
                },
                // Atlas entries
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: self.buffers.input_atlas_entry_buffer.as_entire_binding(),
                },
                // Target texture
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::TextureView(
                        &self
                            .buffers
                            .input_target_texture
                            .create_view(&wgpu::TextureViewDescriptor::default()),
                    ),
                },
                // Canvas texture
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: wgpu::BindingResource::TextureView(
                        &self
                            .buffers
                            .input_canvas_texture
                            .create_view(&wgpu::TextureViewDescriptor::default()),
                    ),
                },
                // Candidate buffer
                wgpu::BindGroupEntry {
                    binding: 4,
                    resource: self.buffers.input_candidate_buffer.as_entire_binding(),
                },
                // Output score buffer
                wgpu::BindGroupEntry {
                    binding: 5,
                    resource: self.buffers.output_score_buffer.as_entire_binding(),
                },
            ],
        });

        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Encoder"),
            });

        // Set up the compute pass
        // Needs its own scope because encoder.begin_compute_pass is a mutable borrow
        {
            let workgroup_count = cfg.candidates_per_generation.div_ceil(64);
            let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("Compute pass"),
                timestamp_writes: None,
            });

            compute_pass.set_pipeline(&self.pipeline);
            compute_pass.set_bind_group(0, &bind_group, &[]);

            compute_pass.dispatch_workgroups(workgroup_count as u32, 0, 0);
        }

        // Get data into a mapped buffer so CPU can read it
        encoder.copy_buffer_to_buffer(
            &self.buffers.output_score_buffer,
            0,
            &self.buffers.readback_buffer,
            0,
            self.buffers.output_score_buffer.size(),
        );
        self.queue.submit(Some(encoder.finish()));

        // Collect the buffer into a float vec and return it
        let result_slice = self.buffers.readback_buffer.slice(std::ops::RangeFull);
        result_slice.map_async(wgpu::MapMode::Read, |_| {});
        self.device.poll(wgpu::PollType::wait_indefinitely())?;

        let result: Vec<f32> =
            bytemuck::allocation::pod_collect_to_vec(&result_slice.get_mapped_range());

        self.buffers.readback_buffer.unmap();

        return Ok(result);
    }
}
