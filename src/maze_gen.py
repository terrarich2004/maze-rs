"""Seeded rectangular and triangular maze generators (Kruskal, Prim, DFS)."""

import random

MAX_GRID_N = 128
MAX_PRIMS_N = 32

_HEX_DIGITS = "0123456789abcdef"


def _validate_size(n: int, max_n: int) -> None:
    if n <= 0:
        raise ValueError("grid size n must be positive")
    if n > max_n:
        raise ValueError(f"grid size n must be at most {max_n}")


def _dsu_find(parent: list[int], x: int) -> int:
    item = x
    while item != parent[item]:
        item = parent[item]
    root = item
    while x != root:
        nxt = parent[x]
        parent[x] = root
        x = nxt
    return root


def _dsu_union(parent: list[int], size: list[int], a: int, b: int) -> bool:
    root_a = _dsu_find(parent, a)
    root_b = _dsu_find(parent, b)
    if root_a == root_b:
        return False
    if size[root_a] < size[root_b]:
        size[root_b] += size[root_a]
        parent[root_a] = root_b
    else:
        size[root_a] += size[root_b]
        parent[root_b] = root_a
    return True


def _kruskal_rect_core(n: int, seed: int) -> tuple[list[list[int]], list[list[int]]]:
    _validate_size(n, MAX_GRID_N)
    total_cells = n * n

    edges_of_graph: list[tuple[int, int]] = []
    for row in range(n):
        row_base = n * row
        for col in range(n):
            node = row_base + col
            if col < n - 1:
                edges_of_graph.append((node, node + 1))
            if row < n - 1:
                edges_of_graph.append((node, node + n))

    rng = random.Random(seed)
    rng.shuffle(edges_of_graph)

    parent = list(range(total_cells))
    size = [1] * total_cells
    # Direction order matches KruskalRectangular: TOP=0, RIGHT=1, BOTTOM=2, LEFT=3
    spanning_tree = [[0, 0, 0, 0] for _ in range(total_cells)]
    accepted_edges: list[list[int]] = []

    for cell1, cell2 in edges_of_graph:
        if _dsu_union(parent, size, cell1, cell2):
            if cell2 == cell1 + 1:
                spanning_tree[cell1][1] = 1
                spanning_tree[cell2][3] = 1
            else:
                spanning_tree[cell1][2] = 1
                spanning_tree[cell2][0] = 1
            accepted_edges.append([cell1, cell2])

    return spanning_tree, accepted_edges


def kruskal_rect_maze(n: int, seed: int) -> list[list[int]]:
    """Generate an n x n rectangular maze adjacency matrix using Randomized Kruskal's.

    Returns a list of length n*n where entry i is [TOP, RIGHT, BOTTOM, LEFT].
    """
    spanning_tree, _ = _kruskal_rect_core(n, seed)
    return spanning_tree


def kruskal_rect_edges(n: int, seed: int) -> list[list[int]]:
    """Generate sorted spanning-tree edge pairs [u, v] for an n x n rectangular maze.

    Matches `RectangularKruskalMaze.create_maze_image()` edge sorting convention
    used by `RectangularDataset` for duplicate detection.
    """
    _, edges = _kruskal_rect_core(n, seed)
    edges.sort(key=lambda edge: edge[0])
    return edges


def prims_square_maze(n: int, seed: int) -> list[list[int]]:
    """Generate an n x n square maze adjacency matrix using Randomized Prim's.

    Matches `PrimsRandomized.prims_mst()` direction order: [TOP, LEFT, BOTTOM, RIGHT].
    """
    _validate_size(n, MAX_PRIMS_N)
    total_nodes = n * n
    mst = [[0, 0, 0, 0] for _ in range(total_nodes)]

    if total_nodes == 1:
        return mst

    rng = random.Random(seed)
    visited = [0]
    visited_mask = [False] * total_nodes
    visited_mask[0] = True

    for _ in range(total_nodes - 1):
        edges_pool: list[tuple[int, int]] = []
        for node in visited:
            row = node // n
            col = node % n
            if row > 0:
                top_node = node - n
                if not visited_mask[top_node]:
                    edges_pool.append((node, top_node))
            if col > 0:
                left_node = node - 1
                if not visited_mask[left_node]:
                    edges_pool.append((node, left_node))
            if row < n - 1:
                bottom_node = node + n
                if not visited_mask[bottom_node]:
                    edges_pool.append((node, bottom_node))
            if col < n - 1:
                right_node = node + 1
                if not visited_mask[right_node]:
                    edges_pool.append((node, right_node))

        node, next_node = rng.choice(edges_pool)
        if node - n == next_node:
            mst[node][0] = 1
            mst[next_node][2] = 1
        elif node - 1 == next_node:
            mst[node][1] = 1
            mst[next_node][3] = 1
        elif node + n == next_node:
            mst[node][2] = 1
            mst[next_node][0] = 1
        else:
            mst[node][3] = 1
            mst[next_node][1] = 1

        visited.append(next_node)
        visited_mask[next_node] = True

    return mst


def kruskal_triangular_maze(num_levels: int, seed: int) -> list[list[int]]:
    """Generate a triangular maze adjacency matrix using Randomized Kruskal's.

    Matches `TriangularMaze.kruskal_spanning_tree()` direction order:
    [PARENT, LEFT, RIGHT, CHILD].
    """
    _validate_size(num_levels, MAX_GRID_N)
    total_cells = num_levels * num_levels

    edges: list[tuple[int, int]] = []
    for lvl in range(num_levels):
        level_start = lvl * lvl
        next_level_start = (lvl + 1) * (lvl + 1)
        num_triangles = 2 * lvl + 1
        for tri in range(num_triangles):
            cell_1d = level_start + tri
            if tri > 0:
                edges.append((cell_1d, cell_1d - 1))
            if tri % 2 == 0 and lvl < num_levels - 1:
                edges.append((cell_1d, next_level_start + tri + 1))

    rng = random.Random(seed)
    rng.shuffle(edges)

    parent = list(range(total_cells))
    size = [1] * total_cells
    spanning_tree = [[0, 0, 0, 0] for _ in range(total_cells)]

    for cell1, cell2 in edges:
        if _dsu_union(parent, size, cell1, cell2):
            if cell2 == cell1 - 1:
                spanning_tree[cell1][1] = 1
                spanning_tree[cell2][2] = 1
            else:
                spanning_tree[cell1][3] = 1
                spanning_tree[cell2][0] = 1

    return spanning_tree


def dfs_triangular_maze(num_levels: int, seed: int) -> list[str]:
    """Generate triangular maze spanning-tree edge keys using Randomized DFS.

    Matches `TriangularMaze.computeMaze()` (`src/algorithms/triangular_using_dfs.py`),
    returning edge keys formatted as `"smaller:larger"` in traversal order.
    """
    _validate_size(num_levels, MAX_GRID_N)
    total_cells = num_levels * num_levels
    rng = random.Random(seed)

    cell_1d = rng.randint(0, total_cells - 1)
    visited_mask = [False] * total_cells
    visited_mask[cell_1d] = True
    visited_count = 1
    stack = [cell_1d]
    edge_keys: list[str] = []

    while visited_count < total_cells:
        # Convert 1D index to (level, cell)
        level = int(cell_1d**0.5)
        while (level + 1) * (level + 1) <= cell_1d:
            level += 1
        while level * level > cell_1d:
            level -= 1
        cell = cell_1d - level * level
        level_len = 2 * level + 1

        valid: list[int] = []
        # Exact neighbor check order: parent, left, child, right
        if cell % 2 != 0 and level > 0:
            parent_idx = (level - 1) * (level - 1) + (cell - 1)
            if not visited_mask[parent_idx]:
                valid.append(parent_idx)
        if cell > 0:
            left_idx = cell_1d - 1
            if not visited_mask[left_idx]:
                valid.append(left_idx)
        if cell % 2 == 0 and level < num_levels - 1:
            child_idx = (level + 1) * (level + 1) + (cell + 1)
            if not visited_mask[child_idx]:
                valid.append(child_idx)
        if cell < level_len - 1:
            right_idx = cell_1d + 1
            if not visited_mask[right_idx]:
                valid.append(right_idx)

        if valid:
            next_cell = rng.choice(valid)
            visited_mask[next_cell] = True
            visited_count += 1
            stack.append(next_cell)
            if cell_1d < next_cell:
                edge_keys.append(f"{cell_1d}:{next_cell}")
            else:
                edge_keys.append(f"{next_cell}:{cell_1d}")
            cell_1d = next_cell
        else:
            cell_1d = stack.pop()

    return edge_keys


def _pack_adj_hex(adj: list[list[int]]) -> str:
    return "".join(
        _HEX_DIGITS[row[0] | (row[1] << 1) | (row[2] << 2) | (row[3] << 3)]
        for row in adj
    )


def generate_maze_packed(seed: int, n: int, algorithm: str) -> str:
    """Generate a maze and return its compact lossless string encoding.

    Designed for high-throughput batch generation across all CPU cores (`--batch`).
    - `"kruskal_rect"`: hex nibble per cell encoding `[TOP, RIGHT, BOTTOM, LEFT]`
    - `"prims_square"`: hex nibble per cell encoding `[TOP, LEFT, BOTTOM, RIGHT]`
    - `"kruskal_tri"`: hex nibble per cell encoding `[PARENT, LEFT, RIGHT, CHILD]`
    - `"dfs_tri"`: comma-separated `"u:v"` edge keys in DFS traversal order
    """
    if algorithm == "kruskal_rect":
        return _pack_adj_hex(kruskal_rect_maze(n, seed))
    if algorithm == "prims_square":
        return _pack_adj_hex(prims_square_maze(n, seed))
    if algorithm == "kruskal_tri":
        return _pack_adj_hex(kruskal_triangular_maze(n, seed))
    if algorithm == "dfs_tri":
        return ",".join(dfs_triangular_maze(n, seed))
    raise ValueError(f"unsupported algorithm: {algorithm}")
