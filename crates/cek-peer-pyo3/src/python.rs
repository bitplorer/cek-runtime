//! CPython bind of the apply-only ABI. No mint. No work at import
//! beyond registering [`PeerAbi`].

use crate::extract;
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
        let mut g = crate::map_mutex_lock(self.inner.lock()).map_err(PyRuntimeError::new_err)?;
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

    /// Apply a shared JSON document (`str` or mapping). Returns `{ receipt, kv, ui, log }`.
    ///
    /// Door A: JSON text via `json.dumps` → [`PeerAbi::apply_json`]. Unchanged.
    fn apply(&self, py: Python<'_>, request: Bound<'_, PyAny>) -> PyResult<PyObject> {
        let json = py.import_bound("json")?;
        let input: String = if let Ok(s) = request.extract::<String>() {
            s
        } else {
            json.call_method1("dumps", (request,))?.extract()?
        };
        let out = {
            let g = crate::map_mutex_lock(self.inner.lock()).map_err(PyRuntimeError::new_err)?;
            py.allow_threads(|| g.apply_json(&input))
                .map_err(PyRuntimeError::new_err)?
        };
        Ok(json.call_method1("loads", (out,))?.unbind())
    }

    /// Apply owned Ops (a sequence of `{ns, name, payload}` mappings) or an
    /// apply-request mapping `{result, profile?, unknown_op_policy?}`.
    ///
    /// Door B: extract in this hop, then [`PeerAbi::apply_request`] with the GIL
    /// released. Same Cap algebra as [`Self::apply`]. Not a JSON text round-trip.
    #[pyo3(signature = (request, *, profile = None, unknown_op_policy = None))]
    fn apply_ops(
        &self,
        py: Python<'_>,
        request: Bound<'_, PyAny>,
        profile: Option<String>,
        unknown_op_policy: Option<String>,
    ) -> PyResult<PyObject> {
        let req = extract::extract_apply_request(&request, profile, unknown_op_policy)?;
        let out = {
            let g = crate::map_mutex_lock(self.inner.lock()).map_err(PyRuntimeError::new_err)?;
            py.allow_threads(|| g.apply_request(&req))
                .map_err(PyRuntimeError::new_err)?
        };
        extract::apply_response_to_py(py, &out)
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
