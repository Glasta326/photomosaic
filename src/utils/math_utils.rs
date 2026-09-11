use std::f32::consts::{PI, TAU};

use rand::{RngExt, rngs::StdRng};

/// Remaps a value linearly from one range into another range
/// Panics if value results in NaN, infinity, or similar
pub fn remap(value: f32, start1: f32, end1: f32, start2: f32, end2: f32) -> f32 {
    let out_value = start2 + (end2 - start2) * ((value - start1) / (end1 - start1));
    if out_value.is_nan() || out_value.is_infinite() {
        panic!(
            "Failed to remap {} from [{}] - [{}], into [{}] - [{}]",
            value, start1, end1, start2, end2
        );
    }
    return out_value.clamp(start2, end2);
}

/// Linearly interpolates between two values
pub fn lerp(value1: f32, value2: f32, amount: f32) -> f32 {
    return value1 + (value2 - value1) * amount;
}

/// Returns a random point on a given x,y sized plane inclusively
pub fn random_point_2d(plane_size: (u32, u32), rng: &mut StdRng) -> (u32, u32) {
    // Fun fact, [..=] is the "range inclusive" operator, so doing 0..=10 means from 0 to 10 inclusive
    let x = rng.random_range(0..=plane_size.0);
    let y = rng.random_range(0..=plane_size.1);
    return (x, y);
}

/// Returns either 1.0 or -1.0 randomly
pub fn coinflip(rng: &mut StdRng) -> f32 {
    if rng.random_bool(0.5) {
        return -1.0;
    } else {
        return 1.0;
    }
}

/// Reduces a given angle to a value between -pi and pi
pub fn wrap_angle(mut angle: f32) -> f32 {
    angle = (angle + PI) % TAU;
    if angle < 0.0 {
        angle += TAU;
    }
    angle - PI
}

pub fn angle_lerp(value1: f32, value2: f32, amount: f32) -> f32 {
    let delta = wrap_angle(value2 - value1);
    return wrap_angle(value1 + delta * amount);
}

pub fn average<T>(values: &[T]) -> T
where
    T: Copy + Default + std::ops::Add<Output = T> + std::ops::Div<Output = T> + From<f32>,
{
    let mut sum = T::default();

    for &value in values {
        sum = sum + value;
    }

    sum / T::from(values.len() as f32)
}
