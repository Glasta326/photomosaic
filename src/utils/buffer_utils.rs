/// Calculates the size to create a wgpu::Buffer with correct padding
pub fn get_padded_buffer_size<T>(size: usize) -> u64 {
    // Modified from .create_buffer_init()'s logic
    let align_mask = wgpu::COPY_BUFFER_ALIGNMENT - 1;
    let padded_candidate_buffer_size = ((((size * std::mem::size_of::<T>()) as u64) + align_mask)
        & !align_mask)
        .max(wgpu::COPY_BUFFER_ALIGNMENT);

    return padded_candidate_buffer_size;
}
