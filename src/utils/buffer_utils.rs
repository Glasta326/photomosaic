use crate::{config_parse::Config, gpu::GpuContext};

/// Calculates the size to create a wgpu::Buffer with correct padding
pub fn get_padded_buffer_size<T>(size: usize) -> u64 {
    // Modified from .create_buffer_init()'s logic
    let align_mask = wgpu::COPY_BUFFER_ALIGNMENT - 1;
    let padded_candidate_buffer_size = ((((size * std::mem::size_of::<T>()) as u64) + align_mask)
        & !align_mask)
        .max(wgpu::COPY_BUFFER_ALIGNMENT);

    return padded_candidate_buffer_size;
}

/// Width and height are the texture width and height
/// Workgroup width and height are from @compute @workgroup_size(X,Y) in WGSL
pub fn compute_texture_work_group_count(
    (width, height): (u32, u32),
    (workgroup_width, workgroup_height): (u32, u32),
) -> (u32, u32) {
    let x = (width + workgroup_width - 1) / workgroup_width;
    let y = (height + workgroup_height - 1) / workgroup_height;

    return (x, y);
}

/// Converts a wgpu::Texture into an RgbaImage on the cpu
/// Primarily for debug or non-performance critical purposes
pub fn texture_to_image(cfg: &Config, context: &GpuContext, texture: &wgpu::Texture) -> Result<image::RgbaImage, Box<dyn std::error::Error>>{
    let width = texture.width();
    let height = texture.height();

    let bytes_per_pixel = 4;
    let unpadded_bytes_per_row = width * bytes_per_pixel;

    // wgpu requires rows to be aligned to 256 bytes
    let padded_bytes_per_row = (unpadded_bytes_per_row + 255) & !255;
    let readback_buffer = context.device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("Texture_to_image utility: readback buffer"),
        size: (padded_bytes_per_row * height) as u64,
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });
    let mut encoder = context
        .device
        .create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Texture_to_image utility: Encoder"),
        });

    encoder.copy_texture_to_buffer(
        wgpu::TexelCopyTextureInfo {
            texture: &texture,
            mip_level: 0,
            origin: wgpu::Origin3d::ZERO,
            aspect: wgpu::TextureAspect::All,
        },
        wgpu::TexelCopyBufferInfo {
            buffer: &readback_buffer,
            layout: wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(padded_bytes_per_row),
                rows_per_image: Some(height),
            },
        },
        wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
    );

    context.queue.submit(Some(encoder.finish()));

    // Wait for GPU work to finish
    let slice = readback_buffer.slice(..);
    slice.map_async(wgpu::MapMode::Read, |_| {});
    context.device.poll(wgpu::PollType::wait_indefinitely())?;
    let data = slice.get_mapped_range();
    // Remove row padding
    let mut pixels = Vec::with_capacity((width * height * 4) as usize);
    for row in data.chunks(padded_bytes_per_row as usize) {
        pixels.extend_from_slice(&row[..unpadded_bytes_per_row as usize]);
    }
    drop(data);
    readback_buffer.unmap();
    
    let image = image::RgbaImage::from_raw(width, height, pixels)
        .ok_or("Texture_to_image utility: Failed to create image")?;

    return Ok(image);
}
