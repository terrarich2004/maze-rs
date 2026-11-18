"""Regression and parity tests comparing Python and Rust PyO3 maze generators."""

from __future__ import annotations

import json
import os
import sys
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
FIXTURES = ROOT / "tests" / "fixtures"

if str(ROOT) not in sys.path:
    sys.path.insert(0, str(ROOT))

from src import maze_gen, maze_shapes_gen
from src.rust_bridge import load_maze_gen, load_maze_shapes


def _replay_fixture(fixture_path: Path, py_mod, rs_mod) -> int:
    count = 0
    with fixture_path.open() as fh:
        for line in fh:
            case = json.loads(line)
            fn_name = case["fn"]
            args = case["args"]
            py_fn = getattr(py_mod, fn_name)
            rs_fn = getattr(rs_mod, fn_name) if rs_mod is not None else None

            if "ok" in case:
                expected = case["ok"]
                assert py_fn(*args) == expected, f"Python mismatch: {fn_name}{args}"
                if rs_fn is not None:
                    assert rs_fn(*args) == expected, f"Rust mismatch: {fn_name}{args}"
            else:
                expected_err = case["error"].split(":", 1)[0]
                try:
                    py_fn(*args)
                    raise AssertionError(f"Python did not raise {expected_err}: {fn_name}{args}")
                except Exception as exc:
                    assert type(exc).__name__ == expected_err
                if rs_fn is not None:
                    try:
                        rs_fn(*args)
                        raise AssertionError(f"Rust did not raise {expected_err}: {fn_name}{args}")
                    except Exception as exc:
                        assert type(exc).__name__ == expected_err
            count += 1
    return count


class TestMazeParity(unittest.TestCase):
    def test_core_fixtures(self) -> None:
        os.environ["MAZE_BACKEND"] = "rust"
        rs = load_maze_gen()
        self.assertIsNotNone(rs, "maze_gen_rs extension is not built")
        checked = _replay_fixture(FIXTURES / "maze_gen_cases.jsonl", maze_gen, rs)
        self.assertEqual(checked, 1350)

    def test_shapes_fixtures(self) -> None:
        os.environ["MAZE_BACKEND"] = "rust"
        rs = load_maze_shapes()
        self.assertIsNotNone(rs, "maze_shapes_gen_rs extension is not built")
        checked = _replay_fixture(FIXTURES / "maze_shapes_cases.jsonl", maze_shapes_gen, rs)
        self.assertEqual(checked, 679)

    def test_batch_matches_python(self) -> None:
        os.environ["MAZE_BACKEND"] = "rust"
        rs_core = load_maze_gen()
        rs_shapes = load_maze_shapes()
        self.assertIsNotNone(rs_core)
        self.assertIsNotNone(rs_shapes)

        seeds = list(range(-5, 45))
        batch_core = rs_core.GenerateMazePackedBatch(seeds)
        for algo, n in [
            ("kruskal_rect", 10),
            ("prims_square", 8),
            ("kruskal_tri", 10),
            ("dfs_tri", 10),
        ]:
            expected = [maze_gen.generate_maze_packed(s, n, algo) for s in seeds]
            self.assertEqual(batch_core.run(n, algo), expected)

        batch_shapes = rs_shapes.GenerateShapePackedBatch(seeds)
        for shape, n in [("circular", 6), ("hexagonal", 9)]:
            expected = [maze_shapes_gen.generate_shape_packed(s, n, shape) for s in seeds]
            self.assertEqual(batch_shapes.run(n, shape), expected)


if __name__ == "__main__":
    unittest.main()
