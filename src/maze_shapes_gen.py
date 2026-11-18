"""Seeded circular and hexagonal DFS maze generators."""

import random

MAX_CIRCULAR_LEVELS = 24
MAX_HEX_LEVELS = 63
_B64_CHARS = "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_"


def _circular_level_counts(num_levels: int) -> list[int]:
    counts = [1]
    for level in range(1, num_levels):
        counts.append(8 << ((level + 1).bit_length() - 1))
    return counts


def circular_dfs_maze(num_levels: int, seed: int) -> list[list[int]]:
    """Generate a circular maze adjacency list using Randomized DFS.

    Matches `CircularMaze(num_levels, r).create_dfs_tree()` (`src/circular.py`).
    """
    if num_levels < 2:
        raise ValueError("circular num_levels must be at least 2")
    if num_levels > MAX_CIRCULAR_LEVELS:
        raise ValueError(f"circular num_levels must be at most {MAX_CIRCULAR_LEVELS}")

    counts = _circular_level_counts(num_levels)
    offsets = [0] * num_levels
    acc = 0
    for lvl in range(num_levels):
        offsets[lvl] = acc
        acc += counts[lvl]
    total_cells = acc

    graph: list[list[int]] = [[] for _ in range(total_cells)]
    rng = random.Random(seed)
    cell_1d = rng.randint(1, total_cells - 1)

    visited_mask = [False] * total_cells
    visited_mask[cell_1d] = True
    visited_count = 1
    stack = [cell_1d]

    while visited_count < total_cells:
        # Locate (level, cell)
        level = 0
        rem = cell_1d
        while rem >= counts[level]:
            rem -= counts[level]
            level += 1
        cell = rem

        unvisited: list[int] = []
        if level == 0:
            for idx in range(1, counts[1] + 1):
                if not visited_mask[idx]:
                    unvisited.append(idx)
        else:
            # Parent
            if level == 1:
                parent = 0
            elif counts[level - 1] < counts[level]:
                parent = offsets[level - 1] + (cell // 2)
            else:
                parent = offsets[level - 1] + cell
            if not visited_mask[parent]:
                unvisited.append(parent)

            # Left
            lvl_len = counts[level]
            lvl_off = offsets[level]
            left = lvl_off + (lvl_len - 1 if cell == 0 else cell - 1)
            if not visited_mask[left]:
                unvisited.append(left)

            # Right
            right = lvl_off + (0 if cell == lvl_len - 1 else cell + 1)
            if not visited_mask[right]:
                unvisited.append(right)

            # Children
            if level <= num_levels - 2:
                next_off = offsets[level + 1]
                if lvl_len < counts[level + 1]:
                    c_left = next_off + 2 * cell
                    c_right = c_left + 1
                    if not visited_mask[c_left]:
                        unvisited.append(c_left)
                    if not visited_mask[c_right]:
                        unvisited.append(c_right)
                else:
                    child = next_off + cell
                    if not visited_mask[child]:
                        unvisited.append(child)

        if unvisited:
            next_cell = rng.choice(unvisited)
            graph[cell_1d].append(next_cell)
            graph[next_cell].append(cell_1d)
            visited_mask[next_cell] = True
            visited_count += 1
            if next_cell != 0:
                stack.append(next_cell)
                cell_1d = next_cell
            else:
                cell_1d = stack.pop()
        else:
            cell_1d = stack.pop()

    return graph


def hexagonal_dfs_maze(num_levels: int, seed: int) -> list[list[int]]:
    """Generate a hexagonal maze adjacency matrix using Randomized DFS.

    Matches `HexagonalMaze(side_len, num_levels).create_dfs_tree()` (`src/hexagonal.py`).
    Even `num_levels` is incremented by 1 for symmetry, matching the original constructor.
    Direction order per cell: [BOTTOM, BOTTOM_RIGHT, TOP_RIGHT, TOP, TOP_LEFT, BOTTOM_LEFT].
    """
    if num_levels <= 0:
        raise ValueError("hexagonal num_levels must be positive")
    if num_levels > MAX_HEX_LEVELS:
        raise ValueError(f"hexagonal num_levels must be at most {MAX_HEX_LEVELS}")

    if num_levels % 2 == 0:
        num_levels += 1

    middle = num_levels // 2
    counts = [0] * num_levels
    counts[middle] = num_levels
    width = num_levels - 1
    above = middle - 1
    below = middle + 1
    while above >= 0 and below < num_levels:
        counts[above] = width
        counts[below] = width
        width -= 1
        above -= 1
        below += 1

    offsets = [0] * num_levels
    acc = 0
    for lvl in range(num_levels):
        offsets[lvl] = acc
        acc += counts[lvl]
    total_cells = acc

    spanning_tree = [[0, 0, 0, 0, 0, 0] for _ in range(total_cells)]
    rng = random.Random(seed)
    cell_1d = rng.randint(0, total_cells - 1)

    visited_mask = [False] * total_cells
    visited_mask[cell_1d] = True
    visited_count = 1
    stack = [cell_1d]

    while visited_count < total_cells:
        level = 0
        rem = cell_1d
        while rem >= counts[level]:
            rem -= counts[level]
            level += 1
        cell = rem
        lvl_len = counts[level]
        increasing = level <= middle
        is_last = cell == lvl_len - 1

        # Exact neighbor order: left_parent, right_parent, left_cell, right_cell, left_child, right_child
        # Each tuple: (neighbor_1d, dir_from_cur, dir_from_nbr)
        valid: list[tuple[int, int, int]] = []

        if level > 0 and not (cell == 0 and increasing):
            lp = offsets[level - 1] + (cell - 1 if increasing else cell)
            if not visited_mask[lp]:
                valid.append((lp, 4, 1))

        if level > 0 and not (is_last and increasing):
            rp = offsets[level - 1] + (cell if increasing else cell + 1)
            if not visited_mask[rp]:
                valid.append((rp, 3, 0))

        if cell > 0:
            lc_same = offsets[level] + (cell - 1)
            if not visited_mask[lc_same]:
                valid.append((lc_same, 5, 2))

        if cell < lvl_len - 1:
            rc_same = offsets[level] + (cell + 1)
            if not visited_mask[rc_same]:
                valid.append((rc_same, 2, 5))

        if level < num_levels - 1 and not (cell == 0 and level >= middle):
            lch = offsets[level + 1] + (cell if level < middle else cell - 1)
            if not visited_mask[lch]:
                valid.append((lch, 0, 3))

        if level < num_levels - 1 and not (is_last and level >= middle):
            rch = offsets[level + 1] + (cell + 1 if level < middle else cell)
            if not visited_mask[rch]:
                valid.append((rch, 1, 4))

        if valid:
            next_cell, direction, reverse_direction = rng.choice(valid)
            visited_mask[next_cell] = True
            visited_count += 1
            stack.append(next_cell)
            spanning_tree[cell_1d][direction] = 1
            spanning_tree[next_cell][reverse_direction] = 1
            cell_1d = next_cell
        else:
            cell_1d = stack.pop()

    return spanning_tree


def generate_shape_packed(seed: int, num_levels: int, shape: str) -> str:
    """Generate a circular or hexagonal maze in compact lossless string encoding.

    Designed for high-throughput multi-core batch generation (`--batch`).
    - `"hexagonal"`: 1 base64url char per cell encoding 6 wall bits `[0..5]`
    - `"circular"`: `";"`-separated adjacency rows of `","`-separated neighbor indices
    """
    if shape == "hexagonal":
        adj = hexagonal_dfs_maze(num_levels, seed)
        return "".join(
            _B64_CHARS[
                r[0]
                | (r[1] << 1)
                | (r[2] << 2)
                | (r[3] << 3)
                | (r[4] << 4)
                | (r[5] << 5)
            ]
            for r in adj
        )
    if shape == "circular":
        graph = circular_dfs_maze(num_levels, seed)
        return ";".join(",".join(str(x) for x in nbrs) for nbrs in graph)
    raise ValueError(f"unsupported shape: {shape}")
