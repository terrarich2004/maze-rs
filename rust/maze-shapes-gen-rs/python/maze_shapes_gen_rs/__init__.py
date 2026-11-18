"""Native Rust bindings for circular and hexagonal DFS maze generation."""

from . import _native

circular_dfs_maze = _native.circular_dfs_maze
hexagonal_dfs_maze = _native.hexagonal_dfs_maze
generate_shape_packed = _native.generate_shape_packed
GenerateShapePackedBatch = _native.GenerateShapePackedBatch

__all__ = [
    "circular_dfs_maze",
    "hexagonal_dfs_maze",
    "generate_shape_packed",
    "GenerateShapePackedBatch",
]
