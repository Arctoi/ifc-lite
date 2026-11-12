# ifc-lite-geometry 28.2.0 with Arctoi's patch

This branch holds only the published `ifc-lite-geometry` 28.2.0 crate (MPL-2.0,
from crates.io, built from LTplus-AG/ifc-lite c748326ee) plus one patch, so
Arctoi can pin it as a small Cargo git dependency. The same patch with history
is on branch `arctoi/gradient-directrix` (commits 19dd0028f and f0381486e).

The patch makes swept-area solids along an IFC 4.3 `IfcGradientCurve`
directrix follow its vertical profile and honour StartParam/EndParam (as a
length or a parameter). Modified files stay under MPL-2.0.
