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
use std::sync::LazyLock;
use tantivy::{
    DocAddress, Score, TantivyDocument,
    collector::{Count, TopDocs},
    schema::Value,
};
use tracing::info;

#[cfg(feature = "vsearch")]
use fastembed::{EmbeddingModel, InitOptions, TextEmbedding};

#[cfg(feature = "vsearch")]
use qdrant_client::{
    Qdrant,
    qdrant::{RecommendPointsBuilder, SearchPointsBuilder, point_id::PointIdOptions},
};

use crate::{AppState, CONFIG, Case, remove_html_tags};

static EXPORT_LIMIT: LazyLock<usize> = LazyLock::new(|| CONFIG.export_limit.unwrap_or(10000));
static MAX_RESULTS: LazyLock<usize> = LazyLock::new(|| CONFIG.max_results.unwrap_or(50000));

#[derive(Template)]
#[template(path = "case.html", escape = "none")]
pub struct CasePage {
    id: u32,
    case: Case,
    enable_similar: bool,
    similar_cases: Vec<(u32, String, String)>,
}

#[cfg(feature = "vsearch")]
#[derive(Debug, Deserialize)]
pub struct QueryCase {
    #[cfg(feature = "vsearch")]
    with_similar: Option<bool>,
}

pub async fn case(
    #[cfg(feature = "vsearch")] Query(params): Query<QueryCase>,
    State(state): State<AppState>,
    Path(id): Path<u32>,
) -> impl IntoResponse {
    info!("id: {}", id);
    if let Some(v) = state.db.get(id.to_be_bytes()).unwrap() {
        let (mut case, _): (Case, _) = bincode::decode_from_slice(&v, standard()).unwrap();
        case.parties = case.parties.trim_matches(',').replace(',', "，");
        case.legal_basis = case.legal_basis.trim_matches(',').replace(',', "，");
        if let Some(pos) = case.full_text.find(r#"c_header"#)
            && let Some(start) = case.full_text[..pos].rfind("<")
        {
            case.full_text = case.full_text[start..].to_owned();
        }

        #[allow(unused_mut)]
        let mut enable_similar = false;
        #[allow(unused_mut)]
        let mut similar_cases = Vec::new();
        #[cfg(feature = "vsearch")]
        {
            let mut with_similar = params.with_similar.unwrap_or(false);
            if case.case_type != "刑事案件" {
                with_similar = false;
            } else {
                enable_similar = true;
            }

            if with_similar {
                let now = std::time::Instant::now();
                let similar_ids = similar(id, &state.qclient).await;

                for sid in similar_ids {
                    if let Some(v) = state.db.get(sid.to_be_bytes()).unwrap() {
                        let (scase, _): (Case, _) =
                            bincode::decode_from_slice(&v, standard()).unwrap();
                        similar_cases.push((sid, scase.case_name, scase.case_id));
                    }
                }
                let elapsed = now.elapsed().as_secs_f32();
                info!(
                    "similar id: {}, found {} similar cases, elapsed: {}s",
                    id,
                    similar_cases.len(),
                    elapsed
                );
            }
        }

        let case = CasePage {
            id,
            case,
            enable_similar,
            similar_cases,
        };
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
    search_type: Option<String>,
}

#[derive(Template)]
#[template(path = "search.html")]
pub struct SearchPage {
    search: String,
    offset: usize,
    total: usize,
    search_type: String,
    enable_vsearch: bool,
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
    let search_type =
        if cfg!(feature = "vsearch") && input.search_type.as_deref() == Some("vsearch") {
            "vsearch".to_owned()
        } else {
            "keyword".to_owned()
        };
    let limit = if export { *EXPORT_LIMIT } else { 20 };
    let mut ids: IndexSet<u32> = IndexSet::with_capacity(20);
    let mut total = 0;
    if !search.trim().is_empty() {
        let now = std::time::Instant::now();
        let search = fast2s::convert(&search);
        if search_type == "keyword" {
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
        } else {
            #[cfg(feature = "vsearch")]
            if search_type == "vsearch" {
                {
                    let modle = match CONFIG.embedding_model {
                        2 => EmbeddingModel::BGELargeZHV15,
                        _ => EmbeddingModel::BGESmallZHV15,
                    };

                    let mut model = TextEmbedding::try_new(
                        InitOptions::new(modle).with_show_download_progress(true),
                    )
                    .unwrap();
                    let query_vec = model.embed(vec![&search], None).unwrap();

                    let client = state.qclient;
                    let search_limit = limit + offset;
                    total = search_limit;
                    if let Ok(search_result) = client
                        .search_points(
                            SearchPointsBuilder::new(
                                &CONFIG.collection_name,
                                query_vec.into_iter().next().unwrap(),
                                search_limit as u64,
                            )
                            .with_payload(false)
                            .limit(limit as u64)
                            .offset(offset as u64),
                        )
                        .await
                    {
                        for point in &search_result.result {
                            let id = point
                                .id
                                .as_ref()
                                .unwrap()
                                .point_id_options
                                .as_ref()
                                .unwrap();
                            if let PointIdOptions::Num(id) = id {
                                ids.insert(*id as u32);
                            }
                        }
                    } else {
                        tracing::error!("Qdrant search_points failed");
                    }
                }
            }
        }

        let elapsed = now.elapsed().as_secs_f32();
        if export {
            info!(
                "export {search_type} {search}, total:{total}, offset: {offset}, limit: {limit}, elapsed: {elapsed}s"
            );
        } else {
            info!(
                "search {search_type} {search}, total:{total}, offset: {offset}, limit: {limit}, elapsed: {elapsed}s "
            );
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
                &case.parties.trim_matches(',').replace(',', "，"),
                &case.cause,
                &case.legal_basis.trim_matches(',').replace(',', "，"),
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
        search_type,
        offset,
        cases,
        total,
        enable_vsearch: cfg!(feature = "vsearch"),
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

#[cfg(feature = "vsearch")]
pub async fn similar(id: u32, qclient: &Qdrant) -> Vec<u32> {
    let mut ids = Vec::with_capacity(10);
    if let Ok(rsp) = qclient
        .recommend(RecommendPointsBuilder::new(&CONFIG.collection_name, 10).add_positive(id as u64))
        .await
    {
        for point in &rsp.result {
            if let Some(id) = point.id.as_ref().unwrap().point_id_options.as_ref()
                && let PointIdOptions::Num(id) = id
            {
                ids.push(*id as u32);
            }
        }
    } else {
        tracing::error!("Qdrant recommend {id} failed");
    }
    ids
}
