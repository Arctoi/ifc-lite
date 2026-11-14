// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! An IFC4x3 `IfcSectionedSurface` lofts open `IfcOpenCrossProfileDef`
//! sections along its directrix into a surface. Before, it (and the open
//! profile) drew nothing. Fixture: a straight plan on a 1 % grade from 100 m;
//! a 6 m section (break points at 3, 0 and -3 m left of travel) at station
//! 100 widens to 8 m at station 300.

use ifc_lite_core::{EntityDecoder, IfcType};
use ifc_lite_geometry::GeometryRouter;

const FIXTURE: &str = "tests/fixtures/sectioned_surface.ifc";

fn surface() -> (Vec<[f64; 3]>, Vec<[f64; 3]>, usize) {
    let content = std::fs::read_to_string(FIXTURE)
        .unwrap_or_else(|e| panic!("failed to read fixture {FIXTURE}: {e}"));
    let entity_index = ifc_lite_core::build_entity_index(&content);
    let mut decoder = EntityDecoder::with_index(&content, entity_index);
    let router = GeometryRouter::with_units(&content, &mut decoder);
    let element = decoder.decode_by_id(51).unwrap();
    assert_eq!(element.ifc_type, IfcType::IfcBuildingElementProxy);
    let mesh = router.process_element(&element, &mut decoder).expect("process #51");
    let points = mesh.positions.chunks_exact(3).map(|p| [p[0] as f64, p[1] as f64, p[2] as f64]);
    let normals = mesh.normals.chunks_exact(3).map(|n| [n[0] as f64, n[1] as f64, n[2] as f64]);
    (points.collect(), normals.collect(), mesh.indices.len() / 3)
}

#[test]
fn sectioned_surface_lofts_its_open_sections_along_the_profile() {
    let (points, _, triangles) = surface();
    assert!(triangles > 0, "the surface drew nothing");
    for p in &points {
        // On the grade, and between the two sections in plan.
        assert!((p[2] - (100.0 + 0.01 * p[0])).abs() < 1e-4, "{p:?} off the grade");
        assert!(p[0] > 100.0 - 1e-4 && p[0] < 300.0 + 1e-4, "{p:?} outside the stations");
        // The width grows linearly from 3 to 4 m either side of the axis.
        let half = 3.0 + (p[0] - 100.0) / 200.0;
        assert!(p[1].abs() <= half + 1e-4, "{p:?} wider than the sections");
    }
    let edge = |x: f64, y: f64| points.iter().any(|p| (p[0] - x).abs() < 1e-4 && (p[1] - y).abs() < 1e-4);
    // Left (+Y), axis and right edges at both stations.
    for (x, half) in [(100.0, 3.0), (300.0, 4.0)] {
        for y in [half, 0.0, -half] {
            assert!(edge(x, y), "missing break point ({x}, {y})");
        }
    }
}

#[test]
fn sectioned_surface_faces_up() {
    let (_, normals, _) = surface();
    assert!(!normals.is_empty());
    assert!(normals.iter().all(|n| n[2] > 0.99), "a road surface must face the sky");
}

