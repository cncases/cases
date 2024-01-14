use askama::Template;
use axum::{
    body::Body,
    extract::{Path, Query, State},
    http::{self, HeaderMap, HeaderName, HeaderValue, Response, StatusCode},
    response::IntoResponse,
};
use indexmap::IndexSet;
use serde::Deserialize;
use tantivy::{
    collector::{Count, TopDocs},
    DocAddress, Score,
};

use crate::{AppState, Case};

#[derive(Template)]
#[template(path = "case.html", escape = "none")]
pub struct CasePage {
    case: Case,
}

pub async fn case(State(state): State<AppState>, Path(id): Path<u32>) -> impl IntoResponse {
    if let Some(v) = state.db.get(id.to_be_bytes()).unwrap() {
        let case: Case = bincode::deserialize(&v).unwrap();
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
}

#[derive(Template)]
#[template(path = "search.html", escape = "none")]
pub struct SearchPage {
    search: String,
    offset: usize,
    total: usize,
    cases: Vec<(u32, Case)>,
    search_type: String,
}

pub async fn search(
    Query(input): Query<QuerySearch>,
    State(state): State<AppState>,
) -> impl IntoResponse {
    let offset = input.offset.unwrap_or_default();
    let search = input.search.unwrap_or_default();
    let mut ids: IndexSet<u32> = IndexSet::with_capacity(20);
    let mut total = 0;
    if !search.is_empty() {
        let query = match input.search_type.as_deref() {
            Some("legal_basis") => format!("legal_basis:{}", search),
            Some("cause") => format!("cause:{}", search),
            _ => search.clone(),
        };
        let (query, _) = state.searcher.query_parser.parse_query_lenient(&query);
        let searcher = state.searcher.reader.searcher();
        total = searcher.search(&query, &Count).unwrap();
        let top_docs: Vec<(Score, DocAddress)> = searcher
            .search(&query, &TopDocs::with_limit(20).and_offset(offset))
            .unwrap_or_default();

        for (_score, doc_address) in top_docs {
            if let Some(id) = searcher
                .doc(doc_address)
                .unwrap()
                .get_first(state.searcher.id)
                .unwrap()
                .as_text()
            {
                ids.insert(id.parse().unwrap());
            }
        }
    }

    let mut cases = Vec::with_capacity(ids.len());
    for id in ids {
        if let Some(v) = state.db.get(id.to_be_bytes()).unwrap() {
            let mut case: Case = bincode::deserialize(&v).unwrap();
            case.full_text = case.full_text.replace("<p>", " ").replace("</p>", " ");
            cases.push((id, case));
        }
    }

    let search_type = input.search_type.unwrap_or_else(|| "case_name".to_string());
    let body = SearchPage {
        search,
        offset,
        cases,
        total,
        search_type,
    };

    into_response(&body)
}

pub async fn style() -> (HeaderMap, &'static str) {
    let mut headers = HeaderMap::new();

    headers.insert(
        HeaderName::from_static("content-type"),
        HeaderValue::from_static("text/css"),
    );
    headers.insert(
        HeaderName::from_static("cache-control"),
        HeaderValue::from_static("public, max-age=1209600, s-maxage=86400"),
    );

    (headers, &include_str!("../static/style.css"))
}

pub async fn logo() -> (HeaderMap, &'static [u8]) {
    let mut headers = HeaderMap::new();

    headers.insert(
        HeaderName::from_static("content-type"),
        HeaderValue::from_static("image/png"),
    );
    headers.insert(
        HeaderName::from_static("cache-control"),
        HeaderValue::from_static("public, max-age=1209600, s-maxage=86400"),
    );

    (headers, &include_bytes!("../static/logo.png").as_slice())
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
