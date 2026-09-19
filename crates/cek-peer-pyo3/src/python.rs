//! CPython bind of the apply-only ABI. No mint. No work at import
//! beyond registering [`PeerAbi`].

use crate::PeerAbi;
use pyo3::exceptions::PyRuntimeError;
use pyo3::prelude::*;
use pyo3::types::PyModule;
use std::sync::Mutex;

/// Python handle: construct → bind → apply → release.
#[pyclass(name = "PeerAbi", module = "cek_peer_pyo3")]
pub(crate) struct PyPeerAbi {
    inner: Mutex<PeerAbi>,
}

impl PyPeerAbi {
    fn with_inner<F, T>(&self, f: F) -> PyResult<T>
    where
        F: FnOnce(&mut PeerAbi) -> Result<T, String>,
    {
        let mut g = self
            .inner
            .lock()
            .map_err(|_| PyRuntimeError::new_err("released"))?;
        f(&mut g).map_err(PyRuntimeError::new_err)
    }
}

#[pymethods]
impl PyPeerAbi {
    /// Allocate a handle. No kernel work.
    #[staticmethod]
    fn construct() -> Self {
        Self {
            inner: Mutex::new(PeerAbi::construct()),
        }
    }

    /// Bind this handle in the current interpreter. Not Host Cap bind.
    fn bind(&self) -> PyResult<()> {
        self.with_inner(|abi| abi.bind())
    }

    /// Apply a wasm JSON document (`str` or mapping). Returns `{ receipt, kv, ui, log }`.
    fn apply(&self, py: Python<'_>, request: Bound<'_, PyAny>) -> PyResult<PyObject> {
        let json = py.import_bound("json")?;
        let input: String = if let Ok(s) = request.extract::<String>() {
            s
        } else {
            json.call_method1("dumps", (request,))?.extract()?
        };
        let out = {
            let g = self
                .inner
                .lock()
                .map_err(|_| PyRuntimeError::new_err("released"))?;
            py.allow_threads(|| g.apply_json(&input))
                .map_err(PyRuntimeError::new_err)?
        };
        Ok(json.call_method1("loads", (out,))?.unbind())
    }

    /// The only cleanup door. Idempotent.
    fn release(&self) -> PyResult<()> {
        self.with_inner(|abi| {
            abi.release();
            Ok(())
        })
    }
}

impl Drop for PyPeerAbi {
    fn drop(&mut self) {
        if let Ok(mut g) = self.inner.lock() {
            g.release();
        }
    }
}

#[pymodule]
pub(crate) fn cek_peer_pyo3(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<PyPeerAbi>()?;
    Ok(())
}
