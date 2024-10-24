use cases::{Case, CONFIG, TABLE};
use redb::Database;
use std::fs;
use tracing::info;
use zstd::stream::copy_encode;

fn main() {
    tracing_subscriber::fmt().init();
    convert(CONFIG.raw_data_path.as_ref().unwrap(), &CONFIG.db);
}

fn convert(raw_path: &str, db_path: &str) {
    let time = std::time::Instant::now();
    let mut ft = Vec::with_capacity(1024);
    let mut id: u32 = 0;

    for subdir in fs::read_dir(raw_path).unwrap() {
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
                    let db = Database::create(db_path).unwrap();
                    for result in rdr.deserialize() {
                        id += 1;
                        let read_txn = db.begin_read().unwrap();
                        if let Ok(table) = read_txn.open_table(TABLE) {
                            if table.get(id).unwrap().is_some() {
                                info!("skipping {}", id);
                                continue;
                            }
                        }

                        let mut case: Case = result.unwrap();
                        case.full_text =
                            case.full_text
                                .split_whitespace()
                                .fold(String::new(), |mut acc, x| {
                                    acc.push_str("<p>");
                                    acc.push_str(x);
                                    acc.push_str("</p>");
                                    acc
                                });
                        ft.push((id, case));

                        if ft.len() >= 1024 {
                            let write_txn = db.begin_write().unwrap();
                            for (id, case) in ft.drain(..) {
                                let mut table = write_txn.open_table(TABLE).unwrap();
                                let encoded = bincode::serialize(&case).unwrap();
                                let mut compressed = Vec::new();
                                copy_encode(&encoded[..], &mut compressed, 9).unwrap();
                                table.insert(id, compressed).unwrap();
                            }
                            write_txn.commit().unwrap();
                            info!("{}, time {}", id, time.elapsed().as_secs());
                            ft.clear();
                        }
                    }
                }
            }

            info!("done {}", subdir_path);
        }
    }

    if !ft.is_empty() {
        let db = Database::create(db_path).unwrap();
        let write_txn = db.begin_write().unwrap();
        for (id, case) in ft.drain(..) {
            let mut table = write_txn.open_table(TABLE).unwrap();
            table
                .insert(id, bincode::serialize(&case).unwrap())
                .unwrap();
        }
        write_txn.commit().unwrap();
        info!("{}, time {}", id, time.elapsed().as_secs());
        ft.clear();
    }
}
