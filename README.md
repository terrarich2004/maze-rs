# maze-rs

Fast procedural maze generation in Rust with PyO3 bindings, multi-core batch execution, and pure-Python fallbacks.

Built on top of [`emadehsan/maze`](https://github.com/emadehsan/maze), this project moves the hot maze generation loops (Kruskal's, Prim's, and randomized DFS across rectangular, triangular, circular, and hexagonal grids) into two Rust extension modules while keeping the original `turtle` drawing scripts and `PIL` dataset export intact.

## Highlights

- **Four grid geometries, three algorithms:** rectangular Kruskal, square Prim's, triangular Kruskal, triangular DFS, circular DFS, and hexagonal DFS.
- **Bit-for-bit CPython RNG parity:** both Rust crates implement CPython's `MT19937` (`random.Random(seed)`, `_randbelow`, and Fisher-Yates `shuffle`), so the same `(size, seed)` produces the exact same maze in Python and Rust.
- **Multi-core batch API:** `GenerateMazePackedBatch` and `GenerateShapePackedBatch` keep seed lists in Rust, release the GIL via `py.detach()`, and generate batches in parallel across CPU cores using `rayon` and `mimalloc`.
- **Zero-panic error handling:** invalid grid sizes and unknown algorithm names raise standard Python exceptions (`ValueError`, `IndexError`) instead of panicking. Both crates forbid `unsafe` code.

## Project Layout

```text
src/
  maze_gen.py              # Seeded Python reference: rectangular & triangular mazes
  maze_shapes_gen.py       # Seeded Python reference: circular & hexagonal mazes
  rust_bridge.py           # Runtime loader (MAZE_BACKEND=rust|python)
  algorithms/              # Original algorithm classes (wired to Rust when built)
  datasets/                # Rectangular PNG dataset generator (uses Rust batch API)
  circular.py, hexagonal.py, rectangular.py, triangular.py
rust/
  maze-gen-rs/             # PyO3 crate for Kruskal, Prim, and triangular DFS
  maze-shapes-gen-rs/      # PyO3 crate for circular and hexagonal DFS
tests/
  test_parity.py           # Replays 2,029 regression fixtures across Python and Rust
  fixtures/                # Recorded reference outputs for all algorithms
```

## Building and Testing

Requires Rust (1.85+ / edition 2024), Python 3.9+, and `maturin`.

```bash
python3 -m venv .venv
source .venv/bin/activate
pip install maturin

# Build and install both PyO3 extensions in release mode
(cd rust/maze-gen-rs && maturin develop --release)
(cd rust/maze-shapes-gen-rs && maturin develop --release)

# Run Rust unit tests and lints
cargo test --manifest-path rust/maze-gen-rs/Cargo.toml
cargo test --manifest-path rust/maze-shapes-gen-rs/Cargo.toml

# Run Python <-> Rust parity suite (2,029 cases + batch checks)
python3 -m unittest tests/test_parity.py
```

## Usage

### Single maze generation

```python
import maze_gen_rs
import maze_shapes_gen_rs

# Rectangular 15x15 maze via Kruskal's: list of [TOP, RIGHT, BOTTOM, LEFT] flags
adj = maze_gen_rs.kruskal_rect_maze(15, seed=42)

# Square 12x12 maze via Prim's: list of [TOP, LEFT, BOTTOM, RIGHT] flags
prim_adj = maze_gen_rs.prims_square_maze(12, seed=42)

# Circular and hexagonal mazes via randomized DFS
circ_adj = maze_shapes_gen_rs.circular_dfs_maze(8, seed=42)
hex_adj = maze_shapes_gen_rs.hexagonal_dfs_maze(11, seed=42)
```

### Multi-core batch generation

Pass a list of seeds once to avoid per-item FFI overhead and run across all cores without the GIL:

```python
import maze_gen_rs
import maze_shapes_gen_rs

seeds = list(range(1000))

# Packed hex string per maze (1 nibble = 4 wall bits) or CSV edge keys for dfs_tri
batch = maze_gen_rs.GenerateMazePackedBatch(seeds)
rect_mazes = batch.run(15, "kruskal_rect")
prim_mazes = batch.run(12, "prims_square")

# Packed base64url string (hexagonal) or semicolon-separated adjacency rows (circular)
shape_batch = maze_shapes_gen_rs.GenerateShapePackedBatch(seeds)
hex_mazes = shape_batch.run(11, "hexagonal")
```

Set `MAZE_BACKEND=python` to disable native extensions in the existing `src/` classes and fall back to pure Python (`MAZE_BACKEND=rust` is the default).

## Benchmarks

Median wall-clock time to generate **1,000 mazes** (16-thread CPU, Python 3.14, `--release` with LTO). Output equality between pure Python, Rust single calls, and Rust batch calls is verified on every run.

| Algorithm | Size | Count | Python (ms) | Rust 1-by-1 (ms) | Speedup (1-by-1) | Rust batch (ms) | Speedup (batch) |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| `kruskal_rect` | 15x15 | 1,000 | 268.56 | 14.60 | 18.4x | 1.69 | **159.0x** |
| `kruskal_rect` | 30x30 | 1,000 | 1,241.20 | 42.55 | 29.2x | 4.54 | **273.4x** |
| `prims_square` | 12x12 | 1,000 | 1,889.43 | 40.79 | 46.3x | 4.45 | **424.2x** |
| `kruskal_tri` | 15 levels | 1,000 | 210.50 | 12.66 | 16.6x | 1.40 | **150.4x** |
| `dfs_tri` | 15 levels | 1,000 | 297.99 | 15.67 | 19.0x | 2.88 | **103.5x** |
| `circular` | 8 levels | 1,000 | 388.89 | 21.56 | 18.0x | 3.47 | **112.0x** |
| `circular` | 12 levels | 1,000 | 1,024.71 | 40.43 | 25.3x | 7.32 | **140.1x** |
| `hexagonal` | 11 levels | 1,000 | 170.01 | 9.56 | 17.8x | 1.21 | **140.4x** |
| `hexagonal` | 21 levels | 1,000 | 790.95 | 19.56 | 40.4x | 2.37 | **334.0x** |

## License

MIT License. Based on [`emadehsan/maze`](https://github.com/emadehsan/maze) — original Copyright (c) 2022 Emad Ehsan; see [LICENSE](LICENSE).
