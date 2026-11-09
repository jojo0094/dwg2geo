#![cfg(feature = "native-backend")]

//! XDATA: an entity's extended data reaches the feature as `xdata`, keyed by application, from a DWG and a
//! DXF alike. AutoCAD's own applications are left out.

use acadrust::{
    CadDocument, DxfVersion,
    entities::{EntityType, Line, Point},
    io::{
        dwg::DwgWriter,
        dxf::{DxfReader, write_dxf},
    },
    tables::appid::AppId,
    xdata::{ExtendedDataRecord, XDataValue},
};
use serde_json::{Value, json};
use tempfile::TempDir;

fn drawing() -> CadDocument {
    let mut document = CadDocument::with_version(DxfVersion::AC1027);
    document
        .app_ids
        .add(AppId::new("SURVEY_APP"))
        .expect("appid");
    let mut point = Point::from_coords(5.0, 6.0, 0.0);
    let mut survey = ExtendedDataRecord::new("SURVEY_APP");
    survey.add_value(XDataValue::String("MH-12".into()));
    survey.add_value(XDataValue::Real(3.5));
    survey.add_value(XDataValue::Integer16(7));
    point.common.extended_data.add_record(survey);
    let mut internal = ExtendedDataRecord::new("ACAD");
    internal.add_value(XDataValue::String("DSTYLE".into()));
    point.common.extended_data.add_record(internal);
    document
        .add_entity(EntityType::Point(point))
        .expect("point");
    document
        .add_entity(EntityType::Line(Line::from_coords(
            0.0, 0.0, 0.0, 1.0, 1.0, 0.0,
        )))
        .expect("line");
    document
}

fn by_type(bytes: &[u8]) -> Vec<(String, Value)> {
    let result = dwg2geo::backend::native::convert_bytes(bytes, true, None).expect("converts");
    let value: Value = serde_json::from_str(&result.geojson).expect("GeoJSON");
    let mut out: Vec<(String, Value)> = value["features"]
        .as_array()
        .unwrap()
        .iter()
        .map(|f| {
            (
                f["properties"]["entity_type"].as_str().unwrap().to_string(),
                f["properties"]["xdata"].clone(),
            )
        })
        .collect();
    out.sort_by(|a, b| a.0.cmp(&b.0));
    out
}

#[test]
fn xdata_reaches_the_feature_from_dwg_and_dxf() {
    let dir = TempDir::new().expect("temp dir");
    let document = drawing();
    let dxf = dir.path().join("x.dxf");
    write_dxf(&document, &dxf).expect("dxf");
    // acadrust's DWG writer emits XDATA only for records it read itself (it re-writes the raw blobs), so the
    // DWG is made from the document as read back from the DXF, the way a real DWG would carry it.
    let reread = DxfReader::from_file(&dxf)
        .expect("open dxf")
        .read()
        .expect("read dxf");
    let dwg = dir.path().join("x.dwg");
    DwgWriter::write_to_file(&dwg, &reread).expect("dwg");

    let expected = vec![
        ("LINE".to_string(), Value::Null),
        (
            "POINT".to_string(),
            json!({ "SURVEY_APP": ["MH-12", 3.5, 7] }),
        ),
    ];
    assert_eq!(by_type(&std::fs::read(&dwg).unwrap()), expected, "from DWG");
    assert_eq!(by_type(&std::fs::read(&dxf).unwrap()), expected, "from DXF");
}
