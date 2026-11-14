# ifc-lite-geometry 28.2.0 with Arctoi's patches

This branch holds only the published `ifc-lite-geometry` 28.2.0 crate (MPL-2.0,
from crates.io, built from LTplus-AG/ifc-lite c748326ee) plus Arctoi's patches,
so Arctoi can pin it as a small Cargo git dependency. Modified files stay under
MPL-2.0.

1. Swept-area solids along an IFC 4.3 `IfcGradientCurve` directrix follow its
   vertical profile and honour StartParam/EndParam (as a length or a
   parameter). With history on branch `arctoi/gradient-directrix` (commits
   19dd0028f and f0381486e); proposed upstream as LTplus-AG/ifc-lite#9969.
2. Large-radius arcs keep their chords within 0.5 mm of the curve: arcs in
   `IfcIndexedPolyCurve`, revolved solids and full circles used as 3D curves
   took fixed angular steps whatever their radius. With history on branch
   `arctoi/arc-chord-tolerance` (commits c16e1dbe6 and f379806e8).
3. IFC 4.3 `IfcSectionedSurface` (road and rail surfaces) with
   `IfcOpenCrossProfileDef` sections, and the `AdvancedSurface`
   representation type, are drawn. With history on branch
   `arctoi/road-surfaces` (commit 7f582965c).
