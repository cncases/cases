use std::path::Path;

use bincode::config::standard;
use cases::{CONFIG, Case, Tan, kv_sep_partition_option, remove_html_tags};
use fjall::Config;
use tantivy::TantivyDocument;
use tracing::info;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[cfg(not(target_os = "windows"))]
#[global_allocator]
static GLOBAL: tikv_jemallocator::Jemalloc = tikv_jemallocator::Jemalloc;

fn main() {
    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::new(
            "info,tantivy=warn,html5ever=error",
        ))
        .with(tracing_subscriber::fmt::layer())
        .init();

    let schema = Tan::schema();

    let id_field = schema.get_field("id").unwrap();
    let case_id = schema.get_field("case_id").unwrap();
    let case_name = schema.get_field("case_name").unwrap();
    let court = schema.get_field("court").unwrap();
    let case_type = schema.get_field("case_type").unwrap();
    let procedure = schema.get_field("procedure").unwrap();
    let year = schema.get_field("year").unwrap();
    let month = schema.get_field("month").unwrap();
    let day = schema.get_field("day").unwrap();
    let judgment_date = schema.get_field("judgment_date").unwrap();
    let public_date = schema.get_field("public_date").unwrap();
    let parties = schema.get_field("parties").unwrap();
    let cause = schema.get_field("cause").unwrap();
    let legal_basis = schema.get_field("legal_basis").unwrap();
    let full_text = schema.get_field("full_text").unwrap();

    let index_path = Path::new(&CONFIG.index_path);
    if !index_path.exists() {
        std::fs::create_dir(index_path).unwrap();
    }

    let index = Tan::index().unwrap();
    let mut writer = index.writer(50 * 1024 * 1024).unwrap();

    let time = std::time::Instant::now();

    let keyspace_new = Config::new(&CONFIG.db)
        .max_write_buffer_size(256_000_000)
        .open()
        .unwrap();

    let cases_new = keyspace_new
        .open_partition("cases", kv_sep_partition_option())
        .unwrap();

    for i in cases_new.iter() {
        let (k, v) = i.unwrap();
        let id = u32::from_be_bytes(k[..].try_into().unwrap());
        let (mut case, _): (Case, _) = bincode::decode_from_slice(&v, standard()).unwrap();

        if CONFIG.index_with_full_text {
            case.full_text = remove_html_tags(&case.full_text);
        }

        let mut doc = TantivyDocument::default();
        doc.add_u64(id_field, id as u64);
        if !case.case_id.is_empty() {
            doc.add_text(case_id, &case.case_id);
        }
        if !case.case_name.is_empty() {
            doc.add_text(case_name, &case.case_name);
        }
        if !case.court.is_empty() {
            doc.add_text(court, &case.court);
        }
        if !case.case_type.is_empty() {
            doc.add_text(case_type, &case.case_type);
        }
        if !case.procedure.is_empty() {
            doc.add_text(procedure, &case.procedure);
        }
        if !case.judgment_date.is_empty() {
            doc.add_text(judgment_date, &case.judgment_date);
            let s: Vec<&str> = case.judgment_date.split("-").collect();
            if let Some(y) = s.get(0) {
                if let Ok(judge_year) = y.parse() {
                    doc.add_u64(year, judge_year);
                }
            }
            if let Some(m) = s.get(1) {
                if let Ok(judge_month) = m.parse() {
                    doc.add_u64(month, judge_month);
                }
            }
            if let Some(d) = s.get(2) {
                if let Ok(judge_day) = d.parse() {
                    doc.add_u64(day, judge_day);
                }
            }
        }
        if !case.public_date.is_empty() {
            doc.add_text(public_date, &case.public_date);
        }
        if !case.parties.is_empty() {
            doc.add_text(parties, &case.parties);
        }
        if !case.cause.is_empty() {
            doc.add_text(cause, &case.cause);
        }
        if !case.legal_basis.is_empty() {
            doc.add_text(legal_basis, &case.legal_basis);
        }
        if CONFIG.index_with_full_text && !case.full_text.is_empty() {
            doc.add_text(full_text, &case.full_text);
        }
        writer.add_document(doc).unwrap();

        if id % 10000 == 0 {
            writer.commit().unwrap();
            info!("{} done, {}", id, time.elapsed().as_secs());
        }
    }

    writer.commit().unwrap();
}
