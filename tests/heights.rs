#![cfg(feature = "native-backend")]

//! Heights: the output stays 2D, but a height the drawing carries is kept as a property — `z` for a point,
//! `elevation` for a shape at one height, `z_min`/`z_max` for one that varies, and `z_start`/`z_end` at a line's two
//! ends in drawing order — from a DWG and a DXF alike. A flat entity at z = 0 gains nothing.

use acadrust::{
    CadDocument, DxfVersion,
    entities::{Circle, EntityType, Line, Point},
    io::{
        dwg::DwgWriter,
        dxf::{DxfReader, write_dxf},
    },
};
use serde_json::{Value, json};
use tempfile::TempDir;

fn drawing() -> CadDocument {
    let mut document = CadDocument::with_version(DxfVersion::AC1027);
    let entities = [
        EntityType::Point(Point::from_coords(5.0, 6.0, 3.25)),
        EntityType::Circle(Circle::from_coords(20.0, 20.0, 7.0, 5.0)),
        EntityType::Line(Line::from_coords(0.0, 0.0, 1.0, 10.0, 0.0, 4.0)),
        EntityType::Line(Line::from_coords(0.0, 5.0, 0.0, 10.0, 5.0, 0.0)),
        // drawn uphill: the order of its ends is the drawing's, not the flow's
        EntityType::Line(Line::from_coords(0.0, 9.0, 6.0, 10.0, 9.0, 2.5)),
    ];
    for entity in entities {
        document.add_entity(entity).expect("entity");
    }
    document
}

const HEIGHT_KEYS: [&str; 6] = ["z", "elevation", "z_min", "z_max", "z_start", "z_end"];

/// Each feature's type and its height properties, ordered by type then by the properties.
fn heights(bytes: &[u8]) -> Vec<(String, Value)> {
    let result = dwg2geo::backend::native::convert_bytes(bytes, true, None).expect("converts");
    let value: Value = serde_json::from_str(&result.geojson).expect("GeoJSON");
    let mut out: Vec<(String, Value)> = value["features"]
        .as_array()
        .unwrap()
        .iter()
        .map(|f| {
            let props = f["properties"].as_object().unwrap();
            let kept: serde_json::Map<String, Value> = HEIGHT_KEYS
                .iter()
                .filter_map(|k| props.get(*k).map(|v| (k.to_string(), v.clone())))
                .collect();
            (
                props["entity_type"].as_str().unwrap().to_string(),
                Value::Object(kept),
            )
        })
        .collect();
    out.sort_by(|a, b| (a.0.clone(), a.1.to_string()).cmp(&(b.0.clone(), b.1.to_string())));
    out
}

#[test]
fn heights_are_kept_as_properties_from_dwg_and_dxf() {
    let dir = TempDir::new().expect("temp dir");
    let dxf = dir.path().join("h.dxf");
    write_dxf(&drawing(), &dxf).expect("dxf");
    let dwg = dir.path().join("h.dwg");
    let reread = DxfReader::from_file(&dxf)
        .expect("open")
        .read()
        .expect("read");
    DwgWriter::write_to_file(&dwg, &reread).expect("dwg");

    let expected = vec![
        ("CIRCLE".to_string(), json!({ "elevation": 7.0 })),
        ("LINE".to_string(), json!({ "z_min": 2.5, "z_max": 6.0, "z_start": 6.0, "z_end": 2.5 })),
        ("LINE".to_string(), json!({ "z_min": 1.0, "z_max": 4.0, "z_start": 1.0, "z_end": 4.0 })),
        ("LINE".to_string(), json!({})),
        ("POINT".to_string(), json!({ "z": 3.25 })),
    ];
    assert_eq!(heights(&std::fs::read(&dxf).unwrap()), expected, "from DXF");
    assert_eq!(heights(&std::fs::read(&dwg).unwrap()), expected, "from DWG");
}
