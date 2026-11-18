# saf-dll

A Rust reader of SAF, the open Structural Analysis Format (`.xlsx`), for Open BIM Suite and
Open 3D Viewer Studio. It reads from bytes in memory, so it works in the browser
(WebAssembly) as well as on the desktop.

| Crate | What it holds |
|---|---|
| `saf-read` | The reader: the sheets of a SAF workbook into the structural analysis model of `fem-model` (repository `fem-dll`), in SI units. |

The neutral model is the one of `fem-model`, so an analysis model looks the same whichever
format it came from.

## What is read

| Sheet | What |
|---|---|
| `StructuralMaterial` | Unit mass, E and G modulus, Poisson coefficient, thermal expansion. |
| `StructuralCrossSection` | Parametric shapes (rectangle, circle, pipe, I, T, L, U, box, tube, angle, channel, rolled I) by their parameters, manufactured profiles by their name, area and moments. |
| `StructuralPointConnection` | The nodes. |
| `StructuralCurveMember` | Members through their nodes (a member per segment; arcs drawn straight), with cross-section, rotation and behaviour. |
| `StructuralSurfaceMember` | Plates, walls and shells with their thickness and material. |
| `StructuralPointSupport`, `RelConnectsStructuralMember` | Supports in nodes (rigid, free, springs) and hinges at member ends. |
| `StructuralLoadGroup`, `StructuralLoadCase`, `StructuralLoadCombination` | Load cases with their kind and self weight, combinations with their factors. |
| `StructuralPointAction`, `StructuralPointMoment`, `StructuralCurveAction`, `StructuralSurfaceAction` | Forces and moments in nodes, point and line loads on members, area loads on 2D members. |

The unit of every column is read from its header (`Coordinate X [m]`, `Value [kN/m2]`), so
metric and imperial workbooks both read. Sheets that are not read (curve and surface supports,
openings, ribs, regions, results) are counted in the notes.

The format is described at [saf.guide](https://saf.guide). The tests use workbooks made in the
tests themselves.

## Use

```rust
let data = std::fs::read("model.xlsx")?;
let doc = saf_read::read("model.xlsx", &data)?;
let analysis = doc.analysis.unwrap();
```

## Licence

LGPL-3.0-or-later.
