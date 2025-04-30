use std::collections::HashSet;

use tantivy::{
    IndexReader, ReloadPolicy,
    directory::MmapDirectory,
    query::QueryParser,
    schema::{
        Field, IndexRecordOption, NumericOptions, STORED, Schema, SchemaBuilder, TextFieldIndexing,
        TextOptions,
    },
    tokenizer::{RemoveLongFilter, StopWordFilter, TextAnalyzer},
};

use crate::CONFIG;

pub struct Searcher {
    pub reader: IndexReader,
    pub query_parser: QueryParser,
    pub id: Field,
}

pub struct Tan;
impl Tan {
    pub fn schema() -> Schema {
        let mut schema_builder = SchemaBuilder::default();

        let text_indexing = TextFieldIndexing::default()
            .set_tokenizer("jieba")
            .set_index_option(IndexRecordOption::WithFreqsAndPositions);
        let num_options = NumericOptions::default().set_indexed();
        let text_options_nostored = TextOptions::default().set_indexing_options(text_indexing);
        schema_builder.add_u64_field("id", STORED);
        schema_builder.add_text_field("case_id", text_options_nostored.clone());
        schema_builder.add_text_field("case_name", text_options_nostored.clone());
        schema_builder.add_text_field("court", text_options_nostored.clone());
        schema_builder.add_text_field("region", text_options_nostored.clone());
        schema_builder.add_text_field("case_type", text_options_nostored.clone());
        schema_builder.add_text_field("procedure", text_options_nostored.clone());
        schema_builder.add_text_field("judgment_date", text_options_nostored.clone());
        schema_builder.add_u64_field("year", num_options.clone());
        schema_builder.add_u64_field("month", num_options.clone());
        schema_builder.add_u64_field("day", num_options);
        schema_builder.add_text_field("public_date", text_options_nostored.clone());
        schema_builder.add_text_field("parties", text_options_nostored.clone());
        schema_builder.add_text_field("cause", text_options_nostored.clone());

        schema_builder.add_text_field("legal_basis", text_options_nostored.clone());
        schema_builder.add_text_field("full_text", text_options_nostored);
        schema_builder.build()
    }

    pub fn index() -> tantivy::Result<tantivy::Index> {
        let path = std::path::Path::new(CONFIG.index_path.as_str());
        if !path.exists() {
            std::fs::create_dir(path).unwrap();
        }
        let schema = Self::schema();
        let index = tantivy::Index::open_or_create(MmapDirectory::open(path).unwrap(), schema)?;
        let stop_words = stop_words::get(stop_words::LANGUAGE::Chinese);
        let custom_stop_words = include_str!("../stopwords.txt");
        let mut custom_stop_words: HashSet<String> = custom_stop_words
            .split_whitespace()
            .map(|x| x.to_owned())
            .collect();
        custom_stop_words.extend(stop_words);

        let jieba_tokenizer = tantivy_jieba::JiebaTokenizer {};
        let tokenizer = TextAnalyzer::builder(jieba_tokenizer)
            .filter(StopWordFilter::remove(custom_stop_words))
            .filter(RemoveLongFilter::limit(40))
            .build();
        index.tokenizers().register("jieba", tokenizer);

        Ok(index)
    }

    pub fn searcher() -> tantivy::Result<Searcher> {
        let schema = Self::schema();

        let id = schema.get_field("id")?;
        let case_id = schema.get_field("case_id")?;
        let case_name = schema.get_field("case_name")?;
        let court = schema.get_field("court")?;
        let region = schema.get_field("region")?;
        let case_type = schema.get_field("case_type")?;
        let cause = schema.get_field("cause")?;
        let legal_basis = schema.get_field("legal_basis")?;
        let parties = schema.get_field("parties")?;
        let procedure = schema.get_field("procedure")?;
        let judgment_date = schema.get_field("judgment_date")?;
        let year = schema.get_field("year")?;
        let month = schema.get_field("month")?;
        let day = schema.get_field("day")?;
        let public_date = schema.get_field("public_date")?;
        let full_text = schema.get_field("full_text")?;

        let index = Self::index()?;
        let mut default_fields = vec![
            case_id,
            case_name,
            court,
            region,
            case_type,
            cause,
            legal_basis,
            parties,
            procedure,
            judgment_date,
            year,
            month,
            day,
            public_date,
        ];

        if CONFIG.index_with_full_text {
            default_fields.push(full_text);
        }

        let mut query_parser = QueryParser::for_index(&index, default_fields);

        query_parser.set_conjunction_by_default();
        query_parser.set_field_boost(case_id, 9.);
        query_parser.set_field_boost(case_name, 3.);

        let reader = index
            .reader_builder()
            .reload_policy(ReloadPolicy::OnCommitWithDelay)
            .try_into()?;

        Ok(Searcher {
            reader,
            query_parser,
            id,
        })
    }
}
