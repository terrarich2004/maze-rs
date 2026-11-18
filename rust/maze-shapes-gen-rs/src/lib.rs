//! PyO3 bindings for circular and hexagonal DFS maze generators.

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
#[pyo3(signature = (num_levels, seed))]
fn circular_dfs_maze(num_levels: i64, seed: i64) -> PyResult<Vec<Vec<i64>>> {
    port::circular_dfs_maze(num_levels, seed).map_err(map_error)
}

#[pyfunction]
#[pyo3(signature = (num_levels, seed))]
fn hexagonal_dfs_maze(num_levels: i64, seed: i64) -> PyResult<Vec<Vec<i64>>> {
    port::hexagonal_dfs_maze(num_levels, seed).map_err(map_error)
}

#[pyfunction]
#[pyo3(signature = (seed, num_levels, shape))]
fn generate_shape_packed(seed: i64, num_levels: i64, shape: String) -> PyResult<String> {
    port::generate_shape_packed(seed, num_levels, shape).map_err(map_error)
}

/// Multi-core batch generator holding a seed list in Rust (releases GIL via `py.detach`).
#[pyclass]
pub struct GenerateShapePackedBatch {
    inner: port::GenerateShapePackedIndex,
}

#[pymethods]
impl GenerateShapePackedBatch {
    #[new]
    fn new(items: Vec<i64>) -> Self {
        Self {
            inner: port::GenerateShapePackedIndex::new(items),
        }
    }

    fn run(&self, py: Python<'_>, num_levels: i64, shape: String) -> PyResult<Vec<String>> {
        py.detach(|| self.inner.run(num_levels, shape))
            .map_err(map_error)
    }
}

#[pymodule]
#[pyo3(name = "_native")]
fn native_module(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(circular_dfs_maze, m)?)?;
    m.add_function(wrap_pyfunction!(hexagonal_dfs_maze, m)?)?;
    m.add_function(wrap_pyfunction!(generate_shape_packed, m)?)?;
    m.add_class::<GenerateShapePackedBatch>()?;
    Ok(())
}
