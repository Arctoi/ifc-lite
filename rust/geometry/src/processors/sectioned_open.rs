// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! Open cross-sections of an IFC4x3 `IfcSectionedSurface`: an
//! `IfcOpenCrossProfileDef` read as an open polyline, and the strips that
//! join two consecutive placed sections (no caps, no closing edge).

use crate::{Error, Mesh, Result};
use ifc_lite_core::{slot, DecodedEntity, EntityDecoder, IfcType};
use nalgebra::{Point2, Point3, Vector3};

/// The break points of an `IfcOpenCrossProfileDef`, in the profile plane
/// (file length units). The profile X axis points left of travel. The
/// points start at `OffsetPoint` (the origin when absent) and each segment
/// runs `Widths[i]` towards -X (left edge to right edge) at `Slopes[i]`,
/// measured horizontally when `HorizontalWidths` is true and along the slope
/// otherwise. This follows IfcOpenShell's horizontal-width reading, which
/// its sampling tests pin; for widths along the slope it uses the same
/// direction and slope sign (IfcOpenShell mirrors that case).
pub(super) fn open_cross_profile(
    profile: &DecodedEntity,
    decoder: &mut EntityDecoder,
) -> Result<Vec<Point2<f64>>> {
    if profile.ifc_type != IfcType::IfcOpenCrossProfileDef {
        return Err(Error::geometry(format!(
            "IfcSectionedSurface cross-section #{} is a {}, not an IfcOpenCrossProfileDef",
            profile.id, profile.ifc_type
        )));
    }
    // CorrectProfileType: an open cross profile is a curve, never an area.
    let curve = profile
        .get(slot::IfcOpenCrossProfileDef::PROFILE_TYPE.index())
        .and_then(|v| v.as_enum())
        .is_some_and(|v| v.eq_ignore_ascii_case("CURVE"));
    if !curve {
        return Err(Error::geometry(format!(
            "IfcOpenCrossProfileDef #{} must have ProfileType .CURVE.",
            profile.id
        )));
    }
    let numbers = |index: usize| -> Option<Vec<f64>> {
        profile.get(index)?.as_list()?.iter().map(|v| v.as_float()).collect()
    };
    let widths = numbers(slot::IfcOpenCrossProfileDef::WIDTHS.index())
        .ok_or_else(|| Error::geometry("IfcOpenCrossProfileDef Widths unreadable".to_string()))?;
    let slopes = numbers(slot::IfcOpenCrossProfileDef::SLOPES.index())
        .ok_or_else(|| Error::geometry("IfcOpenCrossProfileDef Slopes unreadable".to_string()))?;
    if widths.len() != slopes.len() {
        return Err(Error::geometry(format!(
            "IfcOpenCrossProfileDef #{}: {} widths but {} slopes",
            profile.id,
            widths.len(),
            slopes.len()
        )));
    }
    let horizontal = profile
        .get_bool(slot::IfcOpenCrossProfileDef::HORIZONTAL_WIDTHS.index())
        .unwrap_or(true);
    let start = decoder
        .point3_at(profile, slot::IfcOpenCrossProfileDef::OFFSET_POINT.index())?
        .map_or_else(Point2::origin, |[x, y, _]| Point2::new(x, y));
    let to_radians = decoder.plane_angle_to_radians();
    let mut points = Vec::with_capacity(widths.len() + 1);
    points.push(start);
    let mut current = start;
    for (width, slope) in widths.into_iter().zip(slopes) {
        let angle = slope * to_radians;
        let (across, rise) = if horizontal {
            (width, width * angle.tan())
        } else {
            (width * angle.cos(), width * angle.sin())
        };
        // IfcOpenShell: y += w tan(pi - slope), i.e. y -= w tan(slope).
        current = Point2::new(current.x - across, current.y - rise);
        // A horizontal width at a near-vertical slope rises without bound.
        crate::curve_source::admit_f32_range([current.x, current.y])?;
        points.push(current);
    }
    Ok(points)
}

/// Join two placed open sections with one quad per segment. The whole strip
/// faces along `up` (the side the section's profile Y points to), so a road
/// surface faces the sky; the side is chosen once per strip from its summed
/// area normal, so a vertical or overhanging segment (a kerb face) keeps the
/// winding of its neighbours.
pub(super) fn emit_strip(
    mesh: &mut Mesh,
    prev: &[Point3<f64>],
    this: &[Point3<f64>],
    up: Vector3<f64>,
) {
    if prev.len() < 2 || prev.len() != this.len() {
        return;
    }
    let quad_normal = |j: usize| {
        let (a, b, c, d) = (prev[j], prev[j + 1], this[j + 1], this[j]);
        (b - a).cross(&(d - a)) + (c - b).cross(&(d - b))
    };
    let total: Vector3<f64> = (0..prev.len() - 1).map(quad_normal).sum();
    let forward = total.dot(&up) >= 0.0;
    for j in 0..prev.len() - 1 {
        let (a, b, c, d) = (prev[j], prev[j + 1], this[j + 1], this[j]);
        let Some(normal) = quad_normal(j).try_normalize(1e-12) else {
            continue; // Zero-width segment between coincident break points.
        };
        let (normal, quad) = if forward {
            (normal, [a, b, c, d])
        } else {
            (-normal, [a, d, c, b])
        };
        let base = (mesh.positions.len() / 3) as u32;
        for p in quad {
            mesh.add_vertex(p, normal);
        }
        mesh.add_triangle(base, base + 1, base + 2);
        mesh.add_triangle(base, base + 2, base + 3);
    }
}

