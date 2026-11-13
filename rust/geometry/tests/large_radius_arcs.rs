// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! A large-radius arc must stay within the chord tolerance (0.5 mm) of its
//! curve, as 2D trimmed circles already do (ISSUE_129). The other arc
//! sources used radius-independent angular steps, so their chords strayed
//! further from the curve the larger the radius:
//!   - #26, an `IfcIndexedPolyCurve` arc (the IFC4 curved-wall export): 8
//!     steps per 90 degrees, at most 16 per arc, 96 mm off at 20 m;
//!   - #39, an `IfcRevolvedAreaSolid` 10 m from its axis: 15 degrees per
//!     step, 87 mm off;
//!   - #44, an `IfcSweptDiskSolid` along a full 15 m `IfcCircle`: 24 steps,
//!     128 mm off.
//!
//! The largest angular step a chord of radius `r` may take is
//! `2 acos(1 - tol / r)`; each test reads the steps between the mesh's
//! vertices around the arc's centre.

use ifc_lite_core::{EntityDecoder, IfcType};
use ifc_lite_geometry::GeometryRouter;

const FIXTURE: &str = "tests/fixtures/large_radius_arcs.ifc";
const CHORD_TOL_M: f64 = 5.0e-4;

fn positions(id: u32) -> Vec<[f64; 3]> {
    let content = std::fs::read_to_string(FIXTURE)
        .unwrap_or_else(|e| panic!("failed to read fixture {FIXTURE}: {e}"));
    let entity_index = ifc_lite_core::build_entity_index(&content);
    let mut decoder = EntityDecoder::with_index(&content, entity_index);
    let router = GeometryRouter::with_units(&content, &mut decoder);
    let element = decoder.decode_by_id(id).unwrap_or_else(|e| panic!("decode #{id}: {e}"));
    assert_eq!(element.ifc_type, IfcType::IfcBuildingElementProxy);
    let mesh = router
        .process_element(&element, &mut decoder)
        .unwrap_or_else(|e| panic!("process #{id}: {e}"));
    assert!(!mesh.indices.is_empty(), "#{id} produced no geometry");
    mesh.positions
        .chunks_exact(3)
        .map(|p| [p[0] as f64, p[1] as f64, p[2] as f64])
        .collect()
}

/// The largest gap between the distinct angles in `angles` (radians).
fn largest_step(mut angles: Vec<f64>, closed: bool) -> f64 {
    angles.sort_by(f64::total_cmp);
    angles.dedup_by(|a, b| (*a - *b).abs() < 1e-6);
    assert!(angles.len() > 2, "too few stations: {angles:?}");
    let mut step = angles.windows(2).map(|w| w[1] - w[0]).fold(0.0, f64::max);
    if closed {
        step = step.max(angles[0] + std::f64::consts::TAU - angles[angles.len() - 1]);
    }
    step
}

fn allowed_step(radius: f64) -> f64 {
    2.0 * (1.0 - CHORD_TOL_M / radius).acos() * (1.0 + 1e-3)
}

#[test]
fn indexed_poly_curve_wall_arc_keeps_its_chords_on_the_curve() {
    // The outer face of the wall: vertices at 20 m from the arc's centre.
    let angles: Vec<f64> = positions(26)
        .iter()
        .filter(|p| (p[0].hypot(p[1]) - 20.0).abs() < 1e-3)
        .map(|p| p[1].atan2(p[0]))
        .collect();
    let step = largest_step(angles, false);
    assert!(step <= allowed_step(20.0), "{} degrees per step", step.to_degrees());
}

#[test]
fn revolved_beam_far_from_its_axis_keeps_its_chords_on_the_curve() {
    // The beam's outer face: 10.15 m from the +Y axis it revolves about.
    let angles: Vec<f64> = positions(39)
        .iter()
        .filter(|p| (p[0].hypot(p[2]) - 10.15).abs() < 1e-3)
        .map(|p| p[2].atan2(p[0]))
        .collect();
    let step = largest_step(angles, false);
    assert!(step <= allowed_step(10.15), "{} degrees per step", step.to_degrees());
}

#[test]
fn swept_disk_along_a_large_circle_keeps_its_path_on_the_curve() {
    // Every ring stands in a plane through the Z axis, so its vertices share
    // one angle about Z: the distinct angles are the stations on the path.
    let angles: Vec<f64> = positions(44).iter().map(|p| p[1].atan2(p[0])).collect();
    let step = largest_step(angles, true);
    assert!(step <= allowed_step(15.0), "{} degrees per step", step.to_degrees());
}
