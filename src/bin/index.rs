use std::{fs, path::Path};

use cases::{Case, Tan, CONFIG};
use tantivy::TantivyDocument;
use tracing::info;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

fn main() {
    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::new("info,tantivy=warn"))
        .with(tracing_subscriber::fmt::layer())
        .init();

    let schema = Tan::schema();

    let id_field = schema.get_field("id").unwrap();
    let case_id = schema.get_field("case_id").unwrap();
    let case_name = schema.get_field("case_name").unwrap();
    let court = schema.get_field("court").unwrap();
    let region = schema.get_field("region").unwrap();
    let case_type = schema.get_field("case_type").unwrap();
    let procedure = schema.get_field("procedure").unwrap();
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
    let mut id: u32 = 0;

    let path = CONFIG.raw_data_path.as_ref().unwrap();
    for subdir in fs::read_dir(path).unwrap() {
        let subdir = subdir.unwrap();
        let subdir_path = subdir.path().to_str().unwrap().to_string();
        if subdir_path.ends_with(".zip") {
            info!("unzipping {}", subdir_path);
            let file = fs::File::open(&subdir_path).unwrap();
            let mut archive = zip::ZipArchive::new(file).unwrap();

            for i in 0..archive.len() {
                let file = archive.by_index(i).unwrap();
                let raw_name = file.name();
                if raw_name.ends_with(".csv") {
                    let mut rdr = csv::Reader::from_reader(file);
                    for result in rdr.deserialize() {
                        id += 1;
                        let mut case: Case = result.unwrap();
                        if CONFIG.index_with_full_text {
                            case.full_text = case.full_text.split_whitespace().fold(
                                String::new(),
                                |mut acc, x| {
                                    acc.push_str("<p>");
                                    acc.push_str(x);
                                    acc.push_str("</p>");
                                    acc
                                },
                            );
                        }

                        let mut doc = TantivyDocument::default();
                        doc.add_text(id_field, id);
                        if !case.case_id.is_empty() {
                            doc.add_text(case_id, &case.case_id);
                        }
                        if !case.case_name.is_empty() {
                            doc.add_text(case_name, &case.case_name);
                        }
                        if !case.court.is_empty() {
                            doc.add_text(court, &case.court);
                        }
                        if !case.region.is_empty() {
                            doc.add_text(region, &case.region);
                        }
                        if !case.case_type.is_empty() {
                            doc.add_text(case_type, &case.case_type);
                        }
                        if !case.procedure.is_empty() {
                            doc.add_text(procedure, &case.procedure);
                        }
                        if !case.judgment_date.is_empty() {
                            doc.add_text(judgment_date, &case.judgment_date);
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

                        if id % 1000 == 0 {
                            writer.commit().unwrap();
                            info!("{} done, {}", id, time.elapsed().as_secs());
                        }
                    }
                }
            }

            writer.commit().unwrap();

            info!("done {}", subdir_path);
        }
    }
}
