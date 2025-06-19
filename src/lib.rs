use std::io::{self, Read};
use csv::{ReaderBuilder, StringRecord};
use rayon::prelude::*;

mod type_inference;
mod sql_generator;
mod utils;

pub use type_inference::{infer_sql_type, SqlType};
pub use sql_generator::generate_sql;

pub fn infer_schema<R: Read>(reader: R) -> io::Result<(StringRecord, Vec<SqlType>)> {
    let mut rdr = ReaderBuilder::new().has_headers(true).from_reader(reader);
    let headers = rdr.headers()?.clone();
    let mut records = Vec::new();

    for result in rdr.records() {
        records.push(result?);
    }

    let num_columns = headers.len();
    let inferred_types = (0..num_columns)
        .into_par_iter()
        .map(|i| {
            let column_data: Vec<&str> = records.iter().map(|record| &record[i]).collect();
            infer_sql_type(&column_data)
        })
        .collect();

    Ok((headers, inferred_types))
}
