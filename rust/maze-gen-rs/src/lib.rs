//! PyO3 bindings for rectangular and triangular maze generators.

use pyo3::prelude::*;

pub mod port;

#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;

fn map_error(message: String) -> PyErr {
    use pyo3::exceptions as e;
    let (kind, text) = message.split_once(": ").unwrap_or(("", message.as_str()));
    let text = text.to_string();
    match kind {
        "TypeError" => e::PyTypeError::new_err(text),
        "KeyError" => e::PyKeyError::new_err(text),
        "IndexError" => e::PyIndexError::new_err(text),
        "OverflowError" => e::PyOverflowError::new_err(text),
        "ZeroDivisionError" => e::PyZeroDivisionError::new_err(text),
        "AttributeError" => e::PyAttributeError::new_err(text),
        "RuntimeError" => e::PyRuntimeError::new_err(text),
        "NotImplementedError" => e::PyNotImplementedError::new_err(text),
        "LookupError" => e::PyLookupError::new_err(text),
        "ArithmeticError" => e::PyArithmeticError::new_err(text),
        "UnicodeError" => e::PyUnicodeError::new_err(text),
        "ValueError" => e::PyValueError::new_err(text),
        _ => e::PyValueError::new_err(message),
    }
}

#[pyfunction]
#[pyo3(signature = (n, seed))]
fn kruskal_rect_maze(n: i64, seed: i64) -> PyResult<Vec<Vec<i64>>> {
    port::kruskal_rect_maze(n, seed).map_err(map_error)
}

#[pyfunction]
#[pyo3(signature = (n, seed))]
fn kruskal_rect_edges(n: i64, seed: i64) -> PyResult<Vec<Vec<i64>>> {
    port::kruskal_rect_edges(n, seed).map_err(map_error)
}

#[pyfunction]
#[pyo3(signature = (n, seed))]
fn prims_square_maze(n: i64, seed: i64) -> PyResult<Vec<Vec<i64>>> {
    port::prims_square_maze(n, seed).map_err(map_error)
}

#[pyfunction]
#[pyo3(signature = (num_levels, seed))]
fn kruskal_triangular_maze(num_levels: i64, seed: i64) -> PyResult<Vec<Vec<i64>>> {
    port::kruskal_triangular_maze(num_levels, seed).map_err(map_error)
}

#[pyfunction]
#[pyo3(signature = (num_levels, seed))]
fn dfs_triangular_maze(num_levels: i64, seed: i64) -> PyResult<Vec<String>> {
    port::dfs_triangular_maze(num_levels, seed).map_err(map_error)
}

#[pyfunction]
#[pyo3(signature = (seed, n, algorithm))]
fn generate_maze_packed(seed: i64, n: i64, algorithm: String) -> PyResult<String> {
    port::generate_maze_packed(seed, n, algorithm).map_err(map_error)
}

/// Multi-core batch generator holding a seed list in Rust (releases GIL via `py.detach`).
#[pyclass]
pub struct GenerateMazePackedBatch {
    inner: port::GenerateMazePackedIndex,
}

#[pymethods]
impl GenerateMazePackedBatch {
    #[new]
    fn new(items: Vec<i64>) -> Self {
        Self {
            inner: port::GenerateMazePackedIndex::new(items),
        }
    }

    fn run(&self, py: Python<'_>, n: i64, algorithm: String) -> PyResult<Vec<String>> {
        py.detach(|| self.inner.run(n, algorithm))
            .map_err(map_error)
    }
}

#[pymodule]
#[pyo3(name = "_native")]
fn native_module(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(kruskal_rect_maze, m)?)?;
    m.add_function(wrap_pyfunction!(kruskal_rect_edges, m)?)?;
    m.add_function(wrap_pyfunction!(prims_square_maze, m)?)?;
    m.add_function(wrap_pyfunction!(kruskal_triangular_maze, m)?)?;
    m.add_function(wrap_pyfunction!(dfs_triangular_maze, m)?)?;
    m.add_function(wrap_pyfunction!(generate_maze_packed, m)?)?;
    m.add_class::<GenerateMazePackedBatch>()?;
    Ok(())
}
