use crate::kernel::PrimitiveKind;
use glam::{EulerRot, Mat3, Vec3};

/// Local-space (pre-rotation, pre-translation) half-extents of a primitive, centered
/// at the origin — matches how the kernel's `build_shape` constructs each smlibpy
/// primitive before rotating/translating it. Not exact for Cone/Torus (which aren't
/// axis-aligned boxes) but a conservative bound is all this needs for overlap checks
/// and the on-screen "ghost" outline.
fn local_half_extents(kind: PrimitiveKind, params: [f64; 3]) -> Vec3 {
    match kind {
        PrimitiveKind::Box => Vec3::new(
            (params[0] * 0.5) as f32,
            (params[1] * 0.5) as f32,
            (params[2] * 0.5) as f32,
        ),
        PrimitiveKind::Cylinder => {
            let r = params[0] as f32;
            Vec3::new(r, r, (params[1] * 0.5) as f32)
        }
        PrimitiveKind::Sphere => Vec3::splat(params[0] as f32),
        PrimitiveKind::Cone => {
            let r = params[0].max(params[1]) as f32;
            Vec3::new(r, r, (params[2] * 0.5) as f32)
        }
        PrimitiveKind::Torus => {
            let outer = (params[0] + params[1]) as f32;
            Vec3::new(outer, outer, params[1] as f32)
        }
    }
}

/// World-space axis-aligned bounding box `(min, max)` for a primitive, given its kind,
/// dimension params, rotation (degrees, applied X then Y then Z about fixed world axes
/// — matching the kernel's `build_shape`) and position offset.
pub fn world_aabb(
    kind: PrimitiveKind,
    params: [f64; 3],
    rotation_deg: [f64; 3],
    position: [f64; 3],
) -> (Vec3, Vec3) {
    let half = local_half_extents(kind, params);
    let rot = Mat3::from_euler(
        EulerRot::XYZ,
        (rotation_deg[0] as f32).to_radians(),
        (rotation_deg[1] as f32).to_radians(),
        (rotation_deg[2] as f32).to_radians(),
    );
    let pos = Vec3::new(position[0] as f32, position[1] as f32, position[2] as f32);

    let mut min = Vec3::splat(f32::INFINITY);
    let mut max = Vec3::splat(f32::NEG_INFINITY);
    for &sx in &[-1.0_f32, 1.0] {
        for &sy in &[-1.0_f32, 1.0] {
            for &sz in &[-1.0_f32, 1.0] {
                let corner_local = Vec3::new(half.x * sx, half.y * sy, half.z * sz);
                let world = rot * corner_local + pos;
                min = min.min(world);
                max = max.max(world);
            }
        }
    }
    (min, max)
}

/// True if two AABBs overlap (touching counts as overlapping).
pub fn aabb_intersects(a: (Vec3, Vec3), b: (Vec3, Vec3)) -> bool {
    a.0.x <= b.1.x && a.1.x >= b.0.x && a.0.y <= b.1.y && a.1.y >= b.0.y && a.0.z <= b.1.z && a.1.z >= b.0.z
}

/// The 12 edges of an AABB as (start, end) world-space segments, for wireframe drawing.
pub fn aabb_edges((min, max): (Vec3, Vec3)) -> [(Vec3, Vec3); 12] {
    let corners = [
        Vec3::new(min.x, min.y, min.z),
        Vec3::new(max.x, min.y, min.z),
        Vec3::new(max.x, max.y, min.z),
        Vec3::new(min.x, max.y, min.z),
        Vec3::new(min.x, min.y, max.z),
        Vec3::new(max.x, min.y, max.z),
        Vec3::new(max.x, max.y, max.z),
        Vec3::new(min.x, max.y, max.z),
    ];
    [
        (corners[0], corners[1]),
        (corners[1], corners[2]),
        (corners[2], corners[3]),
        (corners[3], corners[0]),
        (corners[4], corners[5]),
        (corners[5], corners[6]),
        (corners[6], corners[7]),
        (corners[7], corners[4]),
        (corners[0], corners[4]),
        (corners[1], corners[5]),
        (corners[2], corners[6]),
        (corners[3], corners[7]),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn box_aabb_matches_unrotated_extents() {
        let (min, max) = world_aabb(PrimitiveKind::Box, [10.0, 20.0, 4.0], [0.0, 0.0, 0.0], [1.0, 2.0, 3.0]);
        assert!((min - Vec3::new(1.0 - 5.0, 2.0 - 10.0, 3.0 - 2.0)).length() < 1e-4);
        assert!((max - Vec3::new(1.0 + 5.0, 2.0 + 10.0, 3.0 + 2.0)).length() < 1e-4);
    }

    #[test]
    fn rotated_cylinder_axis_swap_matches_kernel_convention() {
        // Matches the sketch extrude XZ-plane case verified against the real smlibpy
        // kernel: rotating 90 about X swaps the cylinder's long axis from Z onto Y.
        let (min, max) = world_aabb(PrimitiveKind::Cylinder, [3.0, 10.0, 0.0], [90.0, 0.0, 0.0], [0.0, 0.0, 0.0]);
        let size = max - min;
        assert!((size.y - 10.0).abs() < 1e-3, "expected height on Y, got {size:?}");
        assert!((size.z - 6.0).abs() < 1e-3, "expected diameter on Z, got {size:?}");
    }

    #[test]
    fn disjoint_boxes_do_not_intersect() {
        let a = world_aabb(PrimitiveKind::Box, [10.0, 10.0, 10.0], [0.0, 0.0, 0.0], [0.0, 0.0, 0.0]);
        let b = world_aabb(PrimitiveKind::Box, [10.0, 10.0, 10.0], [0.0, 0.0, 0.0], [100.0, 0.0, 0.0]);
        assert!(!aabb_intersects(a, b));
    }

    #[test]
    fn overlapping_boxes_intersect() {
        let a = world_aabb(PrimitiveKind::Box, [10.0, 10.0, 10.0], [0.0, 0.0, 0.0], [0.0, 0.0, 0.0]);
        let b = world_aabb(PrimitiveKind::Box, [10.0, 10.0, 10.0], [0.0, 0.0, 0.0], [5.0, 0.0, 0.0]);
        assert!(aabb_intersects(a, b));
    }
}
