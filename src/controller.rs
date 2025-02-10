use askama::Template;
use axum::{
    body::Body,
    extract::{Path, Query, State},
    http::{self, header, Response, StatusCode},
    response::IntoResponse,
};
use bincode::config::standard;
use indexmap::IndexSet;
use serde::Deserialize;
use tantivy::{
    collector::{Count, TopDocs},
    schema::Value,
    DocAddress, Score, TantivyDocument,
};
use tracing::info;

use crate::{remove_html_tags, AppState, Case};

#[derive(Template)]
#[template(path = "case.html", escape = "none")]
pub struct CasePage {
    case: Case,
}

pub async fn case(State(state): State<AppState>, Path(id): Path<u32>) -> impl IntoResponse {
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
    search_type: Option<String>,
    export: Option<bool>,
}

#[derive(Template)]
#[template(path = "search.html")]
pub struct SearchPage {
    search: String,
    offset: usize,
    total: usize,
    cases: Vec<(u32, String, Case)>,
    search_type: String,
}

pub async fn search(
    Query(input): Query<QuerySearch>,
    State(state): State<AppState>,
) -> impl IntoResponse {
    let offset = input.offset.unwrap_or_default();
    let search = input.search.unwrap_or_default();
    let export = input.export.unwrap_or_default();
    let limit = if export { 10000 } else { 20 };
    let mut ids: IndexSet<u32> = IndexSet::with_capacity(20);
    let mut total = 0;
    if !search.is_empty() {
        info!("searching: {}", search);
        let query = match input.search_type.as_deref() {
            Some("legal_basis") => format!("legal_basis:{}", search),
            Some("cause") => format!("cause:{}", search),
            _ => search.clone(),
        };
        let (query, _) = state.searcher.query_parser.parse_query_lenient(&query);
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
                .as_str()
            {
                ids.insert(id.parse().unwrap());
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
        let fname = format!("{}_{}_{}_{}.csv", search, total, limit, offset);
        let body = Vec::new();
        let mut wtr = csv::Writer::from_writer(body);
        wtr.write_record([
            "id",
            "url",
            "case_id",
            "case_name",
            "court",
            "region",
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
                &case.region,
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
                &format!("attachment; filename={}", fname),
            ),
        ];
        return (headers, wtr.into_inner().unwrap()).into_response();
    }

    let search_type = input.search_type.unwrap_or_else(|| "default".to_string());
    let body = SearchPage {
        search,
        offset,
        cases,
        total,
        search_type,
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

fn into_response<T: Template>(t: &T) -> Response<Body> {
    match t.render() {
        Ok(body) => {
            let headers = [(
                http::header::CONTENT_TYPE,
                http::HeaderValue::from_static(T::MIME_TYPE),
            )];

            (headers, body).into_response()
        }
        Err(_) => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
    }
}
