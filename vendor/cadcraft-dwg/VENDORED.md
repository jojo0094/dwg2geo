# cadcraft-dwg (vendored)

From [CADCraft](https://github.com/storytold/cadcraft) `crates/dwg`, rev
`2f8f1861886412f529900b37e3f80f8d1e2af119`, MIT OR Apache-2.0 (`LICENSE-MIT`, `LICENSE-APACHE`, `NOTICE`).

It turns DWG bytes into a DXF rendition with acadrust (MPL-2.0, unmodified), with CADCraft's fix-ups
(application objects, proxy entities, header variables, MTEXT contents). The JS binding uses it to give a DWG to
CADCraft's renderer (`render` in `bindings/js`).

Two changes from upstream, both so the bridge is built for `wasm32`:

- `build.rs` sets `no_dwg_bridge` only on FreeBSD aarch64 (upstream: also on wasm32).
- `Cargo.toml` takes acadrust on every target but FreeBSD aarch64, and writes out the fields upstream inherits
  from its workspace.

Everything else is upstream's, unchanged. The root `Cargo.toml` puts this copy in place of the git crate with a
`[patch]`.
