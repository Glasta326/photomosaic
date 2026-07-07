pub mod context;
pub mod score_shader;
pub mod draw_shader;

// Api cleanliness
pub use context::*;
pub use score_shader::*;
pub use draw_shader::*;


// TODO: Perhaps a single init_shaders() function that initalises all of the shaders at once
// Not the context though, that gets done first manually
