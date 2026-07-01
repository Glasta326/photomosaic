use std::num::NonZeroU64;


pub struct compute_system{
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
    pub pipeline: wgpu::ComputePipeline,

    instance: wgpu::Instance,
    adapter: wgpu::Adapter,
    bind_group_layout: wgpu::BindGroupLayout
}

impl compute_system{
    pub fn new() -> Self{
        let _descriptor = wgpu::InstanceDescriptor::new_without_display_handle();
        let _instance = wgpu::Instance::new(_descriptor);
        let _adapter =
            pollster::block_on(_instance.request_adapter(&wgpu::RequestAdapterOptions::default()))
                .expect("Failed to create adapter");
        println!("Created adapter: \n{:#?}", _adapter.get_info());

        // Ensure adapter supports compute shaders
        let supports_compute = _adapter
            .get_downlevel_capabilities()
            .flags
            .contains(wgpu::DownlevelFlags::COMPUTE_SHADERS);
        if !supports_compute {
            panic!("Gpu does not support compute shaders");
        }

        let (_device, _queue) =
            pollster::block_on(_adapter.request_device(&wgpu::DeviceDescriptor::default()))
                .expect("Failed to create device and queue");

        let bind_group_layout = _device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
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

        return todo!();
    } 
}
