/// Convert the cases database to parquet format
///
/// cargo build -r --example parquet
/// ./target/release/examples/parquet
use std::{fs::File, sync::Arc};

use arrow::{
    array::{ArrayRef, RecordBatch, StringArray, UInt32Array},
    datatypes::{DataType, Field, Schema},
};
use bincode::config::standard;
use cases::{kv_sep_partition_option, Case, CONFIG};
use fjall::Config;
use parquet::{arrow::ArrowWriter, basic::Compression, file::properties::WriterProperties};

const ROW_SIZE: usize = 200_000;

fn main() {
    let keyspace = Config::new(CONFIG.db.as_str()).open().unwrap();
    let db = keyspace
        .open_partition("cases", kv_sep_partition_option())
        .unwrap();

    let schema = Arc::new(Schema::new(vec![
        Field::new("id", DataType::UInt32, false),
        Field::new("doc_id", DataType::Utf8, false),
        Field::new("case_id", DataType::Utf8, false),
        Field::new("case_name", DataType::Utf8, false),
        Field::new("court", DataType::Utf8, false),
        Field::new("region", DataType::Utf8, false),
        Field::new("procedure", DataType::Utf8, false),
        Field::new("judgment_date", DataType::Utf8, false),
        Field::new("public_date", DataType::Utf8, false),
        Field::new("parties", DataType::Utf8, false),
        Field::new("cause", DataType::Utf8, false),
        Field::new("legal_basis", DataType::Utf8, false),
        Field::new("full_text", DataType::Utf8, false),
    ]));

    let mut id_vec = Vec::with_capacity(ROW_SIZE);
    let mut doc_id_vec = Vec::with_capacity(ROW_SIZE);
    let mut case_id_vec = Vec::with_capacity(ROW_SIZE);
    let mut case_name_vec = Vec::with_capacity(ROW_SIZE);
    let mut court_vec = Vec::with_capacity(ROW_SIZE);
    let mut region_vec = Vec::with_capacity(ROW_SIZE);
    let mut procedure_vec = Vec::with_capacity(ROW_SIZE);
    let mut judgment_date_vec = Vec::with_capacity(ROW_SIZE);
    let mut public_date_vec = Vec::with_capacity(ROW_SIZE);
    let mut parties_vec = Vec::with_capacity(ROW_SIZE);
    let mut cause_vec = Vec::with_capacity(ROW_SIZE);
    let mut legal_basis_vec = Vec::with_capacity(ROW_SIZE);
    let mut full_text_vec = Vec::with_capacity(ROW_SIZE);

    let props = WriterProperties::builder()
        .set_compression(Compression::LZ4)
        .set_write_batch_size(8192)
        .set_data_page_row_count_limit(ROW_SIZE)
        .build();

    let mut count = 0;
    for i in db.iter() {
        let (k, v) = i.unwrap();
        let id = u32::from_be_bytes(k[..].try_into().unwrap());
        let (case, _): (Case, _) = bincode::decode_from_slice(&v, standard()).unwrap();
        id_vec.push(id);
        doc_id_vec.push(case.doc_id);
        case_id_vec.push(case.case_id);
        case_name_vec.push(case.case_name);
        court_vec.push(case.court);
        region_vec.push(case.region);
        procedure_vec.push(case.procedure);
        judgment_date_vec.push(case.judgment_date);
        public_date_vec.push(case.public_date);
        parties_vec.push(case.parties);
        cause_vec.push(case.cause);
        legal_basis_vec.push(case.legal_basis);
        full_text_vec.push(case.full_text);

        if id_vec.len() >= ROW_SIZE {
            count += 1;
            let id_array = UInt32Array::from(id_vec.clone());
            let doc_id_array = StringArray::from(doc_id_vec.clone());
            let case_id_array = StringArray::from(case_id_vec.clone());
            let case_name_array = StringArray::from(case_name_vec.clone());
            let court_array = StringArray::from(court_vec.clone());
            let region_array = StringArray::from(region_vec.clone());
            let procedure_array = StringArray::from(procedure_vec.clone());
            let judgment_date_array = StringArray::from(judgment_date_vec.clone());
            let public_date_array = StringArray::from(public_date_vec.clone());
            let parties_array = StringArray::from(parties_vec.clone());
            let cause_array = StringArray::from(cause_vec.clone());
            let legal_basis_array = StringArray::from(legal_basis_vec.clone());
            let full_text_array = StringArray::from(full_text_vec.clone());

            let batch = RecordBatch::try_new(
                schema.clone(),
                vec![
                    Arc::new(id_array) as ArrayRef,
                    Arc::new(doc_id_array) as ArrayRef,
                    Arc::new(case_id_array) as ArrayRef,
                    Arc::new(case_name_array) as ArrayRef,
                    Arc::new(court_array) as ArrayRef,
                    Arc::new(region_array) as ArrayRef,
                    Arc::new(procedure_array) as ArrayRef,
                    Arc::new(judgment_date_array) as ArrayRef,
                    Arc::new(public_date_array) as ArrayRef,
                    Arc::new(parties_array) as ArrayRef,
                    Arc::new(cause_array) as ArrayRef,
                    Arc::new(legal_basis_array) as ArrayRef,
                    Arc::new(full_text_array) as ArrayRef,
                ],
            )
            .unwrap();

            let file_name = format!("cases_{}.parquet", count);
            println!("Writing {}", file_name);
            let file = File::create(file_name).unwrap();
            let mut writer =
                ArrowWriter::try_new(&file, batch.schema(), Some(props.clone())).unwrap();
            writer.write(&batch).expect("Writing batch");
            writer.close().unwrap();

            id_vec.clear();
            doc_id_vec.clear();
            case_id_vec.clear();
            case_name_vec.clear();
            court_vec.clear();
            region_vec.clear();
            procedure_vec.clear();
            judgment_date_vec.clear();
            public_date_vec.clear();
            parties_vec.clear();
            cause_vec.clear();
            legal_basis_vec.clear();
            full_text_vec.clear();
        }
    }

    if !id_vec.is_empty() {
        count += 1;
        let id_array = UInt32Array::from(id_vec);
        let doc_id_array = StringArray::from(doc_id_vec);
        let case_id_array = StringArray::from(case_id_vec);
        let case_name_array = StringArray::from(case_name_vec);
        let court_array = StringArray::from(court_vec);
        let region_array = StringArray::from(region_vec);
        let procedure_array = StringArray::from(procedure_vec);
        let judgment_date_array = StringArray::from(judgment_date_vec);
        let public_date_array = StringArray::from(public_date_vec);
        let parties_array = StringArray::from(parties_vec);
        let cause_array = StringArray::from(cause_vec);
        let legal_basis_array = StringArray::from(legal_basis_vec);
        let full_text_array = StringArray::from(full_text_vec);

        let batch = RecordBatch::try_new(
            schema.clone(),
            vec![
                Arc::new(id_array) as ArrayRef,
                Arc::new(doc_id_array) as ArrayRef,
                Arc::new(case_id_array) as ArrayRef,
                Arc::new(case_name_array) as ArrayRef,
                Arc::new(court_array) as ArrayRef,
                Arc::new(region_array) as ArrayRef,
                Arc::new(procedure_array) as ArrayRef,
                Arc::new(judgment_date_array) as ArrayRef,
                Arc::new(public_date_array) as ArrayRef,
                Arc::new(parties_array) as ArrayRef,
                Arc::new(cause_array) as ArrayRef,
                Arc::new(legal_basis_array) as ArrayRef,
                Arc::new(full_text_array) as ArrayRef,
            ],
        )
        .unwrap();

        let file_name = format!("cases_{}.parquet", count);
        println!("Writing {}", file_name);
        let file = File::create(file_name).unwrap();
        let mut writer = ArrowWriter::try_new(&file, batch.schema(), Some(props)).unwrap();
        writer.write(&batch).expect("Writing batch");
        writer.close().unwrap();
    }

    println!("Done");
}
