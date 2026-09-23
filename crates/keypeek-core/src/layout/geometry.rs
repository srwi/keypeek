//! Geometry calculation helpers for rotated key layouts.

pub fn flattened_top_left_after_center_rotation(
    x: f32,
    y: f32,
    w: f32,
    h: f32,
    angle_deg: f32,
    pivot_x: f32,
    pivot_y: f32,
) -> (f32, f32) {
    if angle_deg.abs() <= f32::EPSILON {
        return (x, y);
    }

    let center_x = x + (w * 0.5);
    let center_y = y + (h * 0.5);

    let local_center_x = center_x - pivot_x;
    let local_center_y = center_y - pivot_y;

    let angle = angle_deg.to_radians();
    let cos_a = angle.cos();
    let sin_a = angle.sin();

    let rotated_center_x = (local_center_x * cos_a) - (local_center_y * sin_a) + pivot_x;
    let rotated_center_y = (local_center_x * sin_a) + (local_center_y * cos_a) + pivot_y;

    (rotated_center_x - (w * 0.5), rotated_center_y - (h * 0.5))
}
