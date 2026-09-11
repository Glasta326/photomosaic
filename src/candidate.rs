use std::f32::consts::PI;

use rand::{RngExt, rngs::StdRng};
use wgpu::wgt::error::ErrorType;

use crate::{
    config_parse::Config,
    utils::math_utils::{self, wrap_angle},
};

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Candidate {
    /// The ID of the texture this candidate referrs to in the atlas
    pub texture_id: u32,

    /// The x position of this candidate texture
    pub pos_x: f32,

    /// The y position of this candidate texture
    pub pos_y: f32,

    /// The rotation in radians of this candidate texture. 0.000 means unchanged rotation
    pub rotation: f32,

    /// The scaling effect applied to this candidate texture. 1.000x means unchanged size
    pub scale: f32,

    /// Hue shift value. 0.000 means unchanged color
    pub hue: f32,
}
impl std::fmt::Display for Candidate {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "ID: {}\nX: {}\nY: {}\nRot: {}\nScale: {}\nHuge: {}",
            self.texture_id, self.pos_x, self.pos_y, self.rotation, self.scale, self.hue
        )
    }
}

impl std::fmt::Debug for Candidate {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Candidate")
            .field("texture_id", &self.texture_id)
            .field("pos_x", &self.pos_x)
            .field("pos_y", &self.pos_y)
            .field("rotation", &self.rotation)
            .field("scale", &self.scale)
            .field("hue", &self.hue)
            .finish()
    }
}

impl Candidate {
    pub fn new(id: u32, x: f32, y: f32, rot: f32, scale: f32, hue: f32) -> Self {
        return Candidate {
            texture_id: id,
            pos_x: x,
            pos_y: y,
            rotation: rot,
            scale,
            hue,
        };
    }

    pub fn mutate_new(&self, cfg: &Config, rng: &mut StdRng, strength: f32) -> Self {
        // Generate a random position inside the canvas,
        // then, lerp between our current position x,y to the new position x,y individiually to get the final position
        let random_pos = math_utils::random_point_2d(cfg.extra_data.target_dimensions, rng);
        let new_pos = (
            math_utils::lerp(
                self.pos_x as f32,
                random_pos.0 as f32,
                strength,
            ),
            math_utils::lerp(
                self.pos_y as f32,
                random_pos.1 as f32,
                strength,
            ),
        );

        // For angle, we want to either rotate left or right, and an amount decided by a range, with the limit on that range being affected by mutation strength
        // so for example, if we are at angle 0.0, we randomly choose to rotate clockwise, and mutation strength is 0.2, so we pick an amount to rotate
        // between 0 and +PI * 0.2
        let random_off = math_utils::coinflip(rng)
            * rng.random_range(0.0..PI as f32 * strength) as f32;
        let new_ang = math_utils::wrap_angle(self.rotation + random_off);

        // Scale is slightly different due to being a boundless quantity
        // Instead of being based on any absolute limits like pi and the size of the canvas, we instead make it so the scale is a relative multiplier to the parent's scale

        // mutation strength 0.0 means 2.0 mult, and strength 1.0 means 1.0 mult
        let mult = 2.0 - (1.0 - strength);
        let mut new_scale = self.scale;
        if rng.random_bool(0.5) {
            new_scale *= mult;
        } else {
            new_scale /= mult;
        }

        let mut new_hue = 0.0;
        if cfg.enable_hue {
            // Hue rotation value
            // Again, like in rotation, it's a relative offset to the parent's value
            let random_off = math_utils::coinflip(rng)
                * rng.random_range(0.0..PI as f32 * strength) as f32;
            let new_hue_angle = math_utils::wrap_angle(self.hue.to_radians() + random_off); // Normalise into 0-2pi range
            new_hue = new_hue_angle.to_degrees(); // Hue is in degrees
        }
        
        return Candidate::new(
            self.texture_id,
            new_pos.0,
            new_pos.1,
            new_ang,
            new_scale,
            new_hue,
        );
    }

    /// Generates a fully randomised Candidate
    pub fn random(cfg: &Config, rng: &mut StdRng) -> Self {
        return Candidate {
            texture_id: rng.random_range(0..cfg.extra_data.atlas_entry_count),
            pos_x: rng.random_range(0.0..=cfg.extra_data.target_dimensions.0 as f32),
            pos_y: rng.random_range(0.0..=cfg.extra_data.target_dimensions.1 as f32), // These don't need to be downscaled because the "target" image is already pre-downscaled by the reader
            rotation: rng.random_range(-PI..PI),
            scale: rng.random_range(0.5..=2.0) / cfg.downscale_factor,
            hue: {
                if cfg.enable_hue {
                    rng.random_range(-180.0..180.0)
                }
                else {
                    0.0
                }
            },
        };
    }
}

#[cfg(test)]
mod tests {
    use std::f32::consts::PI;

    use super::*;
    use rand::SeedableRng;

    #[test]
    fn candidate_mutation_boundary() {
        let mut cfg = Config::default();
        cfg.extra_data.atlas_dimensions = (500, 500);
        cfg.extra_data.target_dimensions = (500, 500);
        let mut rng = StdRng::seed_from_u64(cfg.seed);
        let mut c = Candidate::new(0, 0.0, 0.0, 0.0, 0.0, 0.0);

        // Hypothetically if you set this high enough it will eventually fail due to chance
        // a value of ~ 1000 means that if it were to be overly biased, lets say it doubles 75% of the time
        // it would reach a value with 225 digits, so if it's still not f32::inf by then, that means it's good enough
        for _i in 0..1000 {
            c = c.mutate_new(&cfg, &mut rng, cfg.mutation_strength);
        }

        assert!(
            (c.pos_x, c.pos_y) <= (500.0, 500.0),
            "Candidate position drifted outside of canvas bounds!\nCandidate pos: [x: {}, y: {}]",
            c.pos_x,
            c.pos_y
        );

        assert!(
            c.rotation > -PI && c.rotation < PI,
            "Candidate rotation exceeded ±PI\nCandidate rot: [{}]",
            c.rotation
        );

        assert!(
            c.scale > f32::MIN && c.scale < f32::MAX,
            "Candidate scale diverged to zero or infinity!\nCandidate scale: [{}]",
            c.scale
        );

        assert!(
            c.hue > -180.0 && c.hue < 180.0,
            "Candidate hue outside 0.0 - 360 range!\nCandidate hue: [{}]",
            c.hue
        );
    }
}
