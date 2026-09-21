//! Door B: Python mapping / sequence → owned Cap types.
//!
//! Lives in this hop only. No JSON text codec. Never store a
//! live Python dict past return. Callers drop the GIL before kernel apply.

use crate::ApplyResponse;
use cek_contract::{Op, ResultKind, ResultMsg};
use cek_peer_rust::ApplyRequest;
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyo3::types::{PyDict, PyList, PyString, PyTuple};
use serde_json::{Map, Number, Value};

const MAX_JSON_DEPTH: usize = 64;

/// Build [`ApplyRequest`] from a sequence of Ops or an apply-request mapping.
pub(crate) fn extract_apply_request(
    request: &Bound<'_, PyAny>,
    profile: Option<String>,
    unknown_op_policy: Option<String>,
) -> PyResult<ApplyRequest> {
    if is_str(request) {
        return Err(PyValueError::new_err(
            "apply_ops expected a sequence of ops or an apply-request mapping, not str \
             (JSON text is apply / Door A)",
        ));
    }
    if is_mapping(request)? {
        return extract_request_mapping(request, profile, unknown_op_policy);
    }
    if is_ops_sequence(request)? {
        let ops = extract_ops(request, "ops")?;
        let mut req = crate::apply_request_from_ops(ops, None, None, ResultKind::Ok, None);
        if profile.is_some() {
            req.profile = profile;
        }
        if unknown_op_policy.is_some() {
            req.unknown_op_policy = unknown_op_policy;
        }
        return Ok(req);
    }
    Err(PyValueError::new_err(format!(
        "apply_ops expected a sequence of ops or an apply-request mapping, got {}",
        type_name(request)?
    )))
}

/// `{ receipt, kv, ui, log }` from an owned response (no JSON string).
pub(crate) fn apply_response_to_py(py: Python<'_>, resp: &ApplyResponse) -> PyResult<PyObject> {
    let value = serde_json::to_value(resp)
        .map_err(|e| PyValueError::new_err(format!("apply response: {e}")))?;
    value_to_py(py, &value, "response")
}

fn extract_request_mapping(
    obj: &Bound<'_, PyAny>,
    profile: Option<String>,
    unknown_op_policy: Option<String>,
) -> PyResult<ApplyRequest> {
    if mapping_contains(obj, "ns")?
        && mapping_contains(obj, "name")?
        && !mapping_contains(obj, "result")?
    {
        return Err(PyValueError::new_err(
            "apply_ops got a single op mapping; wrap it in a list, or pass an \
             apply-request mapping with result",
        ));
    }
    let Some(result_obj) = mapping_get(obj, "result")? else {
        return Err(PyValueError::new_err(
            "apply-request mapping missing result",
        ));
    };
    let result = extract_result(&result_obj, "result")?;
    let mut req = ApplyRequest {
        result,
        profile: optional_str(obj, "profile", "profile")?,
        unknown_op_policy: optional_str(obj, "unknown_op_policy", "unknown_op_policy")?,
    };
    if profile.is_some() {
        req.profile = profile;
    }
    if unknown_op_policy.is_some() {
        req.unknown_op_policy = unknown_op_policy;
    }
    Ok(req)
}

fn extract_result(obj: &Bound<'_, PyAny>, path: &str) -> PyResult<ResultMsg> {
    if !is_mapping(obj)? {
        return Err(PyValueError::new_err(format!("{path} must be a mapping")));
    }
    let kind = extract_kind(&required_get(obj, "kind", path)?, &format!("{path}.kind"))?;
    let ops = match mapping_get(obj, "ops")? {
        Some(o) => extract_ops(&o, &format!("{path}.ops"))?,
        None => Vec::new(),
    };
    Ok(ResultMsg {
        kind,
        ops,
        error: optional_str(obj, "error", &format!("{path}.error"))?,
        digest: optional_str(obj, "digest", &format!("{path}.digest"))?,
    })
}

fn extract_kind(obj: &Bound<'_, PyAny>, path: &str) -> PyResult<ResultKind> {
    let s = extract_py_str(obj, path)?;
    match s.as_str() {
        "ok" => Ok(ResultKind::Ok),
        "authority_refusal" => Ok(ResultKind::AuthorityRefusal),
        "dispatch_error" => Ok(ResultKind::DispatchError),
        other => Err(PyValueError::new_err(format!(
            "{path} must be ok, authority_refusal, or dispatch_error, got {other:?}"
        ))),
    }
}

fn extract_ops(obj: &Bound<'_, PyAny>, path: &str) -> PyResult<Vec<Op>> {
    if is_str(obj) || !is_ops_sequence(obj)? {
        return Err(PyValueError::new_err(format!(
            "{path} must be a sequence of op mappings"
        )));
    }
    let n = obj.len()?;
    let mut ops = Vec::with_capacity(n);
    for i in 0..n {
        let item = obj.get_item(i)?;
        ops.push(extract_op(&item, &format!("{path}[{i}]"))?);
    }
    Ok(ops)
}

fn extract_op(obj: &Bound<'_, PyAny>, path: &str) -> PyResult<Op> {
    if !is_mapping(obj)? {
        return Err(PyValueError::new_err(format!("{path} must be a mapping")));
    }
    let ns = required_str(obj, "ns", path)?;
    let name = required_str(obj, "name", path)?;
    let payload = match mapping_get(obj, "payload")? {
        Some(p) => py_to_value(&p, &format!("{path}.payload"), 0)?,
        None => Value::Null,
    };
    Ok(Op { ns, name, payload })
}

fn py_to_value(obj: &Bound<'_, PyAny>, path: &str, depth: usize) -> PyResult<Value> {
    if depth > MAX_JSON_DEPTH {
        return Err(PyValueError::new_err(format!(
            "{path} exceeds max JSON depth {MAX_JSON_DEPTH}"
        )));
    }
    if obj.is_none() {
        return Ok(Value::Null);
    }
    if let Ok(b) = obj.extract::<bool>() {
        return Ok(Value::Bool(b));
    }
    if let Ok(i) = obj.extract::<i64>() {
        return Ok(Value::Number(i.into()));
    }
    if let Ok(u) = obj.extract::<u64>() {
        return Ok(Value::Number(u.into()));
    }
    if let Ok(f) = obj.extract::<f64>() {
        let n = Number::from_f64(f)
            .ok_or_else(|| PyValueError::new_err(format!("{path} is not a finite JSON number")))?;
        return Ok(Value::Number(n));
    }
    if let Ok(s) = obj.extract::<String>() {
        return Ok(Value::String(s));
    }
    if is_mapping(obj)? {
        return mapping_to_value(obj, path, depth);
    }
    if is_ops_sequence(obj)? {
        let n = obj.len()?;
        let mut arr = Vec::with_capacity(n);
        for i in 0..n {
            let item = obj.get_item(i)?;
            arr.push(py_to_value(&item, &format!("{path}[{i}]"), depth + 1)?);
        }
        return Ok(Value::Array(arr));
    }
    Err(PyValueError::new_err(format!(
        "cannot extract {path} as JSON (type {})",
        type_name(obj)?
    )))
}

fn mapping_to_value(obj: &Bound<'_, PyAny>, path: &str, depth: usize) -> PyResult<Value> {
    let mut map = Map::new();
    if let Ok(dict) = obj.downcast::<PyDict>() {
        for (k, v) in dict.iter() {
            let key: String = k
                .extract()
                .map_err(|_| PyValueError::new_err(format!("{path} keys must be str")))?;
            map.insert(
                key.clone(),
                py_to_value(&v, &format!("{path}.{key}"), depth + 1)?,
            );
        }
        return Ok(Value::Object(map));
    }
    let items = obj.call_method0("items")?;
    for pair in items.iter()? {
        let pair = pair?;
        let key: String = pair
            .get_item(0)?
            .extract()
            .map_err(|_| PyValueError::new_err(format!("{path} keys must be str")))?;
        let v = pair.get_item(1)?;
        map.insert(
            key.clone(),
            py_to_value(&v, &format!("{path}.{key}"), depth + 1)?,
        );
    }
    Ok(Value::Object(map))
}

fn value_to_py(py: Python<'_>, value: &Value, path: &str) -> PyResult<PyObject> {
    match value {
        Value::Null => Ok(py.None()),
        Value::Bool(b) => Ok(b.into_py(py)),
        Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                Ok(i.into_py(py))
            } else if let Some(u) = n.as_u64() {
                Ok(u.into_py(py))
            } else if let Some(f) = n.as_f64() {
                Ok(f.into_py(py))
            } else {
                Err(PyValueError::new_err(format!(
                    "{path} number is not representable in Python"
                )))
            }
        }
        Value::String(s) => Ok(s.into_py(py)),
        Value::Array(items) => {
            let list = PyList::empty_bound(py);
            for (i, item) in items.iter().enumerate() {
                list.append(value_to_py(py, item, &format!("{path}[{i}]"))?)?;
            }
            Ok(list.into_py(py))
        }
        Value::Object(map) => {
            let dict = PyDict::new_bound(py);
            for (k, v) in map {
                dict.set_item(k, value_to_py(py, v, &format!("{path}.{k}"))?)?;
            }
            Ok(dict.into_py(py))
        }
    }
}

fn is_str(obj: &Bound<'_, PyAny>) -> bool {
    obj.is_instance_of::<PyString>()
}

fn is_mapping(obj: &Bound<'_, PyAny>) -> PyResult<bool> {
    if obj.downcast::<PyDict>().is_ok() {
        return Ok(true);
    }
    let mapping = obj
        .py()
        .import_bound("collections.abc")?
        .getattr("Mapping")?;
    obj.is_instance(&mapping)
}

fn is_ops_sequence(obj: &Bound<'_, PyAny>) -> PyResult<bool> {
    if is_str(obj) {
        return Ok(false);
    }
    if obj.downcast::<PyList>().is_ok() || obj.downcast::<PyTuple>().is_ok() {
        return Ok(true);
    }
    let seq = obj
        .py()
        .import_bound("collections.abc")?
        .getattr("Sequence")?;
    obj.is_instance(&seq)
}

fn mapping_contains(obj: &Bound<'_, PyAny>, key: &str) -> PyResult<bool> {
    obj.contains(key)
}

fn mapping_get<'py>(obj: &Bound<'py, PyAny>, key: &str) -> PyResult<Option<Bound<'py, PyAny>>> {
    if !obj.contains(key)? {
        return Ok(None);
    }
    Ok(Some(obj.get_item(key)?))
}

fn required_get<'py>(
    obj: &Bound<'py, PyAny>,
    key: &str,
    path: &str,
) -> PyResult<Bound<'py, PyAny>> {
    mapping_get(obj, key)?.ok_or_else(|| PyValueError::new_err(format!("{path} missing {key}")))
}

fn required_str(obj: &Bound<'_, PyAny>, key: &str, path: &str) -> PyResult<String> {
    extract_py_str(&required_get(obj, key, path)?, &format!("{path}.{key}"))
}

fn optional_str(obj: &Bound<'_, PyAny>, key: &str, path: &str) -> PyResult<Option<String>> {
    match mapping_get(obj, key)? {
        None => Ok(None),
        Some(v) if v.is_none() => Ok(None),
        Some(v) => Ok(Some(extract_py_str(&v, path)?)),
    }
}

fn extract_py_str(obj: &Bound<'_, PyAny>, path: &str) -> PyResult<String> {
    obj.extract::<String>()
        .map_err(|_| PyValueError::new_err(format!("{path} must be str")))
}

fn type_name(obj: &Bound<'_, PyAny>) -> PyResult<String> {
    obj.get_type().name().map(|s| s.to_string())
}
