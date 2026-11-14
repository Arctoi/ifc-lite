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
    let start = match profile.get(slot::IfcOpenCrossProfileDef::OFFSET_POINT.index()) {
        Some(attr) if !attr.is_null() => {
            let [x, y, _] = crate::curve_source::point_from_ref(attr, decoder)?;
            Point2::new(x, y)
        }
        _ => Point2::origin(),
    };
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
        if !(current.x.is_finite() && current.y.is_finite()) {
            return Err(Error::geometry(format!(
                "IfcOpenCrossProfileDef #{} has a non-finite break point",
                profile.id
            )));
        }
        points.push(current);
    }
    Ok(points)
}

/// Join two placed open sections with one quad per segment. The quads face
/// along `up` (the side the section's profile Y points to), so a road
/// surface faces the sky.
pub(super) fn emit_strip(
    mesh: &mut Mesh,
    prev: &[Point3<f64>],
    this: &[Point3<f64>],
    up: Vector3<f64>,
) {
    if prev.len() < 2 || prev.len() != this.len() {
        return;
    }
    for j in 0..prev.len() - 1 {
        let (a, b, c, d) = (prev[j], prev[j + 1], this[j + 1], this[j]);
        let normal = (b - a).cross(&(d - a)) + (c - b).cross(&(d - b));
        let Some(normal) = normal.try_normalize(1e-12) else {
            continue; // Zero-width segment between coincident break points.
        };
        let (normal, quad) = if normal.dot(&up) >= 0.0 {
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

