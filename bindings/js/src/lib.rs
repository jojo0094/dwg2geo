//! WebAssembly bindings for `dwg2geo` — the npm package `dwg2geo`.
//!
//! ```js
//! import init, { convert } from 'dwg2geo';
//! await init();
//! const result = convert(new Uint8Array(dwgBytes), false, undefined);
//! const fc = JSON.parse(result.geojson); // FeatureCollection, local coords
//! const drawn = render(new Uint8Array(dwgBytes)); // the drawing as drawn: lines and triangles for a GPU
//! ```

use wasm_bindgen::prelude::*;

/// Convert DWG or DXF bytes (ASCII or binary DXF; the format is sniffed from
/// the bytes) to a GeoJSON FeatureCollection (local drawing coordinates) plus a
/// conversion report. Returns a JS object shaped like
/// `dwg2geo::backend::native::EmbedResult`: `{ geojson, feature_count,
/// model_space_entities, converted, skipped, failed, warnings, bbox,
/// source_sha256, geodata, crs_text_hints }`. Reprojection is the caller's
/// responsibility (e.g. proj4js) — this function never guesses a CRS. What
/// it does surface is every CRS *declaration* the drawing carries: the
/// GEODATA object (`geodata`, or `null`) and text strings from any space
/// that mention a CRS keyword (`crs_text_hints`, e.g. a title block's
/// "UTM - SIRGAS 2000 - FUSO 25 SUL"), for the caller to interpret.
#[wasm_bindgen]
pub fn convert(
    bytes: &[u8],
    polygonize_closed: bool,
    curve_tolerance: Option<f64>,
) -> Result<JsValue, JsValue> {
    console_error_panic_hook::set_once();
    let result = dwg2geo::backend::native::convert_bytes(bytes, polygonize_closed, curve_tolerance)
        .map_err(|error| JsValue::from_str(&format!("{error:#}")))?;
    serde_wasm_bindgen::to_value(&result).map_err(|error| JsValue::from_str(&error.to_string()))
}

/// The drawing AS DRAWN, for a GPU: model space through CADCraft's DXF reader and renderer (cadcraft-io,
/// cadcraft-render; MIT OR Apache-2.0), with line types, hatches, text, blocks and dimensions already turned into
/// lines and triangles. A DWG is first given a DXF rendition by CADCraft's DWG bridge (acadrust, failsafe read).
///
/// Returns `{ verts, tris, prims, notes, truncated }`, all in drawing units:
/// - `verts`: Float64Array of x, y pairs, used by line, point and infinite-line primitives;
/// - `tris`: Float64Array of x, y pairs, three per triangle (fills, filled glyphs, masks);
/// - `prims`: Uint32Array, seven per primitive, in drawing order:
///   `[handle_lo, handle_hi, rgb, flags, lineweight_um, start, len]`. `rgb` is 0xRRGGBB. `flags` is the kind
///   (0 polyline: verts[start..start+len]; 1 triangles: tris[start..start+len]; 2 mask: as 1, filled with the
///   background; 3 point: verts[start]; 4 infinite line: verts[start] its base, verts[start+1] its unit direction;
///   5 image: not drawn) plus bit 8 set for colour 7 (white on a dark background, black on a light one) and bit 9
///   for a ray. The handle is the TOP-LEVEL model-space entity's: a block's contents carry their INSERT's, so it is
///   the same handle `convert` gives that entity (the first segment of a feature's id).
/// - `notes`: what the DWG read left out; `truncated`: the renderer's block-expansion budget was spent.
#[wasm_bindgen]
pub fn render(bytes: &[u8]) -> Result<JsValue, JsValue> {
    console_error_panic_hook::set_once();
    // A DWG's DXF rendition is ~6x the file; the app caps a DWG at 25 MB, so 512 MB of DXF is a damaged read.
    let limits = cadcraft_dwg::Limits { max_dwg_bytes: 64 << 20, max_dxf_bytes: 512 << 20 };
    let (dxf, notes) = if cadcraft_dwg::is_dwg(bytes) {
        cadcraft_dwg::dwg_to_dxf_noted(bytes, limits).map_err(|e| JsValue::from_str(&e))?
    } else {
        (bytes.to_vec(), Vec::new())
    };
    let drawing = cadcraft_io::read_dxf(&dxf).map_err(|e| JsValue::from_str(&format!("{e}")))?;
    drop(dxf);
    let list = cadcraft_render::build(&drawing, &cadcraft_doc::Space::Model, &cadcraft_render::Options::default());
    drop(drawing);

    let verts: Vec<f64> = list.verts.iter().flat_map(|v| [v.x, v.y]).collect();
    let tris: Vec<f64> = list.tris.iter().flat_map(|v| [v.x, v.y]).collect();
    let mut prims: Vec<u32> = Vec::with_capacity(list.prims.len() * 7);
    for p in &list.prims {
        let (kind, ray) = match p.kind {
            cadcraft_render::Kind::Polyline => (0u32, false),
            cadcraft_render::Kind::Tris => (1, false),
            cadcraft_render::Kind::Mask => (2, false),
            cadcraft_render::Kind::Point => (3, false),
            cadcraft_render::Kind::Infinite { ray } => (4, ray),
            cadcraft_render::Kind::Image { .. } => (5, false),
        };
        let rgb = (u32::from(p.color.0) << 16) | (u32::from(p.color.1) << 8) | u32::from(p.color.2);
        let flags = kind | (u32::from(p.aci7) << 8) | (u32::from(ray) << 9);
        let lw_um = (f64::from(p.lw).max(0.0) * 1000.0).round() as u32;
        prims.extend([p.handle.0 as u32, (p.handle.0 >> 32) as u32, rgb, flags, lw_um, p.start, p.len]);
    }

    let out = js_sys::Object::new();
    let set = |k: &str, v: &JsValue| js_sys::Reflect::set(&out, &JsValue::from_str(k), v).map(|_| ());
    set("verts", &js_sys::Float64Array::from(&verts[..]).into())?;
    set("tris", &js_sys::Float64Array::from(&tris[..]).into())?;
    set("prims", &js_sys::Uint32Array::from(&prims[..]).into())?;
    let js_notes = js_sys::Array::new();
    for n in &notes {
        js_notes.push(&JsValue::from_str(n));
    }
    set("notes", &js_notes.into())?;
    set("truncated", &JsValue::from_bool(list.truncated))?;
    Ok(out.into())
}
