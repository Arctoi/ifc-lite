// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! A large-radius arc must stay within the chord tolerance (0.5 mm) of its
//! curve, as 2D trimmed circles already do (ISSUE_129). The other arc
//! sources used radius-independent angular steps, so their chords strayed
//! further from the curve the larger the radius:
//!   - `IfcIndexedPolyCurve` arcs (the IFC4 curved-wall export): 8 steps per
//!     90 degrees, at most 16 per arc, 96 mm off at 20 m (#26, #56, and the
//!     3D path of #85);
//!   - `IfcRevolvedAreaSolid` 10 m from its axis: 15 degrees per step,
//!     87 mm off (#39 a quarter turn, #73 a full one);
//!   - an `IfcSweptDiskSolid` along a full 15 m `IfcCircle`: 24 steps,
//!     128 mm off (#44).
//!
//! A chord of radius `r` may span at most `2 acos(1 - tol / r)`. Each test
//! reads the mesh's own edges around the arc's centre (so dense vertices
//! under coarse faces cannot pass) and the arc's span (so a dense fragment
//! of the arc cannot pass either). #66 checks that a small arc keeps the
//! count it always had.

use ifc_lite_core::{EntityDecoder, IfcType};
use ifc_lite_geometry::GeometryRouter;
use std::f64::consts::TAU;

const FIXTURE: &str = "tests/fixtures/large_radius_arcs.ifc";
const CHORD_TOL_M: f64 = 5.0e-4;

/// Vertex positions and triangles of element `id`.
fn mesh(id: u32) -> (Vec<[f64; 3]>, Vec<[usize; 3]>) {
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
    let positions = mesh
        .positions
        .chunks_exact(3)
        .map(|p| [p[0] as f64, p[1] as f64, p[2] as f64])
        .collect();
    let triangles = mesh
        .indices
        .chunks_exact(3)
        .map(|t| [t[0] as usize, t[1] as usize, t[2] as usize])
        .collect();
    (positions, triangles)
}

/// Polar radius and angle of a vertex about the arc's axis, and its height
/// along that axis.
type Polar = fn(&[f64; 3]) -> (f64, f64, f64);

fn about_z(p: &[f64; 3]) -> (f64, f64, f64) {
    (p[0].hypot(p[1]), p[1].atan2(p[0]), p[2])
}

fn about_y(p: &[f64; 3]) -> (f64, f64, f64) {
    (p[0].hypot(p[2]), p[2].atan2(p[0]), p[1])
}

fn angular(a: f64, b: f64) -> f64 {
    let d = (a - b).abs();
    d.min(TAU - d)
}

/// Span of the arc covered by `angles` (radians): everything but the widest
/// circular gap between them.
fn span(mut angles: Vec<f64>) -> f64 {
    angles.sort_by(f64::total_cmp);
    let outside = angles
        .windows(2)
        .map(|w| w[1] - w[0])
        .fold(angles[0] + TAU - angles[angles.len() - 1], f64::max);
    TAU - outside
}

/// (largest chord step, arc span) of the curved face at `radius` of a solid:
/// the triangles lying on that face whose vertices are not all at one
/// height, and their edges between two vertices at the same height. Flat
/// caps (all at one height) may join distant arc vertices inside the cap;
/// those are not chords of the arc.
fn face_chords(id: u32, polar: Polar, radius: f64) -> (f64, f64) {
    let (positions, triangles) = mesh(id);
    let on_face = |i: usize| (polar(&positions[i]).0 - radius).abs() < 1e-3;
    let (mut longest, mut angles) = (0.0_f64, Vec::new());
    for t in triangles.iter().filter(|t| t.iter().all(|&i| on_face(i))) {
        let p = t.map(|i| polar(&positions[i]));
        if p.iter().all(|v| (v.2 - p[0].2).abs() < 1e-6) {
            continue;
        }
        for (a, b) in [(0, 1), (1, 2), (2, 0)] {
            if (p[a].2 - p[b].2).abs() < 1e-6 {
                longest = longest.max(angular(p[a].1, p[b].1));
            }
        }
        angles.extend(p.iter().map(|v| v.1));
    }
    assert!(angles.len() > 2, "#{id}: no curved face at {radius}");
    (longest, span(angles))
}

/// (largest step, arc span) between the stations of a swept disk whose path
/// lies in the XY plane: every ring stands in a plane through Z, so its
/// vertices cluster at the station's angle (the end rings of a path tilt by
/// half a step, so the cluster's mean is the station).
fn path_stations(id: u32, closed: bool) -> (f64, f64) {
    let (positions, _) = mesh(id);
    let mut angles: Vec<f64> = positions
        .iter()
        .filter(|p| p[0].hypot(p[1]) > 1e-6)
        .map(|p| p[1].atan2(p[0]))
        .collect();
    angles.sort_by(f64::total_cmp);
    let mut stations: Vec<Vec<f64>> = Vec::new();
    for a in angles {
        match stations.last_mut() {
            Some(cluster) if a - cluster[cluster.len() - 1] < 1e-4 => cluster.push(a),
            _ => stations.push(vec![a]),
        }
    }
    let mut stations: Vec<f64> =
        stations.iter().map(|c| c.iter().sum::<f64>() / c.len() as f64).collect();
    // The clusters either side of +-180 degrees are one station.
    if stations.len() > 1 && stations[0] + TAU - stations[stations.len() - 1] < 1e-4 {
        stations.pop();
    }
    assert!(stations.len() > 2, "#{id}: too few stations: {stations:?}");
    let mut gaps: Vec<f64> = stations.windows(2).map(|w| w[1] - w[0]).collect();
    gaps.push(stations[0] + TAU - stations[stations.len() - 1]);
    gaps.sort_by(f64::total_cmp);
    // An open path skips the part of the circle outside it: its widest gap.
    if !closed {
        gaps.pop();
    }
    (gaps[gaps.len() - 1], span(stations))
}

fn allowed_step(radius: f64) -> f64 {
    2.0 * (1.0 - CHORD_TOL_M / radius).acos() * (1.0 + 1e-3)
}

fn check(id: u32, (longest, covered): (f64, f64), radius: f64, span_degrees: f64) {
    assert!(
        longest <= allowed_step(radius),
        "#{id}: a chord spans {} degrees, over {}",
        longest.to_degrees(),
        allowed_step(radius).to_degrees()
    );
    // A closed arc covers everything but one step.
    let expected = if span_degrees >= 360.0 { TAU - longest } else { span_degrees.to_radians() };
    assert!(
        (covered - expected).abs() < 1e-3,
        "#{id}: covers {} degrees, expected {}",
        covered.to_degrees(),
        expected.to_degrees()
    );
}

#[test]
fn indexed_poly_curve_wall_arc_keeps_its_chords_on_the_curve() {
    check(26, face_chords(26, about_z, 20.0), 20.0, 90.0);
}

/// An `IfcArcIndex` middle point need not be halfway along the arc: here it
/// sits at 100 degrees on an arc from 0 to 90 the long way round, a 270
/// degree sweep that an estimate from the middle point's offset would read
/// as about 200 degrees.
#[test]
fn indexed_poly_curve_major_arc_is_measured_by_its_real_sweep() {
    check(56, face_chords(56, about_z, 20.0), 20.0, 270.0);
}

/// The same arc as a 3D curve: the path of a swept disk.
#[test]
fn indexed_3d_major_arc_keeps_its_chords_on_the_curve() {
    check(85, path_stations(85, false), 20.0, 270.0);
}

#[test]
fn revolved_beam_far_from_its_axis_keeps_its_chords_on_the_curve() {
    // The beam's outer face is 10.15 m from the +Y axis it revolves about.
    check(39, face_chords(39, about_y, 10.15), 10.15, 90.0);
}

#[test]
fn full_revolution_far_from_its_axis_keeps_its_chords_on_the_curve() {
    check(73, face_chords(73, about_y, 10.15), 10.15, 360.0);
}

#[test]
fn swept_disk_along_a_large_circle_keeps_its_path_on_the_curve() {
    check(44, path_stations(44, true), 15.0, 360.0);
}

/// An 80 mm arc is already within tolerance at the angular floor (8 steps
/// per 90 degrees leave 0.39 mm), so it keeps exactly that: 9 stations on its
/// outer face. (From about 0.1 m the floor leaves more than 0.5 mm.)
#[test]
fn small_arc_keeps_its_angular_floor() {
    let (positions, _) = mesh(66);
    let mut angles: Vec<f64> = positions
        .iter()
        .filter(|p| (p[0].hypot(p[1]) - 0.08).abs() < 1e-6)
        .map(|p| p[1].atan2(p[0]))
        .collect();
    angles.sort_by(f64::total_cmp);
    angles.dedup_by(|a, b| (*a - *b).abs() < 1e-6);
    assert_eq!(angles.len(), 9, "{angles:?}");
}
