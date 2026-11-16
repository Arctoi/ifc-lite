// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! An IFC4x3 `IfcSectionedSurface` lofts open `IfcOpenCrossProfileDef`
//! sections along its directrix into a surface. Before, it (and the open
//! profile) drew nothing.
//!
//! `sectioned_surface.ifc`: a straight plan on a 1 % grade from 100 m; a 6 m
//! section (break points 3, 0 and -3 m left of travel) at station 100 widens
//! to 8 m at station 300.
//!
//! `sectioned_surface_crossfall.ifc` (angles in degrees, level at 50 m):
//! - #45: 2 m left of the axis falling 2.5 %, 5 m right of it rising 4 %
//!   (horizontal widths), so the left edge, axis and right edge sit at 50,
//!   49.95 and 50.15 m;
//! - #54: the same with widths along the slope;
//! - #66: a middle lane that widens from 0 to 1 m between two 3 m lanes.
//!
//! Both are also read moved to SWEREF 99 TM magnitudes, where an f32 steps
//! 0.5 m in northing.

use ifc_lite_core::{EntityDecoder, IfcType, RtcVerdict};
use ifc_lite_geometry::GeometryRouter;

struct Surface {
    points: Vec<[f64; 3]>,
    normals: Vec<[f64; 3]>,
    triangles: Vec<[usize; 3]>,
}

fn surface(fixture: &str, id: u32) -> Surface {
    let path = format!("tests/fixtures/{fixture}");
    let content = std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {path}: {e}"));
    let entity_index = ifc_lite_core::build_entity_index(&content);
    let mut decoder = EntityDecoder::with_index(&content, entity_index);
    let router = GeometryRouter::with_units(&content, &mut decoder);
    let element = decoder.decode_by_id(id).unwrap();
    assert_eq!(element.ifc_type, IfcType::IfcBuildingElementProxy);
    let mesh = router
        .process_element(&element, &mut decoder)
        .unwrap_or_else(|e| panic!("#{id}: {e}"));
    let triple = |c: &[f32]| [c[0] as f64, c[1] as f64, c[2] as f64];
    let surface = Surface {
        points: mesh.positions.chunks_exact(3).map(triple).collect(),
        normals: mesh.normals.chunks_exact(3).map(triple).collect(),
        triangles: mesh
            .indices
            .chunks_exact(3)
            .map(|t| [t[0] as usize, t[1] as usize, t[2] as usize])
            .collect(),
    };
    assert!(!surface.triangles.is_empty(), "#{id} drew nothing");
    surface
}

fn sub(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}

fn cross(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [a[1] * b[2] - a[2] * b[1], a[2] * b[0] - a[0] * b[2], a[0] * b[1] - a[1] * b[0]]
}

fn has_point(surface: &Surface, x: f64, y: f64, z: f64) -> bool {
    surface
        .points
        .iter()
        .any(|p| (p[0] - x).abs() < 1e-4 && (p[1] - y).abs() < 1e-4 && (p[2] - z).abs() < 1e-4)
}

#[test]
fn sectioned_surface_lofts_its_open_sections_along_the_profile() {
    let surface = surface("sectioned_surface.ifc", 51);
    for p in &surface.points {
        // On the grade, and between the two sections in plan.
        assert!((p[2] - (100.0 + 0.01 * p[0])).abs() < 1e-4, "{p:?} off the grade");
        assert!(p[0] > 100.0 - 1e-4 && p[0] < 300.0 + 1e-4, "{p:?} outside the stations");
        // The width grows linearly from 3 to 4 m either side of the axis.
        let half = 3.0 + (p[0] - 100.0) / 200.0;
        assert!(p[1].abs() <= half + 1e-4, "{p:?} wider than the sections");
    }
    // Left (+Y), axis and right edges at both stations.
    for (x, half) in [(100.0_f64, 3.0), (300.0, 4.0)] {
        for y in [half, 0.0, -half] {
            assert!(has_point(&surface, x, y, 100.0 + 0.01 * x), "missing break point ({x}, {y})");
        }
    }
}

/// The left edge is the profile's +X: 2 m to the left (+Y for travel along
/// +X) falling 2.5 % to the axis, then 5 m to the right rising 4 %.
#[test]
fn crossfall_sections_put_each_edge_at_its_height_and_side() {
    for id in [45, 54] {
        let surface = surface("sectioned_surface_crossfall.ifc", id);
        for x in [0.0, 100.0] {
            assert!(has_point(&surface, x, 2.0, 50.0), "#{id}: left edge at {x}");
            assert!(has_point(&surface, x, 0.0, 49.95), "#{id}: axis at {x}");
            assert!(has_point(&surface, x, -5.0, 50.15), "#{id}: right edge at {x}");
        }
    }
}

/// Every triangle is wound with its stored normal, and the surface faces the
/// sky.
#[test]
fn sectioned_surface_winding_matches_its_normals_and_faces_up() {
    for (fixture, id) in [
        ("sectioned_surface.ifc", 51),
        ("sectioned_surface_crossfall.ifc", 45),
        ("sectioned_surface_crossfall.ifc", 54),
    ] {
        let surface = surface(fixture, id);
        for t in &surface.triangles {
            let [a, b, c] = t.map(|i| surface.points[i]);
            let geometric = cross(sub(b, a), sub(c, a));
            let stored = surface.normals[t[0]];
            let along: f64 = (0..3).map(|i| geometric[i] * stored[i]).sum();
            assert!(along > 0.0, "#{id}: triangle {t:?} wound against its normal");
            assert!(geometric[2] > 0.0, "#{id}: triangle {t:?} faces down");
        }
    }
}

/// Break points are paired by position even where a width is zero, so the
/// widening middle lane joins the two sections: the surface covers the whole
/// trapezoid, 6 m wide at station 0 and 7 m at station 100.
#[test]
fn a_lane_widening_from_zero_keeps_the_sections_joined() {
    let surface = surface("sectioned_surface_crossfall.ifc", 66);
    let area: f64 = surface
        .triangles
        .iter()
        .map(|t| {
            let [a, b, c] = t.map(|i| surface.points[i]);
            let n = cross(sub(b, a), sub(c, a));
            0.5 * (n[0] * n[0] + n[1] * n[1] + n[2] * n[2]).sqrt()
        })
        .sum();
    assert!((area - 650.0).abs() < 1e-2, "covers {area} m2 of 650");
    assert!(has_point(&surface, 0.0, -3.0, 50.0) && has_point(&surface, 100.0, -4.0, 50.0));
}

/// Plan position of both fixtures' horizontal alignment, moved from the
/// origin to SWEREF 99 TM magnitudes. The fractions are ones an f32 cannot
/// hold there: it steps 0.5 m at this northing and 1/64 m at this easting.
const SWEREF: [f64; 2] = [150_000.137, 6_600_000.263];

/// `fixture` with its alignment starting at [`SWEREF`] instead of the origin.
/// The directrix carries the coordinates; the element stays at the origin.
fn at_sweref(fixture: &str) -> String {
    let path = format!("tests/fixtures/{fixture}");
    let content = std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {path}: {e}"));
    let start = "#24=IFCAXIS2PLACEMENT2D(#20,#21);";
    assert!(content.contains(start), "{fixture}: horizontal start placement moved");
    content.replace(
        start,
        &format!(
            "#19=IFCCARTESIANPOINT(({:.3},{:.3}));
#24=IFCAXIS2PLACEMENT2D(#19,#21);",
            SWEREF[0], SWEREF[1]
        ),
    )
}

/// Reviewed in Arctoi #1336: a road surface at national-grid coordinates
/// was lowered in absolute coordinates and narrowed to f32 there, so its
/// break points snapped to a 0.5 m grid. The file has no other geometry, so
/// the surface itself must vote the RTC anchor (an `AdvancedSurface` rep
/// abstained), and its rings must be rebased in f64 before narrowing: every
/// vertex then matches the near-origin surface, moved, to the millimetre.
#[test]
fn a_sectioned_surface_at_national_grid_coordinates_keeps_millimetres() {
    for (fixture, id) in [
        ("sectioned_surface.ifc", 51),
        ("sectioned_surface_crossfall.ifc", 45),
        ("sectioned_surface_crossfall.ifc", 54),
        ("sectioned_surface_crossfall.ifc", 66),
    ] {
        let near = surface(fixture, id).points;
        let content = at_sweref(fixture);
        let entity_index = ifc_lite_core::build_entity_index(&content);
        let mut decoder = EntityDecoder::with_index(&content, entity_index);
        let mut router = GeometryRouter::with_units(&content, &mut decoder);
        let anchor = match router.detect_rtc_offset_for_file(content.as_bytes(), &mut decoder) {
            Some(RtcVerdict::Large { anchor }) => anchor,
            other => panic!("#{id} in {fixture}: no large-coordinate anchor, got {other:?}"),
        };
        // The anchor is the directrix start, on the road, not a centroid
        // between the road and the origin placements.
        assert!(
            (anchor.0 - SWEREF[0]).abs() < 1e-6 && (anchor.1 - SWEREF[1]).abs() < 1e-6,
            "#{id} in {fixture}: anchor {anchor:?} is not on the road"
        );
        router.set_rtc_offset(anchor);
        let element = decoder.decode_by_id(id).unwrap();
        let mesh = router
            .process_element(&element, &mut decoder)
            .unwrap_or_else(|e| panic!("#{id} in {fixture}: {e}"));
        let far: Vec<[f64; 3]> = mesh
            .positions
            .chunks_exact(3)
            .map(|c| [0, 1, 2].map(|axis| c[axis] as f64 + mesh.origin[axis]))
            .collect();
        assert_eq!(far.len(), near.len(), "#{id} in {fixture}: vertex count changed");
        let shift = [SWEREF[0] - anchor.0, SWEREF[1] - anchor.1, -anchor.2];
        for (i, (f, n)) in far.iter().zip(&near).enumerate() {
            for axis in 0..3 {
                let error = (f[axis] - (n[axis] + shift[axis])).abs();
                assert!(
                    error < 1e-3,
                    "#{id} in {fixture}: vertex {i} axis {axis} off by {error} m ({f:?} vs {n:?})"
                );
            }
        }
    }
}
