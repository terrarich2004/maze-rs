//! Deterministic rectangular and triangular maze generators (Kruskal, Prim, DFS)
//! with CPython MT19937 parity and Rayon batch execution.

#![forbid(unsafe_code)]

use rayon::prelude::*;

const MAX_GRID_N: i64 = 128;
const MAX_PRIMS_N: i64 = 32;
const HEX_DIGITS: &[u8; 16] = b"0123456789abcdef";

const MT_N: usize = 624;
const MT_M: usize = 397;
const MATRIX_A: u32 = 0x9908_b0df;
const UPPER_MASK: u32 = 0x8000_0000;
const LOWER_MASK: u32 = 0x7fff_ffff;

/// CPython-compatible Mersenne Twister (`random.Random(seed)`).
struct PyRandom {
    mt: [u32; MT_N],
    index: usize,
}

impl PyRandom {
    fn new(seed: i64) -> Self {
        let abs = seed.unsigned_abs();
        let lo = abs as u32;
        let hi = (abs >> 32) as u32;
        if hi == 0 {
            Self::from_key(&[lo])
        } else {
            Self::from_key(&[lo, hi])
        }
    }

    fn from_key(key: &[u32]) -> Self {
        let mut mt = [0u32; MT_N];
        mt[0] = 19_650_218;
        for i in 1..MT_N {
            mt[i] = 1_812_433_253u32
                .wrapping_mul(mt[i - 1] ^ (mt[i - 1] >> 30))
                .wrapping_add(i as u32);
        }
        let mut i: usize = 1;
        let mut j: usize = 0;
        let key_len = key.len();
        for _ in 0..MT_N.max(key_len) {
            mt[i] = (mt[i] ^ ((mt[i - 1] ^ (mt[i - 1] >> 30)).wrapping_mul(1_664_525)))
                .wrapping_add(key[j])
                .wrapping_add(j as u32);
            i += 1;
            j += 1;
            if i >= MT_N {
                mt[0] = mt[MT_N - 1];
                i = 1;
            }
            if j >= key_len {
                j = 0;
            }
        }
        for _ in 0..(MT_N - 1) {
            mt[i] = (mt[i] ^ ((mt[i - 1] ^ (mt[i - 1] >> 30)).wrapping_mul(1_566_083_941)))
                .wrapping_sub(i as u32);
            i += 1;
            if i >= MT_N {
                mt[0] = mt[MT_N - 1];
                i = 1;
            }
        }
        mt[0] = 0x8000_0000;
        Self { mt, index: MT_N }
    }

    #[inline]
    fn next_u32(&mut self) -> u32 {
        if self.index >= MT_N {
            for kk in 0..(MT_N - MT_M) {
                let y = (self.mt[kk] & UPPER_MASK) | (self.mt[kk + 1] & LOWER_MASK);
                self.mt[kk] =
                    self.mt[kk + MT_M] ^ (y >> 1) ^ if (y & 1) != 0 { MATRIX_A } else { 0 };
            }
            for kk in (MT_N - MT_M)..(MT_N - 1) {
                let y = (self.mt[kk] & UPPER_MASK) | (self.mt[kk + 1] & LOWER_MASK);
                self.mt[kk] =
                    self.mt[kk + MT_M - MT_N] ^ (y >> 1) ^ if (y & 1) != 0 { MATRIX_A } else { 0 };
            }
            let y = (self.mt[MT_N - 1] & UPPER_MASK) | (self.mt[0] & LOWER_MASK);
            self.mt[MT_N - 1] =
                self.mt[MT_M - 1] ^ (y >> 1) ^ if (y & 1) != 0 { MATRIX_A } else { 0 };
            self.index = 0;
        }
        let mut y = self.mt[self.index];
        self.index += 1;
        y ^= y >> 11;
        y ^= (y << 7) & 0x9d2c_5680;
        y ^= (y << 15) & 0xefc6_0000;
        y ^= y >> 18;
        y
    }

    #[inline]
    fn randbelow(&mut self, n: usize) -> usize {
        let n32 = n as u32;
        let shift = n32.leading_zeros();
        loop {
            let r = self.next_u32() >> shift;
            if r < n32 {
                return r as usize;
            }
        }
    }

    fn shuffle<T>(&mut self, slice: &mut [T]) {
        for i in (1..slice.len()).rev() {
            let j = self.randbelow(i + 1);
            slice.swap(i, j);
        }
    }
}

#[inline]
fn validate_size(n: i64, max_n: i64) -> Result<usize, String> {
    if n <= 0 {
        return Err("ValueError: grid size n must be positive".to_string());
    }
    if n > max_n {
        return Err(format!("ValueError: grid size n must be at most {max_n}"));
    }
    Ok(n as usize)
}

#[inline]
fn dsu_find(parent: &mut [u32], mut x: usize) -> usize {
    let mut item = x;
    while item != parent[item] as usize {
        item = parent[item] as usize;
    }
    let root = item;
    while x != root {
        let nxt = parent[x] as usize;
        parent[x] = root as u32;
        x = nxt;
    }
    root
}

#[inline]
fn dsu_union(parent: &mut [u32], size: &mut [u32], a: usize, b: usize) -> bool {
    let ra = dsu_find(parent, a);
    let rb = dsu_find(parent, b);
    if ra == rb {
        return false;
    }
    if size[ra] < size[rb] {
        size[rb] += size[ra];
        parent[ra] = rb as u32;
    } else {
        size[ra] += size[rb];
        parent[rb] = ra as u32;
    }
    true
}

fn rect_grid_edges(n: usize) -> Vec<(u32, u32)> {
    let mut edges = Vec::with_capacity(2 * n * (n - 1));
    for row in 0..n {
        let row_base = n * row;
        for col in 0..n {
            let node = (row_base + col) as u32;
            if col + 1 < n {
                edges.push((node, node + 1));
            }
            if row + 1 < n {
                edges.push((node, node + n as u32));
            }
        }
    }
    edges
}

fn tri_grid_edges(num_levels: usize) -> Vec<(u32, u32)> {
    let mut edges = Vec::with_capacity(num_levels * num_levels * 3 / 2);
    for lvl in 0..num_levels {
        let level_start = lvl * lvl;
        let next_start = (lvl + 1) * (lvl + 1);
        let num_tris = 2 * lvl + 1;
        for tri in 0..num_tris {
            let cell = (level_start + tri) as u32;
            if tri > 0 {
                edges.push((cell, cell - 1));
            }
            if tri.is_multiple_of(2) && lvl + 1 < num_levels {
                edges.push((cell, (next_start + tri + 1) as u32));
            }
        }
    }
    edges
}

fn bits_to_adj(bits: &[u8]) -> Vec<Vec<i64>> {
    bits.iter()
        .map(|&b| {
            vec![
                i64::from(b & 1),
                i64::from((b >> 1) & 1),
                i64::from((b >> 2) & 1),
                i64::from((b >> 3) & 1),
            ]
        })
        .collect()
}

fn bits_to_hex(bits: &[u8]) -> String {
    let mut out = String::with_capacity(bits.len());
    for &b in bits {
        out.push(HEX_DIGITS[(b & 0x0f) as usize] as char);
    }
    out
}

#[derive(Default)]
struct KruskalScratch {
    edges: Vec<(u32, u32)>,
    parent: Vec<u32>,
    size: Vec<u32>,
    bits: Vec<u8>,
}

impl KruskalScratch {
    fn prepare(&mut self, total: usize, seed: i64, base_edges: &[(u32, u32)]) {
        self.edges.clear();
        self.edges.extend_from_slice(base_edges);
        let mut rng = PyRandom::new(seed);
        rng.shuffle(&mut self.edges);

        self.parent.clear();
        self.parent.extend(0..total as u32);
        self.size.clear();
        self.size.resize(total, 1);
        self.bits.clear();
        self.bits.resize(total, 0);
    }
}

fn kruskal_rect_run(
    n: usize,
    seed: i64,
    base_edges: &[(u32, u32)],
    sc: &mut KruskalScratch,
    mut accepted: Option<&mut Vec<Vec<i64>>>,
) {
    sc.prepare(n * n, seed, base_edges);
    for &(c1, c2) in &sc.edges {
        let u = c1 as usize;
        let v = c2 as usize;
        if dsu_union(&mut sc.parent, &mut sc.size, u, v) {
            if v == u + 1 {
                sc.bits[u] |= 1 << 1;
                sc.bits[v] |= 1 << 3;
            } else {
                sc.bits[u] |= 1 << 2;
                sc.bits[v] |= 1 << 0;
            }
            if let Some(acc) = accepted.as_deref_mut() {
                acc.push(vec![i64::from(c1), i64::from(c2)]);
            }
        }
    }
}

fn prims_square_bits(n: usize, seed: i64, bits: &mut Vec<u8>) {
    let total = n * n;
    bits.clear();
    bits.resize(total, 0);
    if total == 1 {
        return;
    }

    let mut rng = PyRandom::new(seed);
    let mut visited_mask = vec![false; total];
    visited_mask[0] = true;
    // Frontier preserves exact insertion order of `visited`, pruning saturated nodes.
    let mut frontier = Vec::with_capacity(total);
    frontier.push(0usize);
    let mut edges_pool: Vec<(usize, usize)> = Vec::with_capacity(4 * n);

    for _ in 0..(total - 1) {
        edges_pool.clear();
        frontier.retain(|&node| {
            let row = node / n;
            let col = node % n;
            let before = edges_pool.len();
            if row > 0 {
                let top = node - n;
                if !visited_mask[top] {
                    edges_pool.push((node, top));
                }
            }
            if col > 0 {
                let left = node - 1;
                if !visited_mask[left] {
                    edges_pool.push((node, left));
                }
            }
            if row + 1 < n {
                let bottom = node + n;
                if !visited_mask[bottom] {
                    edges_pool.push((node, bottom));
                }
            }
            if col + 1 < n {
                let right = node + 1;
                if !visited_mask[right] {
                    edges_pool.push((node, right));
                }
            }
            edges_pool.len() > before
        });

        let pick = rng.randbelow(edges_pool.len());
        let (node, next_node) = edges_pool[pick];
        if node >= n && node - n == next_node {
            bits[node] |= 1 << 0;
            bits[next_node] |= 1 << 2;
        } else if node >= 1 && node - 1 == next_node {
            bits[node] |= 1 << 1;
            bits[next_node] |= 1 << 3;
        } else if node + n == next_node {
            bits[node] |= 1 << 2;
            bits[next_node] |= 1 << 0;
        } else {
            bits[node] |= 1 << 3;
            bits[next_node] |= 1 << 1;
        }

        visited_mask[next_node] = true;
        frontier.push(next_node);
    }
}

fn kruskal_tri_run(
    num_levels: usize,
    seed: i64,
    base_edges: &[(u32, u32)],
    sc: &mut KruskalScratch,
) {
    sc.prepare(num_levels * num_levels, seed, base_edges);
    for &(c1, c2) in &sc.edges {
        let u = c1 as usize;
        let v = c2 as usize;
        if dsu_union(&mut sc.parent, &mut sc.size, u, v) {
            if u == v + 1 {
                sc.bits[u] |= 1 << 1;
                sc.bits[v] |= 1 << 2;
            } else {
                sc.bits[u] |= 1 << 3;
                sc.bits[v] |= 1 << 0;
            }
        }
    }
}

fn dfs_tri_pairs(num_levels: usize, seed: i64) -> Result<Vec<(usize, usize)>, String> {
    let total = num_levels * num_levels;
    let mut rng = PyRandom::new(seed);
    let mut cell_1d = rng.randbelow(total);

    let mut visited_mask = vec![false; total];
    visited_mask[cell_1d] = true;
    let mut visited_count = 1usize;
    let mut stack = Vec::with_capacity(total);
    stack.push(cell_1d);
    let mut out = Vec::with_capacity(total.saturating_sub(1));

    while visited_count < total {
        let level = cell_1d.isqrt();
        let cell = cell_1d - level * level;
        let level_len = 2 * level + 1;

        let mut valid = [0usize; 4];
        let mut valid_len = 0usize;
        if !cell.is_multiple_of(2) && level > 0 {
            let p = (level - 1) * (level - 1) + (cell - 1);
            if !visited_mask[p] {
                valid[valid_len] = p;
                valid_len += 1;
            }
        }
        if cell > 0 {
            let l = cell_1d - 1;
            if !visited_mask[l] {
                valid[valid_len] = l;
                valid_len += 1;
            }
        }
        if cell.is_multiple_of(2) && level + 1 < num_levels {
            let c = (level + 1) * (level + 1) + (cell + 1);
            if !visited_mask[c] {
                valid[valid_len] = c;
                valid_len += 1;
            }
        }
        if cell + 1 < level_len {
            let r = cell_1d + 1;
            if !visited_mask[r] {
                valid[valid_len] = r;
                valid_len += 1;
            }
        }

        if valid_len > 0 {
            let next_cell = valid[rng.randbelow(valid_len)];
            visited_mask[next_cell] = true;
            visited_count += 1;
            stack.push(next_cell);
            let pair = if cell_1d < next_cell {
                (cell_1d, next_cell)
            } else {
                (next_cell, cell_1d)
            };
            out.push(pair);
            cell_1d = next_cell;
        } else if let Some(prev) = stack.pop() {
            cell_1d = prev;
        } else {
            return Err("IndexError: pop from empty list".to_string());
        }
    }
    Ok(out)
}

fn push_usize_ascii(out: &mut String, mut v: usize) {
    let mut buf = [0u8; 20];
    let mut pos = buf.len();
    if v == 0 {
        out.push('0');
        return;
    }
    while v > 0 {
        pos -= 1;
        buf[pos] = b'0' + (v % 10) as u8;
        v /= 10;
    }
    for &b in &buf[pos..] {
        out.push(b as char);
    }
}

fn format_dfs_csv(pairs: &[(usize, usize)]) -> String {
    let mut out = String::with_capacity(pairs.len() * 8);
    for (idx, &(a, b)) in pairs.iter().enumerate() {
        if idx > 0 {
            out.push(',');
        }
        push_usize_ascii(&mut out, a);
        out.push(':');
        push_usize_ascii(&mut out, b);
    }
    out
}

/// n x n rectangular maze adjacency matrix: `[TOP, RIGHT, BOTTOM, LEFT]` per cell.
pub fn kruskal_rect_maze(n: i64, seed: i64) -> Result<Vec<Vec<i64>>, String> {
    let n = validate_size(n, MAX_GRID_N)?;
    let base = rect_grid_edges(n);
    let mut sc = KruskalScratch::default();
    kruskal_rect_run(n, seed, &base, &mut sc, None);
    Ok(bits_to_adj(&sc.bits))
}

/// Spanning-tree edge pairs `[u, v]` stably sorted by `u`.
pub fn kruskal_rect_edges(n: i64, seed: i64) -> Result<Vec<Vec<i64>>, String> {
    let n = validate_size(n, MAX_GRID_N)?;
    let base = rect_grid_edges(n);
    let mut sc = KruskalScratch::default();
    let mut accepted = Vec::with_capacity(n * n - 1);
    kruskal_rect_run(n, seed, &base, &mut sc, Some(&mut accepted));
    accepted.sort_by_key(|edge| edge[0]);
    Ok(accepted)
}

/// n x n square maze via Randomized Prim's: `[TOP, LEFT, BOTTOM, RIGHT]` per cell.
pub fn prims_square_maze(n: i64, seed: i64) -> Result<Vec<Vec<i64>>, String> {
    let n = validate_size(n, MAX_PRIMS_N)?;
    let mut bits = Vec::new();
    prims_square_bits(n, seed, &mut bits);
    Ok(bits_to_adj(&bits))
}

/// Triangular maze via Randomized Kruskal's: `[TOP, LEFT, RIGHT, BOTTOM]` per cell.
pub fn kruskal_triangular_maze(num_levels: i64, seed: i64) -> Result<Vec<Vec<i64>>, String> {
    let num_levels = validate_size(num_levels, MAX_GRID_N)?;
    let base = tri_grid_edges(num_levels);
    let mut sc = KruskalScratch::default();
    kruskal_tri_run(num_levels, seed, &base, &mut sc);
    Ok(bits_to_adj(&sc.bits))
}

/// Triangular maze via Randomized DFS returning `"u:v"` edge keys in traversal order.
pub fn dfs_triangular_maze(num_levels: i64, seed: i64) -> Result<Vec<String>, String> {
    let num_levels = validate_size(num_levels, MAX_GRID_N)?;
    let pairs = dfs_tri_pairs(num_levels, seed)?;
    let mut out = Vec::with_capacity(pairs.len());
    for (a, b) in pairs {
        let mut s = String::with_capacity(11);
        push_usize_ascii(&mut s, a);
        s.push(':');
        push_usize_ascii(&mut s, b);
        out.push(s);
    }
    Ok(out)
}

/// Compact string encoding: 1 hex nibble per cell (4 wall bits) or CSV edge keys for `dfs_tri`.
pub fn generate_maze_packed(seed: i64, n: i64, algorithm: String) -> Result<String, String> {
    match algorithm.as_str() {
        "kruskal_rect" => {
            let n = validate_size(n, MAX_GRID_N)?;
            let base = rect_grid_edges(n);
            let mut sc = KruskalScratch::default();
            kruskal_rect_run(n, seed, &base, &mut sc, None);
            Ok(bits_to_hex(&sc.bits))
        }
        "prims_square" => {
            let n = validate_size(n, MAX_PRIMS_N)?;
            let mut bits = Vec::new();
            prims_square_bits(n, seed, &mut bits);
            Ok(bits_to_hex(&bits))
        }
        "kruskal_tri" => {
            let n = validate_size(n, MAX_GRID_N)?;
            let base = tri_grid_edges(n);
            let mut sc = KruskalScratch::default();
            kruskal_tri_run(n, seed, &base, &mut sc);
            Ok(bits_to_hex(&sc.bits))
        }
        "dfs_tri" => {
            let n = validate_size(n, MAX_GRID_N)?;
            Ok(format_dfs_csv(&dfs_tri_pairs(n, seed)?))
        }
        _ => Err(format!("ValueError: unsupported algorithm: {algorithm}")),
    }
}

/// Batch index for `GenerateMazePackedBatch`: shares base edge templates and
/// reuses per-thread scratch buffers across all CPU cores via Rayon.
pub struct GenerateMazePackedIndex {
    items: Vec<i64>,
}

impl GenerateMazePackedIndex {
    pub fn new(items: Vec<i64>) -> Self {
        Self { items }
    }

    pub fn run(&self, n: i64, algorithm: String) -> Result<Vec<String>, String> {
        if self.items.is_empty() {
            return Ok(Vec::new());
        }
        match algorithm.as_str() {
            "kruskal_rect" => {
                let n = validate_size(n, MAX_GRID_N)?;
                let base = rect_grid_edges(n);
                Ok(self
                    .items
                    .par_iter()
                    .map_init(KruskalScratch::default, |sc, &seed| {
                        kruskal_rect_run(n, seed, &base, sc, None);
                        bits_to_hex(&sc.bits)
                    })
                    .collect())
            }
            "prims_square" => {
                let n = validate_size(n, MAX_PRIMS_N)?;
                Ok(self
                    .items
                    .par_iter()
                    .map_init(Vec::new, |bits, &seed| {
                        prims_square_bits(n, seed, bits);
                        bits_to_hex(bits)
                    })
                    .collect())
            }
            "kruskal_tri" => {
                let n = validate_size(n, MAX_GRID_N)?;
                let base = tri_grid_edges(n);
                Ok(self
                    .items
                    .par_iter()
                    .map_init(KruskalScratch::default, |sc, &seed| {
                        kruskal_tri_run(n, seed, &base, sc);
                        bits_to_hex(&sc.bits)
                    })
                    .collect())
            }
            "dfs_tri" => {
                let n = validate_size(n, MAX_GRID_N)?;
                self.items
                    .par_iter()
                    .map(|&seed| dfs_tri_pairs(n, seed).map(|pairs| format_dfs_csv(&pairs)))
                    .collect()
            }
            _ => Err(format!("ValueError: unsupported algorithm: {algorithm}")),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mt19937_matches_cpython() {
        let cases: &[(i64, [u32; 3])] = &[
            (0, [3_626_764_237, 1_654_615_998, 3_255_389_356]),
            (1, [577_090_037, 2_444_712_010, 3_639_700_191]),
            (-1, [577_090_037, 2_444_712_010, 3_639_700_191]),
            (42, [2_746_317_213, 478_163_327, 107_420_369]),
            (1i64 << 32, [485_306_839, 1_508_871_100, 1_794_561_286]),
            (
                -4_611_686_018_427_400_249,
                [2_104_045_543, 3_248_428_082, 3_990_934_593],
            ),
        ];
        for &(seed, expected) in cases {
            let mut rng = PyRandom::new(seed);
            assert_eq!([rng.next_u32(), rng.next_u32(), rng.next_u32()], expected);
        }
    }

    #[test]
    fn batch_index_matches_single_calls() -> Result<(), String> {
        let seeds: Vec<i64> = vec![0, 1, -1, 42, 1337, 1i64 << 33];
        let batch = GenerateMazePackedIndex::new(seeds.clone());
        for algo in ["kruskal_rect", "prims_square", "kruskal_tri", "dfs_tri"] {
            let batched = batch.run(6, algo.to_string())?;
            let single: Result<Vec<String>, String> = seeds
                .iter()
                .map(|&s| generate_maze_packed(s, 6, algo.to_string()))
                .collect();
            assert_eq!(batched, single?);
        }
        Ok(())
    }

    #[test]
    fn invalid_inputs_return_value_error() {
        assert!(kruskal_rect_maze(0, 1).is_err());
        assert!(kruskal_rect_maze(129, 1).is_err());
        assert!(prims_square_maze(0, 1).is_err());
        assert!(prims_square_maze(33, 1).is_err());
        assert!(kruskal_triangular_maze(-1, 1).is_err());
        assert!(dfs_triangular_maze(0, 1).is_err());
        assert_eq!(
            dfs_triangular_maze(2, 7),
            Err("IndexError: pop from empty list".to_string())
        );
        assert!(generate_maze_packed(1, 5, "bogus".to_string()).is_err());
    }
}
