use bincode::{Decode, Encode};
pub use config::CONFIG;
pub use controller::{case, logo, search, style};
use fjall::PartitionHandle;
use serde::Deserialize;
use std::sync::Arc;
use tantivy::Searcher;
pub use tantivy::Tan;

mod config;
mod controller;
mod tantivy;

#[derive(Clone)]
pub struct AppState {
    pub db: PartitionHandle,
    pub searcher: Arc<Searcher>,
}

#[derive(Debug, Encode, Decode, Deserialize)]
pub struct Case {
    #[serde(rename(deserialize = "原始链接"))]
    pub doc_id: String,
    #[serde(rename(deserialize = "案号"))]
    pub case_id: String,
    #[serde(rename(deserialize = "案件名称"))]
    pub case_name: String,
    #[serde(rename(deserialize = "法院"))]
    pub court: String,
    #[serde(rename(deserialize = "所属地区"))]
    pub region: String,
    #[serde(rename(deserialize = "案件类型"))]
    pub case_type: String,
    // #[serde(rename(deserialize = "案件类型编码"))]
    // case_type_code: String,
    // #[serde(rename(deserialize = "来源"))]
    // source: String,
    #[serde(rename(deserialize = "审理程序"))]
    pub procedure: String,
    #[serde(rename(deserialize = "裁判日期"))]
    pub judgment_date: String,
    #[serde(rename(deserialize = "公开日期"))]
    pub public_date: String,
    #[serde(rename(deserialize = "当事人"))]
    pub parties: String,
    #[serde(rename(deserialize = "案由"))]
    pub cause: String,
    #[serde(rename(deserialize = "法律依据"))]
    pub legal_basis: String,
    #[serde(rename(deserialize = "全文"))]
    pub full_text: String,
}
