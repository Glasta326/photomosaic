use image::{ImageBuffer, Rgb};
use wgpu::include_wgsl;

use crate::config_parse;

pub struct compute {
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
    pub pipeline: wgpu::ComputePipeline,

    instance: wgpu::Instance,
    adapter: wgpu::Adapter,
    bind_group_layout: wgpu::BindGroupLayout,

    /// This is the texture containing all of the images the user wants to be in the pallette.
    /// Never changes after initalisation
    pub input_atlas_texture: Option<wgpu::Texture>,

    /// This is the reference data for the atlas, so when the shader sees {image_id:0,...},
    /// it looks into this buffer at id[0] to get posiiton and height information for the correct image in the atlas.
    /// Never changes after initalisation
    pub input_atlas_key_buffer: Option<wgpu::Buffer>,

    /// This is the image the user wants to re-create from the pallette.
    /// Never changes after initalisation
    pub input_target_texture: Option<wgpu::Texture>,

    /// This is the canvas that is drawn to over time.
    /// Changes after every cycle
    pub input_canvas_texture: Option<wgpu::Texture>,

    /// This is the array of candidate info, for example, an entry may look like: {image_id: 0, rotation: 0.2324, scale: 1.56}.
    /// Changes after every cycle
    pub input_candidate_buffer: Option<wgpu::Buffer>,

    /// This is the buffer the final color difference score is put into for a candiate.
    /// Changes after every cycle
    pub output_score_buffer: Option<wgpu::Buffer>,
}

pub struct ComputeSystemInitData {}

impl compute {
    pub fn new() -> Result<Self, Box<dyn std::error::Error>> {
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
        // TODO: update this for the shader later
        let bind_group_layout =
            _device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("Bind group layout"),
                entries: &[
                    // Input texture, so in this case: The atlas texture
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Texture {
                            sample_type: (wgpu::TextureSampleType::Float {
                                filterable: (false),
                            }),
                            view_dimension: (wgpu::TextureViewDimension::D2),
                            multisampled: (false),
                        },
                        count: None,
                    },
                    // Output texture
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::StorageTexture {
                            access: (wgpu::StorageTextureAccess::WriteOnly),
                            format: (wgpu::TextureFormat::Rgba8Unorm),
                            view_dimension: (wgpu::TextureViewDimension::D2),
                        },
                        count: None,
                    },
                    // Input data buffer, so our atlas metadata structs
                    wgpu::BindGroupLayoutEntry {
                        binding: 2,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Buffer {
                            ty: (wgpu::BufferBindingType::Storage { read_only: (true) }),
                            has_dynamic_offset: (false),
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

        // TODO: this kind of sucks
        // mayb put all the buffers into their own struct?
        return Ok(compute {
            device: _device,
            queue: _queue,
            pipeline: pipeline,
            instance: _instance,
            adapter: _adapter,
            bind_group_layout: bind_group_layout,
            input_atlas_texture: None,
            input_atlas_key_buffer: None,
            input_target_texture: None,
            input_canvas_texture: None,
            input_candidate_buffer: None,
            output_score_buffer: None,
        });
    }

    pub fn buffer_init(
        &mut self,
        cfg: config_parse::Config,
        atlas_texture: ImageBuffer<Rgb<u8>, Vec<u8>>,
        //atlas_keys: Vec<AtlasKey>,
        target_texture: ImageBuffer<Rgb<u8>, Vec<u8>>
    ) {

        
    }
}
