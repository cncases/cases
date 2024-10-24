use cases::{Case, CONFIG};
use rocksdb::{WriteBatchWithTransaction, DB};
use std::fs;
use tracing::info;

#[cfg(not(target_env = "msvc"))]
use tikv_jemallocator::Jemalloc;

#[cfg(not(target_env = "msvc"))]
#[global_allocator]
static GLOBAL: Jemalloc = Jemalloc;

fn main() {
    tracing_subscriber::fmt().init();
    convert(CONFIG.raw_data_path.as_ref().unwrap(), &CONFIG.db);
}

fn convert(raw_path: &str, db_path: &str) {
    let time = std::time::Instant::now();
    let mut ft = Vec::with_capacity(1024);
    let mut id: u32 = 0;
    let db = DB::open_default(db_path).unwrap();
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
                    for result in rdr.deserialize() {
                        id += 1;
                        if db.key_may_exist(id.to_be_bytes()) {
                            info!("skipping {}", id);
                            continue;
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
                            info!("inserting {id}, time: {}", time.elapsed().as_secs());
                            let mut batch = WriteBatchWithTransaction::<false>::default();
                            for (id, case) in ft.iter() {
                                batch.put((*id).to_be_bytes(), bincode::serialize(case).unwrap());
                            }
                            db.write(batch).unwrap();
                            ft.clear();
                        }
                    }
                }
            }

            info!("done {}", subdir_path);
        }
    }

    if !ft.is_empty() {
        info!("inserting {id}");
        let mut batch = WriteBatchWithTransaction::<false>::default();
        for (id, case) in ft.iter() {
            batch.put((*id).to_be_bytes(), bincode::serialize(case).unwrap());
        }
        db.write(batch).unwrap();
        ft.clear();
        drop(db);
    }
}
