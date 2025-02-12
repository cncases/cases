/// generate stop words
///
/// cargo build -r --example stopwords
/// ./target/release/examples/stopwords
use jieba_rs::Jieba;
use tracing::info;

use cases::{Case, CONFIG};
use std::{
    collections::{HashMap, HashSet},
    fs::{self, read_to_string},
    io::Write,
};

fn main() {
    tracing_subscriber::fmt().init();
    unzip(CONFIG.raw_data_path.as_ref().unwrap());
}

fn unzip(path: &str) {
    let time = std::time::Instant::now();
    let mut id = 0;
    let jieba = Jieba::new();
    let mut meta_count = HashMap::new();
    let mut fulltext_count = HashMap::new();
    let stop_words = stop_words::get(stop_words::LANGUAGE::Chinese);
    let custom_stop_words = read_to_string("stopwords.txt").unwrap();
    let mut custom_stop_words: HashSet<String> = custom_stop_words
        .split_whitespace()
        .map(|x| x.to_owned())
        .collect();
    custom_stop_words.extend(stop_words);

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
                    let mut j = 0;
                    for result in rdr.deserialize() {
                        let mut case: Case = result.unwrap();
                        id += 1;
                        j += 1;
                        case.full_text =
                            case.full_text
                                .split_whitespace()
                                .fold(String::new(), |mut acc, x| {
                                    acc.push_str("<p>");
                                    acc.push_str(x);
                                    acc.push_str("</p>");
                                    acc
                                });

                        let meta = vec![
                            case.case_id,
                            case.case_name,
                            case.court,
                            case.region,
                            case.case_type,
                            case.procedure,
                            case.judgment_date,
                            case.public_date,
                            case.parties,
                            case.cause,
                            case.legal_basis,
                        ]
                        .join("\n");

                        let fulltext = case.full_text;

                        let meta_words = jieba.cut(&meta, false);
                        let fulltext_words = jieba.cut(&fulltext, false);

                        for word in meta_words {
                            if custom_stop_words.contains(word) {
                                continue;
                            }
                            let count = meta_count.entry(word.to_owned()).or_insert(0);
                            *count += 1;
                        }

                        for word in fulltext_words {
                            if custom_stop_words.contains(word) {
                                continue;
                            }
                            let count = fulltext_count.entry(word.to_owned()).or_insert(0);
                            *count += 1;
                        }

                        if j % 1000 == 0 {
                            info!("{} {} {}", id, j, time.elapsed().as_secs_f64());
                        }

                        if j > 10000 {
                            break;
                        }
                    }
                }
            }

            info!("done {id} {}", subdir_path);
        }
    }

    let mut meta_count: Vec<_> = meta_count.into_iter().collect();
    meta_count.sort_by(|a, b| b.1.cmp(&a.1));
    let mut fulltext_count: Vec<_> = fulltext_count.into_iter().collect();
    fulltext_count.sort_by(|a, b| b.1.cmp(&a.1));

    let mut meta_file = fs::File::create("meta.txt").unwrap();
    for (word, count) in meta_count {
        writeln!(meta_file, "{:05} {}", count, word).unwrap();
    }

    let mut fulltext_file = fs::File::create("fulltext.txt").unwrap();
    for (word, count) in fulltext_count {
        writeln!(fulltext_file, "{:05} {}", count, word).unwrap();
    }
}
