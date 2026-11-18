"""Native Rust bindings for rectangular and triangular maze generation."""

from . import _native

kruskal_rect_maze = _native.kruskal_rect_maze
kruskal_rect_edges = _native.kruskal_rect_edges
prims_square_maze = _native.prims_square_maze
kruskal_triangular_maze = _native.kruskal_triangular_maze
dfs_triangular_maze = _native.dfs_triangular_maze
generate_maze_packed = _native.generate_maze_packed
GenerateMazePackedBatch = _native.GenerateMazePackedBatch

__all__ = [
    "kruskal_rect_maze",
    "kruskal_rect_edges",
    "prims_square_maze",
    "kruskal_triangular_maze",
    "dfs_triangular_maze",
    "generate_maze_packed",
    "GenerateMazePackedBatch",
]
