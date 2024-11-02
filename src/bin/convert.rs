use cases::{Case, CONFIG};
use fjall::{Config, KvSeparationOptions, PartitionCreateOptions};
use std::fs;
use tracing::info;
use tracing_subscriber::{layer::SubscriberExt as _, util::SubscriberInitExt};

fn main() {
    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::new("info,fjall=warn"))
        .with(tracing_subscriber::fmt::layer())
        .init();
    convert(CONFIG.raw_data_path.as_ref().unwrap(), &CONFIG.db);
}

fn convert(raw_path: &str, db_path: &str) {
    let time = std::time::Instant::now();
    let mut ft = Vec::with_capacity(1024);
    let mut id: u32 = 0;
    let keyspace = Config::new(db_path)
        .max_write_buffer_size(256_000_000)
        .open()
        .unwrap();
    let db = keyspace
        .open_partition(
            "cases",
            PartitionCreateOptions::default()
                .max_memtable_size(128_000_000)
                .with_kv_separation(
                    KvSeparationOptions::default()
                        .separation_threshold(750)
                        .file_target_size(256_000_000),
                ),
        )
        .unwrap();
    for subdir in fs::read_dir(raw_path).unwrap() {
        let subdir = subdir.unwrap();
        let subdir_path = subdir.path().to_str().unwrap().to_string();
        if subdir_path.ends_with(".zip") {
            info!("unzipping {}", subdir_path);
            let file = fs::File::open(&subdir_path).unwrap();
            let mut archive = zip::ZipArchive::new(file).unwrap();

            let mut buf = String::new();
            for i in 0..archive.len() {
                let file = archive.by_index(i).unwrap();
                let raw_name = file.name();
                if raw_name.ends_with(".csv") {
                    let mut rdr = csv::Reader::from_reader(file);
                    for result in rdr.deserialize() {
                        id += 1;
                        if db.contains_key(id.to_be_bytes()).unwrap() {
                            if id % 10000 == 0 {
                                info!("skipping {}", id);
                            }
                            continue;
                        }

                        let mut case: Case = result.unwrap();

                        case.full_text.split_whitespace().for_each(|word| {
                            buf.push_str("<p>");
                            buf.push_str(word);
                            buf.push_str("</p>");
                        });

                        case.full_text = buf.clone();
                        buf.clear();

                        ft.push((id, case));

                        if ft.len() >= 10240 {
                            info!("inserting {id}, time: {}", time.elapsed().as_secs());
                            let mut batch = keyspace.batch();
                            for (id, case) in ft.iter() {
                                batch.insert(
                                    &db,
                                    (*id).to_be_bytes(),
                                    bincode::serialize(case).unwrap(),
                                );
                            }
                            batch.commit().unwrap();
                            ft.clear();
                        }
                    }
                }
            }

            info!("done {}", subdir_path);
        }
    }

    if !ft.is_empty() {
        info!("inserting {id}, time: {}", time.elapsed().as_secs());
        let mut batch = keyspace.batch();
        for (id, case) in ft.iter() {
            batch.insert(&db, (*id).to_be_bytes(), bincode::serialize(case).unwrap());
        }
        batch.commit().unwrap();
        ft.clear();
    }
}
