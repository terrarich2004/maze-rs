//! Deterministic circular and hexagonal DFS maze generators with CPython
//! MT19937 parity and Rayon batch execution.

#![forbid(unsafe_code)]

use rayon::prelude::*;

const MAX_CIRCULAR_LEVELS: i64 = 24;
const MAX_HEX_LEVELS: i64 = 63;
const B64_CHARS: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_";

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
        let (mut i, mut j, key_len) = (1usize, 0usize, key.len());
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
}

struct GridLayout {
    num_levels: usize,
    middle: usize,
    counts: Vec<usize>,
    offsets: Vec<usize>,
    cell_level: Vec<u8>,
    total_cells: usize,
}

impl GridLayout {
    fn from_counts(counts: Vec<usize>, middle: usize) -> Self {
        let num_levels = counts.len();
        let mut offsets = Vec::with_capacity(num_levels);
        let mut total_cells = 0usize;
        for &c in &counts {
            offsets.push(total_cells);
            total_cells += c;
        }
        let mut cell_level = Vec::with_capacity(total_cells);
        for (lvl, &c) in counts.iter().enumerate() {
            cell_level.resize(cell_level.len() + c, lvl as u8);
        }
        Self {
            num_levels,
            middle,
            counts,
            offsets,
            cell_level,
            total_cells,
        }
    }

    fn circular(num_levels: i64) -> Result<Self, String> {
        if num_levels < 2 {
            return Err("ValueError: circular num_levels must be at least 2".to_string());
        }
        if num_levels > MAX_CIRCULAR_LEVELS {
            return Err(format!(
                "ValueError: circular num_levels must be at most {MAX_CIRCULAR_LEVELS}"
            ));
        }
        let n = num_levels as usize;
        let mut counts = Vec::with_capacity(n);
        counts.push(1);
        for level in 1..n {
            let shift = (usize::BITS - 1) - (level + 1).leading_zeros();
            counts.push(8usize << shift);
        }
        Ok(Self::from_counts(counts, 0))
    }

    fn hexagonal(num_levels: i64) -> Result<Self, String> {
        if num_levels <= 0 {
            return Err("ValueError: hexagonal num_levels must be positive".to_string());
        }
        if num_levels > MAX_HEX_LEVELS {
            return Err(format!(
                "ValueError: hexagonal num_levels must be at most {MAX_HEX_LEVELS}"
            ));
        }
        let mut n = num_levels as usize;
        if n.is_multiple_of(2) {
            n += 1;
        }
        let middle = n / 2;
        let mut counts = vec![0usize; n];
        counts[middle] = n;
        let mut width = n - 1;
        for d in 1..=middle {
            counts[middle - d] = width;
            counts[middle + d] = width;
            width -= 1;
        }
        Ok(Self::from_counts(counts, middle))
    }
}

fn circular_dfs_run(layout: &GridLayout, seed: i64) -> Result<Vec<Vec<i64>>, String> {
    let total = layout.total_cells;
    let mut graph: Vec<Vec<i64>> = vec![Vec::new(); total];
    let mut rng = PyRandom::new(seed);
    let mut cell_1d = 1 + rng.randbelow(total - 1);

    let mut visited = vec![false; total];
    visited[cell_1d] = true;
    let mut visited_count = 1usize;
    let mut stack = Vec::with_capacity(total);
    stack.push(cell_1d);
    let mut unvisited = Vec::with_capacity(16);

    while visited_count < total {
        let level = layout.cell_level[cell_1d] as usize;
        let cell = cell_1d - layout.offsets[level];
        unvisited.clear();

        if level == 0 {
            for (idx, &seen) in visited
                .iter()
                .enumerate()
                .take(layout.counts[1] + 1)
                .skip(1)
            {
                if !seen {
                    unvisited.push(idx);
                }
            }
        } else {
            let parent = if level == 1 {
                0
            } else if layout.counts[level - 1] < layout.counts[level] {
                layout.offsets[level - 1] + (cell / 2)
            } else {
                layout.offsets[level - 1] + cell
            };
            if !visited[parent] {
                unvisited.push(parent);
            }
            let lvl_len = layout.counts[level];
            let lvl_off = layout.offsets[level];
            let left = lvl_off + if cell == 0 { lvl_len - 1 } else { cell - 1 };
            if !visited[left] {
                unvisited.push(left);
            }
            let right = lvl_off + if cell + 1 == lvl_len { 0 } else { cell + 1 };
            if !visited[right] {
                unvisited.push(right);
            }
            if level + 2 <= layout.num_levels {
                let next_off = layout.offsets[level + 1];
                if lvl_len < layout.counts[level + 1] {
                    let c_left = next_off + 2 * cell;
                    let c_right = c_left + 1;
                    if !visited[c_left] {
                        unvisited.push(c_left);
                    }
                    if !visited[c_right] {
                        unvisited.push(c_right);
                    }
                } else {
                    let child = next_off + cell;
                    if !visited[child] {
                        unvisited.push(child);
                    }
                }
            }
        }

        if !unvisited.is_empty() {
            let next_cell = unvisited[rng.randbelow(unvisited.len())];
            graph[cell_1d].push(next_cell as i64);
            graph[next_cell].push(cell_1d as i64);
            visited[next_cell] = true;
            visited_count += 1;
            if next_cell != 0 {
                stack.push(next_cell);
                cell_1d = next_cell;
            } else if let Some(prev) = stack.pop() {
                cell_1d = prev;
            } else {
                return Err("IndexError: pop from empty list".to_string());
            }
        } else if let Some(prev) = stack.pop() {
            cell_1d = prev;
        } else {
            return Err("IndexError: pop from empty list".to_string());
        }
    }
    Ok(graph)
}

fn hexagonal_dfs_bits(layout: &GridLayout, seed: i64) -> Result<Vec<u8>, String> {
    let total = layout.total_cells;
    let mut bits = vec![0u8; total];
    let mut rng = PyRandom::new(seed);
    let mut cell_1d = rng.randbelow(total);

    let mut visited = vec![false; total];
    visited[cell_1d] = true;
    let mut visited_count = 1usize;
    let mut stack = Vec::with_capacity(total);
    stack.push(cell_1d);

    while visited_count < total {
        let level = layout.cell_level[cell_1d] as usize;
        let cell = cell_1d - layout.offsets[level];
        let lvl_len = layout.counts[level];
        let increasing = level <= layout.middle;
        let is_last = cell + 1 == lvl_len;

        let mut valid = [(0usize, 0u8, 0u8); 6];
        let mut valid_len = 0usize;

        if level > 0 && !(cell == 0 && increasing) {
            let lp = layout.offsets[level - 1] + if increasing { cell - 1 } else { cell };
            if !visited[lp] {
                valid[valid_len] = (lp, 4, 1);
                valid_len += 1;
            }
        }
        if level > 0 && !(is_last && increasing) {
            let rp = layout.offsets[level - 1] + if increasing { cell } else { cell + 1 };
            if !visited[rp] {
                valid[valid_len] = (rp, 3, 0);
                valid_len += 1;
            }
        }
        if cell > 0 {
            let lc = layout.offsets[level] + (cell - 1);
            if !visited[lc] {
                valid[valid_len] = (lc, 5, 2);
                valid_len += 1;
            }
        }
        if cell + 1 < lvl_len {
            let rc = layout.offsets[level] + (cell + 1);
            if !visited[rc] {
                valid[valid_len] = (rc, 2, 5);
                valid_len += 1;
            }
        }
        if level + 1 < layout.num_levels && !(cell == 0 && level >= layout.middle) {
            let lch = layout.offsets[level + 1]
                + if level < layout.middle {
                    cell
                } else {
                    cell - 1
                };
            if !visited[lch] {
                valid[valid_len] = (lch, 0, 3);
                valid_len += 1;
            }
        }
        if level + 1 < layout.num_levels && !(is_last && level >= layout.middle) {
            let rch = layout.offsets[level + 1]
                + if level < layout.middle {
                    cell + 1
                } else {
                    cell
                };
            if !visited[rch] {
                valid[valid_len] = (rch, 1, 4);
                valid_len += 1;
            }
        }

        if valid_len > 0 {
            let (next_cell, dir, rev) = valid[rng.randbelow(valid_len)];
            visited[next_cell] = true;
            visited_count += 1;
            stack.push(next_cell);
            bits[cell_1d] |= 1 << dir;
            bits[next_cell] |= 1 << rev;
            cell_1d = next_cell;
        } else if let Some(prev) = stack.pop() {
            cell_1d = prev;
        } else {
            return Err("IndexError: pop from empty list".to_string());
        }
    }
    Ok(bits)
}

fn push_i64_ascii(out: &mut String, mut v: i64) {
    if v == 0 {
        out.push('0');
        return;
    }
    let mut buf = [0u8; 20];
    let mut pos = buf.len();
    while v > 0 {
        pos -= 1;
        buf[pos] = b'0' + (v % 10) as u8;
        v /= 10;
    }
    for &b in &buf[pos..] {
        out.push(b as char);
    }
}

fn pack_circular(graph: &[Vec<i64>]) -> String {
    let mut out = String::with_capacity(graph.len() * 8);
    for (i, nbrs) in graph.iter().enumerate() {
        if i > 0 {
            out.push(';');
        }
        for (j, &n) in nbrs.iter().enumerate() {
            if j > 0 {
                out.push(',');
            }
            push_i64_ascii(&mut out, n);
        }
    }
    out
}

fn pack_hex_b64(bits: &[u8]) -> String {
    let mut out = String::with_capacity(bits.len());
    for &b in bits {
        out.push(B64_CHARS[(b & 0x3f) as usize] as char);
    }
    out
}

/// Circular maze adjacency list via Randomized DFS.
pub fn circular_dfs_maze(num_levels: i64, seed: i64) -> Result<Vec<Vec<i64>>, String> {
    let layout = GridLayout::circular(num_levels)?;
    circular_dfs_run(&layout, seed)
}

/// Hexagonal maze adjacency matrix (`[BOTTOM, BOTTOM_RIGHT, TOP_RIGHT, TOP, TOP_LEFT, BOTTOM_LEFT]`).
pub fn hexagonal_dfs_maze(num_levels: i64, seed: i64) -> Result<Vec<Vec<i64>>, String> {
    let layout = GridLayout::hexagonal(num_levels)?;
    let bits = hexagonal_dfs_bits(&layout, seed)?;
    Ok(bits
        .into_iter()
        .map(|b| (0..6).map(|k| i64::from((b >> k) & 1)).collect())
        .collect())
}

/// Compact string encoding: 1 base64url char per cell for `hexagonal`, semicolon-separated rows for `circular`.
pub fn generate_shape_packed(seed: i64, num_levels: i64, shape: String) -> Result<String, String> {
    match shape.as_str() {
        "hexagonal" => {
            let layout = GridLayout::hexagonal(num_levels)?;
            Ok(pack_hex_b64(&hexagonal_dfs_bits(&layout, seed)?))
        }
        "circular" => {
            let layout = GridLayout::circular(num_levels)?;
            Ok(pack_circular(&circular_dfs_run(&layout, seed)?))
        }
        _ => Err(format!("ValueError: unsupported shape: {shape}")),
    }
}

/// Batch index for `GenerateShapePackedBatch`: precomputes grid layout once
/// and runs all seeds in parallel across CPU cores via Rayon.
pub struct GenerateShapePackedIndex {
    items: Vec<i64>,
}

impl GenerateShapePackedIndex {
    pub fn new(items: Vec<i64>) -> Self {
        Self { items }
    }

    pub fn run(&self, num_levels: i64, shape: String) -> Result<Vec<String>, String> {
        if self.items.is_empty() {
            return Ok(Vec::new());
        }
        match shape.as_str() {
            "hexagonal" => {
                let layout = GridLayout::hexagonal(num_levels)?;
                self.items
                    .par_iter()
                    .map(|&seed| hexagonal_dfs_bits(&layout, seed).map(|b| pack_hex_b64(&b)))
                    .collect()
            }
            "circular" => {
                let layout = GridLayout::circular(num_levels)?;
                self.items
                    .par_iter()
                    .map(|&seed| circular_dfs_run(&layout, seed).map(|g| pack_circular(&g)))
                    .collect()
            }
            _ => Err(format!("ValueError: unsupported shape: {shape}")),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn batch_matches_single_for_both_shapes() -> Result<(), String> {
        let seeds = vec![0, 1, -1, 42, 1337, 1i64 << 33];
        let batch = GenerateShapePackedIndex::new(seeds.clone());
        for (shape, lvl) in [("circular", 5), ("hexagonal", 7)] {
            let batched = batch.run(lvl, shape.to_string())?;
            let single: Result<Vec<String>, String> = seeds
                .iter()
                .map(|&s| generate_shape_packed(s, lvl, shape.to_string()))
                .collect();
            assert_eq!(batched, single?);
        }
        Ok(())
    }

    #[test]
    fn invalid_inputs_return_value_error() {
        assert!(circular_dfs_maze(1, 0).is_err());
        assert!(circular_dfs_maze(25, 0).is_err());
        assert!(hexagonal_dfs_maze(0, 0).is_err());
        assert!(hexagonal_dfs_maze(64, 0).is_err());
        assert!(generate_shape_packed(0, 5, "bogus".to_string()).is_err());
    }
}
