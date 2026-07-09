use image::RgbaImage;
use wgpu::{TexelCopyBufferLayout, TextureUsages, util::DeviceExt};

use crate::{
    config_parse::Config,
    data_reader::AtlasEntry,
    profiling::{Dropwatch, stopwatch},
};

pub struct GpuContext {
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
    pub buffers: Buffers,
}

/// Long-lived read-only buffers used as a shared resource among most shaders
/// Texture atlas, atlas entries, ect
pub struct Buffers {
    /// This is the texture containing all of the images the user wants to be in the pallette.
    pub input_atlas_texture: wgpu::Texture,

    /// Reference data for the atlas texture, for example a shader may want the image [0],
    /// index 0 contains an X,Y location, along with width and height and other metadata
    pub input_atlas_entry_buffer: wgpu::Buffer,

    /// This is the image the user wants to re-create from the pallette.
    pub input_target_texture: wgpu::Texture,

    /// This is the downscaled canvas that is drawn to over time.
    /// Changes after every cycle
    pub input_canvas_texture: wgpu::Texture,

    /// This is the full-scale canvas that is saved and outputted at the end of operations
    pub output_canvas_texture: wgpu::Texture,
}

impl GpuContext {
    pub fn init(
        cfg: &Config,
        atlas_texture: RgbaImage,
        atlas_entries: Vec<AtlasEntry>,
        target_texture: RgbaImage,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let _d = Dropwatch::new("GpuContext init");

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

        let (device, queue) =
            pollster::block_on(_adapter.request_device(&wgpu::DeviceDescriptor::default()))?;

        let buffers = Buffers::new(
            cfg,
            &device,
            &queue,
            atlas_texture,
            atlas_entries,
            target_texture,
        );

        return Ok(GpuContext {
            device,
            queue,
            buffers,
        });
    }
}

impl Buffers {
    fn new(
        cfg: &Config,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        atlas_texture: RgbaImage,
        atlas_entries: Vec<AtlasEntry>,
        target_texture: RgbaImage,
    ) -> Self {
        let _d = Dropwatch::new("GpuContext buffer creation");

        let atlas_texture_size = wgpu::Extent3d {
            width: atlas_texture.width(),
            height: atlas_texture.height(),
            depth_or_array_layers: 1,
        };

        let target_texture_size = wgpu::Extent3d {
            width: target_texture.width(),
            height: target_texture.height(),
            depth_or_array_layers: 1,
        };

        // The output canvas is scaled up by the downscale factor
        let output_canvas_texture_size = wgpu::Extent3d {
            width: (target_texture.width() as f32 * cfg.downscale_factor) as u32,
            height: (target_texture.height() as f32 * cfg.downscale_factor) as u32,
            depth_or_array_layers: 1,
        };

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
            size: target_texture_size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: TextureUsages::TEXTURE_BINDING | TextureUsages::COPY_DST, // Ditto
            view_formats: &[],
        });

        let input_canvas_texture_buffer = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Score shader: Input canvas texture buffer"),
            size: target_texture_size, // Same size as target texture
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: TextureUsages::TEXTURE_BINDING
                | TextureUsages::STORAGE_BINDING
                | TextureUsages::COPY_DST
                | TextureUsages::COPY_SRC,
            view_formats: &[],
        });

        let output_canvas_texture_buffer = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Score shader: Output canvas texture buffer"),
            size: output_canvas_texture_size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: TextureUsages::TEXTURE_BINDING
                | TextureUsages::STORAGE_BINDING
                | TextureUsages::COPY_DST
                | TextureUsages::COPY_SRC,
            view_formats: &[],
        });

        // Upload both static textures
        // NOTE: atlas entries are already preloaded inside buffer as we use device.create_buffer.init()
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
            target_texture_size,
        );

        // Initalise canvas texture with a blank image
        let input_canvas_texture = RgbaImage::new(
            cfg.extra_data.target_dimensions.0,
            cfg.extra_data.target_dimensions.1,
        );
        queue.write_texture(
            input_canvas_texture_buffer.as_image_copy(),
            bytemuck::cast_slice(input_canvas_texture.as_raw()),
            TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(4 * input_canvas_texture.width()),
                rows_per_image: None,
            },
            target_texture_size,
        );

        // Output canvas needs to have scaled-up size
        let output_canvas_texture = RgbaImage::new(
            (cfg.extra_data.target_dimensions.0 as f32 * cfg.downscale_factor) as u32,
            (cfg.extra_data.target_dimensions.1 as f32 * cfg.downscale_factor) as u32,
        );
        queue.write_texture(
            output_canvas_texture_buffer.as_image_copy(),
            bytemuck::cast_slice(output_canvas_texture.as_raw()),
            TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(4 * output_canvas_texture.width()),
                rows_per_image: None,
            },
            target_texture_size,
        );

        return Buffers {
            input_atlas_texture: atlas_texture_buffer,
            input_atlas_entry_buffer: atlas_entry_buffer,
            input_target_texture: target_texture_buffer,
            input_canvas_texture: input_canvas_texture_buffer,
            output_canvas_texture: output_canvas_texture_buffer,
        };
    }
}
