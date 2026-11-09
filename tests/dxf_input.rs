#![cfg(feature = "native-backend")]

//! DXF input: `convert_bytes` reads a DXF (ASCII or binary) into the same
//! document model as a DWG, so the same drawing converts to the same features
//! whichever format it was saved in.

use acadrust::{
    CadDocument, DxfVersion,
    entities::{Circle, EntityType, Line, Point},
    io::{
        dwg::DwgWriter,
        dxf::{write_binary_dxf, write_dxf},
    },
};
use serde_json::Value;
use tempfile::TempDir;

fn drawing() -> CadDocument {
    let mut document = CadDocument::with_version(DxfVersion::AC1027);
    document
        .add_entity(EntityType::Point(Point::from_coords(5.0, 6.0, 0.0)))
        .expect("point");
    document
        .add_entity(EntityType::Line(Line::from_coords(
            0.0, 0.0, 0.0, 10.0, 4.0, 0.0,
        )))
        .expect("line");
    document
        .add_entity(EntityType::Circle(Circle::from_coords(
            20.0, 20.0, 0.0, 5.0,
        )))
        .expect("circle");
    document
}

/// The features with what identifies them across formats: type, layer, geometry.
fn features(bytes: &[u8]) -> Vec<(String, String, Value)> {
    let result = dwg2geo::backend::native::convert_bytes(bytes, true, None).expect("converts");
    let value: Value = serde_json::from_str(&result.geojson).expect("valid GeoJSON");
    let mut out: Vec<(String, String, Value)> = value["features"]
        .as_array()
        .expect("features")
        .iter()
        .map(|f| {
            (
                f["properties"]["entity_type"].as_str().unwrap().to_string(),
                f["properties"]["layer"].as_str().unwrap().to_string(),
                f["geometry"].clone(),
            )
        })
        .collect();
    out.sort_by(|a, b| a.0.cmp(&b.0));
    out
}

#[test]
fn the_same_drawing_converts_the_same_from_dwg_ascii_dxf_and_binary_dxf() {
    let dir = TempDir::new().expect("temp dir");
    let document = drawing();
    let dwg_path = dir.path().join("parity.dwg");
    DwgWriter::write_to_file(&dwg_path, &document).expect("write dwg");
    let dwg = std::fs::read(&dwg_path).expect("read dwg");
    let ascii_path = dir.path().join("parity.dxf");
    write_dxf(&document, &ascii_path).expect("write ascii dxf");
    let ascii = std::fs::read(&ascii_path).expect("read ascii dxf");
    let binary_path = dir.path().join("parity-binary.dxf");
    write_binary_dxf(&document, &binary_path).expect("write binary dxf");
    let binary = std::fs::read(&binary_path).expect("read binary dxf");

    assert!(!ascii.starts_with(b"AC10") && !binary.starts_with(b"AC10"));
    let from_dwg = features(&dwg);
    assert_eq!(from_dwg.len(), 3);
    assert_eq!(
        features(&ascii),
        from_dwg,
        "ASCII DXF converts like the DWG"
    );
    assert_eq!(
        features(&binary),
        from_dwg,
        "binary DXF converts like the DWG"
    );
}

#[test]
fn garbage_is_still_refused() {
    assert!(dwg2geo::backend::native::convert_bytes(b"not a drawing at all", false, None).is_err());
    assert!(dwg2geo::backend::native::convert_bytes(b"", false, None).is_err());
}
