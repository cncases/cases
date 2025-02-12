/// convert cases to clickhouse
///
/// You need to change the clickhouse user and password.
///
/// ```bash
/// cargo build -r --example clickhouse
/// ./target/release/examples/clickhouse
/// ```
///
/// ```sql
/// CREATE TABLE
///     cases (
///         `id` UInt32,
///         `doc_id` String,
///         `case_id` Nullable(String),
///         `case_name` Nullable(String),
///         `court` Nullable(String),
///         `region` LowCardinality(Nullable(String)),
///         `case_type` LowCardinality(Nullable(String)),
///         `procedure` LowCardinality(Nullable(String)),
///         `judgment_date` LowCardinality(Nullable(String)),
///         `public_date` LowCardinality(Nullable(String)),
///         `parties` Nullable(String),
///         `cause` Nullable(String)  CODEC(ZSTD),
///         `legal_basis` Nullable(String)  CODEC(ZSTD),
///     ) ENGINE = MergeTree () PRIMARY KEY (
///         id
///     )
/// ```
use bincode::config::standard;
use cases::{kv_sep_partition_option, Case, CONFIG};
use clickhouse::{Client, Row};
use fjall::Config;
use serde::{Deserialize, Serialize};

#[tokio::main]
async fn main() {
    let time = std::time::Instant::now();
    let client = Client::default()
        .with_url("http://localhost:8123")
        .with_user("clickhouse_user")
        .with_password("clickhouse_password");

    let keyspace = Config::new(CONFIG.db.as_str()).open().unwrap();
    let db = keyspace
        .open_partition("cases", kv_sep_partition_option())
        .unwrap();

    let mut count = 0;
    let mut batch = vec![];

    for i in db.iter() {
        let (k, v) = i.unwrap();
        let id = u32::from_be_bytes(k[..].try_into().unwrap());
        let (case, _): (Case, _) = bincode::decode_from_slice(&v, standard()).unwrap();

        let legal_basis = if case.legal_basis.is_empty() || case.legal_basis == "," {
            None
        } else {
            Some(case.legal_basis)
        };

        let new_case = NewCase {
            id,
            doc_id: case.doc_id,
            case_id: (!case.case_id.is_empty()).then(|| case.case_id),
            case_name: (!case.case_name.is_empty()).then(|| case.case_name),
            court: (!case.court.is_empty()).then(|| case.court),
            region: (!case.region.is_empty()).then(|| case.region),
            case_type: (!case.case_type.is_empty()).then(|| case.case_type),
            procedure: (!case.procedure.is_empty()).then(|| case.procedure),
            judgment_date: (!case.judgment_date.is_empty()).then(|| case.judgment_date),
            public_date: (!case.public_date.is_empty()).then(|| case.public_date),
            parties: (!case.parties.is_empty()).then(|| case.parties),
            cause: (!case.cause.is_empty()).then(|| case.cause),
            legal_basis,
        };

        count += 1;
        batch.push(new_case);
        if batch.len() >= 100000 {
            let mut insert = client.insert("cases").unwrap();
            for case in batch.iter() {
                insert.write(case).await.unwrap();
            }
            insert.end().await.unwrap();
            batch.clear();
            println!("batch {count}, time: {}", time.elapsed().as_secs())
        }
    }

    let mut insert = client.insert("cases").unwrap();
    for case in batch.iter() {
        insert.write(case).await.unwrap();
    }
    insert.end().await.unwrap();
    batch.clear();
    println!("Done, time: {}", time.elapsed().as_secs())
}

#[derive(Debug, Row, Serialize, Deserialize)]
struct NewCase {
    id: u32,
    doc_id: String,
    case_id: Option<String>,
    case_name: Option<String>,
    court: Option<String>,
    region: Option<String>,
    case_type: Option<String>,
    procedure: Option<String>,
    judgment_date: Option<String>,
    public_date: Option<String>,
    parties: Option<String>,
    cause: Option<String>,
    legal_basis: Option<String>,
}
