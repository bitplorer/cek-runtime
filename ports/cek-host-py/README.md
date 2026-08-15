# Python Host — use the published package

**The Cap machine is `cek-host` on PyPI.** This directory is a historic in-tree sketch (Cap-as-dict, Ed25519, lineage). Do **not** publish it as a second Host.

PyPI **0.1.0** has no `create-app`. From the [cek-python](https://github.com/bitplorer/cek-python) tree:

```bash
git clone https://github.com/bitplorer/cek-python && cd cek-python
pip install -e ./cek-host -e ./cek-surface
python -m cek_host create-app ./hello && python ./hello/app.py
```

Start: [cek-python/START.md](https://github.com/bitplorer/cek-python/blob/main/START.md)

Law: [cek-framework](https://github.com/bitplorer/cek-framework). Reference vectors stay in `crates/cek-contract/vectors` (this repo).

To exercise the *legacy* in-tree runner (not the published Host):

```bash
python3 ports/cek-host-py/run_vectors.py crates/cek-contract/vectors
```

51 CORE vectors pass on the sketch. Product authority is `cek-host`.
