use std::fmt::Write as _;

use bevy::ecs::{
    message::MessageReader,
    system::Query,
};

use crate::core::{Building, ExportEvent, OpenSpace};

pub fn handle_export(
    mut events: MessageReader<ExportEvent>,
    buildings: Query<&Building>,
    open_spaces: Query<&OpenSpace>,
) {
    for event in events.read() {
        let obj = build_obj(buildings.iter(), open_spaces.iter());

        match std::fs::write(&event.filename, obj) {
            Ok(()) => bevy::log::info!("exported city to {}", event.filename),
            Err(e) => bevy::log::error!("failed to export {}: {e}", event.filename),
        }
    }
}

/// Writes each building as an extruded prism and each open space as a flat
/// polygon, in Wavefront OBJ. OBJ indices are 1-based and global to the file.
fn build_obj<'a>(
    buildings: impl Iterator<Item = &'a Building>,
    open_spaces: impl Iterator<Item = &'a OpenSpace>,
) -> String {
    let mut out = String::from("# procedural-city-gen export\n");
    let mut vertex_base: usize = 1;

    for building in buildings {
        let n = building.footprint.len();
        if n < 3 {
            continue;
        }

        let _ = writeln!(out, "o building_{}", building.id);

        // Bottom ring then top ring.
        for v in &building.footprint {
            let _ = writeln!(out, "v {} 0 {}", v.x, v.y);
        }
        for v in &building.footprint {
            let _ = writeln!(out, "v {} {} {}", v.x, building.height, v.y);
        }

        // Walls, as one quad per edge.
        for i in 0..n {
            let j = (i + 1) % n;
            let b0 = vertex_base + i;
            let b1 = vertex_base + j;
            let t0 = vertex_base + n + i;
            let t1 = vertex_base + n + j;
            let _ = writeln!(out, "f {b0} {b1} {t1} {t0}");
        }

        // Roof as a single n-gon.
        let roof: Vec<String> = (0..n).map(|i| (vertex_base + n + i).to_string()).collect();
        let _ = writeln!(out, "f {}", roof.join(" "));

        vertex_base += n * 2;
    }

    for space in open_spaces {
        let n = space.polygon.len();
        if n < 3 {
            continue;
        }

        let _ = writeln!(out, "o open_space_{}", space.block_id);
        for v in &space.polygon {
            let _ = writeln!(out, "v {} 0 {}", v.x, v.y);
        }

        let face: Vec<String> = (0..n).map(|i| (vertex_base + i).to_string()).collect();
        let _ = writeln!(out, "f {}", face.join(" "));

        vertex_base += n;
    }

    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::{OpenSpaceKind, Polygon};
    use bevy::math::Vec2;

    fn square() -> Polygon {
        vec![
            Vec2::new(0.0, 0.0),
            Vec2::new(2.0, 0.0),
            Vec2::new(2.0, 2.0),
            Vec2::new(0.0, 2.0),
        ]
    }

    #[test]
    fn test_obj_has_both_rings_per_building() {
        let b = Building {
            id: 0,
            footprint: square(),
            height: 5.0,
        };
        let obj = build_obj([&b].into_iter(), std::iter::empty());
        let verts = obj.lines().filter(|l| l.starts_with("v ")).count();
        // 4 bottom + 4 top
        assert_eq!(verts, 8);
        assert!(obj.contains("v 0 5 0"), "missing top ring at wall height");
    }

    #[test]
    fn test_obj_face_indices_stay_in_range() {
        let a = Building {
            id: 0,
            footprint: square(),
            height: 3.0,
        };
        let b = Building {
            id: 1,
            footprint: square(),
            height: 4.0,
        };
        let obj = build_obj([&a, &b].into_iter(), std::iter::empty());

        let vertex_count = obj.lines().filter(|l| l.starts_with("v ")).count();
        for line in obj.lines().filter(|l| l.starts_with("f ")) {
            for tok in line.split_whitespace().skip(1) {
                let idx: usize = tok.parse().expect("face index should parse");
                assert!(idx >= 1, "OBJ indices are 1-based, got {idx}");
                assert!(
                    idx <= vertex_count,
                    "face index {idx} exceeds {vertex_count} vertices"
                );
            }
        }
    }

    #[test]
    fn test_obj_includes_open_space() {
        let s = OpenSpace {
            block_id: 7,
            kind: OpenSpaceKind::Park,
            polygon: square(),
        };
        let obj = build_obj(std::iter::empty(), [&s].into_iter());
        assert!(obj.contains("o open_space_7"));
        assert_eq!(obj.lines().filter(|l| l.starts_with("v ")).count(), 4);
    }

    #[test]
    fn test_obj_skips_degenerate() {
        let b = Building {
            id: 0,
            footprint: vec![Vec2::ZERO, Vec2::ONE],
            height: 3.0,
        };
        let obj = build_obj([&b].into_iter(), std::iter::empty());
        assert_eq!(obj.lines().filter(|l| l.starts_with("v ")).count(), 0);
    }
}
