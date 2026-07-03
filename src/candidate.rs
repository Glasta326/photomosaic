#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Candidate {
    /// The ID of the texture this candidate referrs to in the atlas
    pub texture_id: usize,

    /// The x position of this candidate texture
    pub pos_x: u32,

    /// The y position of this candidate texture
    pub pos_y: u32,

    /// The rotation in radians of this candidate texture. 0.000 means unchanged rotation
    pub rotation: f32,

    /// The scaling effect applied to this candidate texture. 1.000x means unchanged size
    pub scale: f32,
}
impl std::fmt::Display for Candidate {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "ID: {}\nX: {}\nY: {}\nRot: {}\nScale: {}",
            self.texture_id, self.pos_x, self.pos_y, self.rotation, self.scale
        )
    }
}
