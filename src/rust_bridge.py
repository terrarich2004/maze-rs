"""Optional loader for the compiled PyO3 maze extensions.

Set MAZE_BACKEND=python to force the pure-Python fallback (defaults to "rust").
"""

from __future__ import annotations

import os
import sys
from pathlib import Path
from typing import Any

_REPO_ROOT = Path(__file__).resolve().parent.parent


def _add_local_extensions() -> None:
    for rel in (
        "rust/maze-gen-rs/python",
        "rust/maze-shapes-gen-rs/python",
    ):
        path = str(_REPO_ROOT / rel)
        if path not in sys.path and os.path.isdir(path):
            sys.path.insert(0, path)


def _use_rust() -> bool:
    return os.environ.get("MAZE_BACKEND", "rust").lower() != "python"


def load_maze_gen() -> Any | None:
    if not _use_rust():
        return None
    _add_local_extensions()
    try:
        import maze_gen_rs  # type: ignore[import-not-found]

        return maze_gen_rs
    except ImportError:
        return None


def load_maze_shapes() -> Any | None:
    if not _use_rust():
        return None
    _add_local_extensions()
    try:
        import maze_shapes_gen_rs  # type: ignore[import-not-found]

        return maze_shapes_gen_rs
    except ImportError:
        return None
