# cu-map-payloads

Standardized map payload types for Copper.

A map is a set of grid layers sharing one `CuGridGeometry`. Each layer is its
own message, so a task receives only the layers it is wired to.

| Type | Cell | Reads as | Never observed |
| --- | --- | --- | --- |
| `CuOccupancyGridMap` | `u8` | `Ratio` | `0` |
| `CuLabelGridMap` | `u16` | `CuSemanticClassId` | `u16::MAX` |
| `CuElevationGridMap` | `Length` | `Length` | NaN |
| `CuCostGridMap` | `f32` | `f32` | NaN |
| `CuFeatureGridMap<_, D>` | `D` × `f32` | `[f32; D]` | NaN in the first element |

`get` and `get_at` return `None` for cells that were never observed, so unmapped
space cannot be read as free space or as class `0`.

`CuSemanticLabelSet<N>` names and colors up to `N` class ids of a label layer.
It is stored inline; send it as a latched state. Implement `CuGridLayer` to
define a layer with another cell representation.

## Geometry

`CuGridGeometry::origin` is the lower-left corner of cell `(0, 0)` in the map
frame. Cells are stored row-major, +X first. The grid is axis-aligned in its
frame.

## Features

- `std` (default): the handle-backed `CuGridMap` layers. Geometry, layer
  descriptions, and the label set are available without it.
- `textlogs`
- `reflect`
