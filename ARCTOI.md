# ifc-lite-geometry 30.0.0 with Arctoi's patches

This branch holds only the published `ifc-lite-geometry` 30.0.0 crate (MPL-2.0,
from crates.io, built from LTplus-AG/ifc-lite v13.0.0, d97f72e06) plus
Arctoi's patches, so Arctoi can pin it as a small Cargo git dependency.
Modified files stay under MPL-2.0.

The patches were carried over from `arctoi/ifc-lite-geometry-28.2.0-patched`
(8988c9f8e) unchanged and without conflicts. Upstream's changes from 28.2.0 to
30.0.0 contain none of them, so none were dropped.

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
   `arctoi/road-surfaces` (commits 7f582965c and c9d2f92a3).
4. Sectioned surfaces and solids at national-grid coordinates (SWEREF 99,
   northings around 6.6e6 m) are rebased in f64 before the f32 mesh cast, and
   vote for the large-coordinate offset with their directrix start; before,
   their vertices snapped to the f32 grid (0.5 m at 6.6e6 m). With history on
   branch `arctoi/sectioned-surface-rtc` (commit 5d2fa1541).
