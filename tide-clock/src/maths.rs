pub fn lerp(t: f32, min: i32, max: i32) -> i32 {
    let t01 = clamp(t, 0f32, 1f32);
    let range = (max - min) as f32;

    let value = t01 * range + min as f32;

    value.round() as i32
}

pub fn inverse_lerp(value: f32, min: f32, max: f32) -> f32 {
    assert!(min < max);

    let clamped = clamp(value, min, max);

    (clamped - min) / (max - min)
}

pub fn clamp(value: f32, min: f32, max: f32) -> f32 {
    assert!(min <= max);
    let mut x = value;
    if x < min {
        x = min;
    }
    if x > max {
        x = max;
    }

    x
}
