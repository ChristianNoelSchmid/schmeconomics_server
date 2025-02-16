use serde::Deserialize;

#[derive(Deserialize)]
pub struct ResponseModel {
    pub data: DataModel
}

#[derive(Deserialize)]
pub struct DataModel {
    pub mid: f64
}