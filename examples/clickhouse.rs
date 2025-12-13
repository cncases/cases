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
///         `case_id` Nullable(String),
///         `case_name` Nullable(String),
///         `court` Nullable(String),
///         `case_type` LowCardinality(Nullable(String)),
///         `procedure` LowCardinality(Nullable(String)),
///         `judgment_date` Nullable(Date),
///         `public_date` Nullable(Date),
///         `parties` Nullable(String),
///         `cause` Nullable(String)  CODEC(ZSTD),
///     ) ENGINE = MergeTree () PRIMARY KEY (
///         id
///     )
/// ```
use bincode::config::standard;
use cases::{CONFIG, Case, kv_sep_partition_option};
use clickhouse::{Client, Row};
use fjall::Config;
use jiff::civil::{Date, date};
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
        let judgment_date = case
            .judgment_date
            .parse::<Date>()
            .ok()
            .map(|d| d.since(date(1970, 1, 1)).unwrap().get_days() as u16);
        let public_date = case
            .public_date
            .parse::<Date>()
            .ok()
            .map(|d| d.since(date(1970, 1, 1)).unwrap().get_days() as u16);
        let new_case = NewCase {
            id,
            case_id: (!case.case_id.is_empty()).then_some(case.case_id),
            case_name: (!case.case_name.is_empty()).then_some(case.case_name),
            court: (!case.court.is_empty()).then_some(case.court),
            case_type: (!case.case_type.is_empty()).then_some(case.case_type),
            procedure: (!case.procedure.is_empty()).then_some(case.procedure),
            judgment_date,
            public_date,
            parties: (!case.parties.is_empty()).then_some(case.parties),
            cause: (!case.cause.is_empty()).then_some(case.cause),
        };

        count += 1;
        batch.push(new_case);
        if batch.len() >= 100000 {
            let mut insert = client.insert::<NewCase>("cases").await.unwrap();
            for case in batch.iter() {
                insert.write(case).await.unwrap();
            }
            insert.end().await.unwrap();
            batch.clear();
            println!("batch {count}, time: {}", time.elapsed().as_secs())
        }
    }

    let mut insert = client.insert::<NewCase>("cases").await.unwrap();
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
    case_id: Option<String>,
    case_name: Option<String>,
    court: Option<String>,
    case_type: Option<String>,
    procedure: Option<String>,
    judgment_date: Option<u16>,
    public_date: Option<u16>,
    parties: Option<String>,
    cause: Option<String>,
}
