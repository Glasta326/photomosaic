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
