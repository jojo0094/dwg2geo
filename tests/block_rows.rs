#![cfg(feature = "native-backend")]

//! One row per block in model space: an INSERT point with the block's name, insertion height, rotation and scale,
//! beside the parts it is drawn with, whether or not the block carries attributes. A block nested in another gets
//! no row of its own (its outer block is the asset). From a DWG and a DXF alike.

use acadrust::{
    BlockRecord, CadDocument, DxfVersion, Vector3,
    entities::{Circle, EntityType, Insert},
    io::{
        dwg::DwgWriter,
        dxf::{DxfReader, write_dxf},
    },
};
use serde_json::Value;
use tempfile::TempDir;

fn drawing() -> CadDocument {
    let mut document = CadDocument::with_version(DxfVersion::AC1027);
    let mut valve = BlockRecord::new("VALVE");
    valve.handle = document.allocate_handle();
    valve.base_point = Vector3::ZERO;
    let owner = valve.handle;
    document.block_records.add(valve).expect("block");
    let mut circle = EntityType::Circle(Circle::from_coords(0.0, 0.0, 0.0, 0.5));
    circle.common_mut().owner_handle = owner;
    document.add_entity(circle).expect("circle");
    let mut a = Insert::new("VALVE", Vector3::new(10.0, 20.0, 46.3));
    a.rotation = std::f64::consts::FRAC_PI_2;
    document.add_entity(EntityType::Insert(a)).expect("insert");
    document
        .add_entity(EntityType::Insert(Insert::new("VALVE", Vector3::new(30.0, 20.0, 45.9))))
        .expect("insert");
    document
}

fn inserts(bytes: &[u8]) -> Vec<Value> {
    let result = dwg2geo::backend::native::convert_bytes(bytes, true, None).expect("converts");
    let value: Value = serde_json::from_str(&result.geojson).expect("GeoJSON");
    let mut rows: Vec<Value> = value["features"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|f| f["properties"]["entity_type"] == "INSERT")
        .cloned()
        .collect();
    rows.sort_by(|a, b| a["properties"]["z"].as_f64().partial_cmp(&b["properties"]["z"].as_f64()).unwrap());
    rows
}

#[test]
fn every_block_in_model_space_is_a_row() {
    let dir = TempDir::new().expect("temp dir");
    let dxf = dir.path().join("b.dxf");
    write_dxf(&drawing(), &dxf).expect("dxf");
    let dwg = dir.path().join("b.dwg");
    let reread = DxfReader::from_file(&dxf).expect("open").read().expect("read");
    DwgWriter::write_to_file(&dwg, &reread).expect("dwg");

    for (label, path) in [("DXF", &dxf), ("DWG", &dwg)] {
        let rows = inserts(&std::fs::read(path).unwrap());
        assert_eq!(rows.len(), 2, "{label}: one row per insert");
        let p = &rows[1]["properties"];
        assert_eq!(p["block_name"], "VALVE", "{label}");
        assert_eq!(p["z"].as_f64(), Some(46.3), "{label}");
        assert!((p["rotation_deg"].as_f64().unwrap() - 90.0).abs() < 1e-9, "{label}");
        assert_eq!(p["x_scale"].as_f64(), Some(1.0), "{label}");
        assert_eq!(rows[1]["geometry"]["coordinates"], serde_json::json!([10.0, 20.0]), "{label}");
    }
}
