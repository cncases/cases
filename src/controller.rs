use std::sync::LazyLock;

use askama::Template;
use axum::{
    body::Body,
    extract::{Path, Query, State},
    http::{Response, StatusCode, header},
    response::{Html, IntoResponse},
};
use bincode::config::standard;
use indexmap::IndexSet;
use serde::Deserialize;
use tantivy::{
    DocAddress, Score, TantivyDocument,
    collector::{Count, TopDocs},
    schema::Value,
};
use tracing::info;

use crate::{AppState, CONFIG, Case, remove_html_tags};

static EXPORT_LIMIT: LazyLock<usize> = LazyLock::new(|| CONFIG.export_limit.unwrap_or(10000));
static MAX_RESULTS: LazyLock<usize> = LazyLock::new(|| CONFIG.max_results.unwrap_or(50000));

#[derive(Template)]
#[template(path = "case.html", escape = "none")]
pub struct CasePage {
    case: Case,
}

pub async fn case(State(state): State<AppState>, Path(id): Path<u32>) -> impl IntoResponse {
    info!("id: {}", id);
    if let Some(v) = state.db.get(id.to_be_bytes()).unwrap() {
        let (case, _): (Case, _) = bincode::decode_from_slice(&v, standard()).unwrap();
        let case = CasePage { case };
        into_response(&case)
    } else {
        (StatusCode::NOT_FOUND, "Not found").into_response()
    }
}

#[derive(Debug, Deserialize)]
pub struct QuerySearch {
    search: Option<String>,
    offset: Option<usize>,
    export: Option<bool>,
}

#[derive(Template)]
#[template(path = "search.html")]
pub struct SearchPage {
    search: String,
    offset: usize,
    total: usize,
    cases: Vec<(u32, String, Case)>,
}

pub async fn search(
    Query(input): Query<QuerySearch>,
    State(state): State<AppState>,
) -> impl IntoResponse {
    let mut offset = input.offset.unwrap_or_default();
    if offset > *MAX_RESULTS {
        offset = *MAX_RESULTS
    }
    let search = input.search.unwrap_or_default();
    let export = input.export.unwrap_or_default();
    let limit = if export { *EXPORT_LIMIT } else { 20 };
    let mut ids: IndexSet<u32> = IndexSet::with_capacity(20);
    let mut total = 0;
    if !search.is_empty() {
        if export {
            info!("exporting: {search}, offset: {offset}, limit: {limit}");
        } else {
            info!("searching: {search}, offset: {offset}, limit: {limit}");
        }
        let (query, _) = state.searcher.query_parser.parse_query_lenient(&search);
        let searcher = state.searcher.reader.searcher();
        total = searcher.search(&query, &Count).unwrap();

        let top_docs: Vec<(Score, DocAddress)> = searcher
            .search(&query, &TopDocs::with_limit(limit).and_offset(offset))
            .unwrap_or_default();

        for (_score, doc_address) in top_docs {
            if let Some(id) = searcher
                .doc::<TantivyDocument>(doc_address)
                .unwrap()
                .get_first(state.searcher.id)
                .unwrap()
                .as_u64()
            {
                ids.insert(id as u32);
            }
        }
    }

    let mut cases = Vec::with_capacity(ids.len());
    for id in ids {
        if let Some(v) = state.db.get(id.to_be_bytes()).unwrap() {
            let (case, _): (Case, _) = bincode::decode_from_slice(&v, standard()).unwrap();
            let preview = remove_html_tags(&case.full_text)
                .chars()
                .take(240)
                .collect();
            cases.push((id, preview, case));
        }
    }

    // export to csv
    if export {
        let fname = format!("{search}_{total}_{limit}_{offset}.csv");
        let body = Vec::new();
        let mut wtr = csv::Writer::from_writer(body);
        wtr.write_record([
            "id",
            "url",
            "case_id",
            "case_name",
            "court",
            "case_type",
            "procedure",
            "judgment_date",
            "public_date",
            "parties",
            "cause",
            "legal_basis",
            "full_text",
        ])
        .unwrap();
        for (id, _, case) in &cases {
            wtr.write_record([
                &id.to_string(),
                &case.doc_id,
                &case.case_id,
                &case.case_name,
                &case.court,
                &case.case_type,
                &case.procedure,
                &case.judgment_date,
                &case.public_date,
                &case.parties,
                &case.cause,
                &case.legal_basis,
                &case.full_text,
            ])
            .unwrap();
        }
        wtr.flush().unwrap();

        let headers = [
            (header::CONTENT_TYPE, "text/csv; charset=utf-8"),
            (
                header::CONTENT_DISPOSITION,
                &format!("attachment; filename={fname}"),
            ),
        ];
        return (headers, wtr.into_inner().unwrap()).into_response();
    }

    let body = SearchPage {
        search,
        offset,
        cases,
        total,
    };

    into_response(&body)
}

pub async fn style() -> impl IntoResponse {
    let headers = [
        (header::CONTENT_TYPE, "text/css"),
        (
            header::CACHE_CONTROL,
            "public, max-age=1209600, s-maxage=86400",
        ),
    ];

    (headers, include_str!("../static/style.css"))
}

pub async fn logo() -> impl IntoResponse {
    let headers = [
        (header::CONTENT_TYPE, "image/png"),
        (
            header::CACHE_CONTROL,
            "public, max-age=1209600, s-maxage=86400",
        ),
    ];

    (headers, include_bytes!("../static/logo.png").as_slice())
}

pub async fn help() -> impl IntoResponse {
    let headers = [
        (header::CONTENT_TYPE, "text/plain; charset=utf-8"),
        (
            header::CACHE_CONTROL,
            "public, max-age=1209600, s-maxage=86400",
        ),
    ];

    (headers, include_bytes!("../static/help.txt").as_slice())
}

fn into_response<T: Template>(t: &T) -> Response<Body> {
    match t.render() {
        Ok(body) => Html(body).into_response(),
        Err(_) => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
    }
}
