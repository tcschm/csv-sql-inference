use std::io::{self, Read};
use csv::{ReaderBuilder, StringRecord};
use rayon::prelude::*;

mod type_inference;
mod sql_generator;
pub mod python_generator; // declare the new module, make it pub for PkStrategy in main
mod utils;

pub use type_inference::{infer_sql_type, SqlType};
pub use sql_generator::generate_sql; // for sql ddl
pub use python_generator::generate_sqlmodel_python; // for python sqlmodel

pub fn infer_schema<R: Read>(reader: R) -> io::Result<(StringRecord, Vec<SqlType>)> {
    let mut rdr = ReaderBuilder::new().has_headers(true).from_reader(reader);
    let headers = rdr.headers()?.clone();

    // if headers were expected but are empty (0 fields),
    // this implies an empty or malformed csv input that csv::Reader::headers()
    // should have errored on for completely empty input.
    // this check makes the function robust if headers() unexpectedly returns ok with 0 fields.
    if rdr.has_headers() && headers.is_empty() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "csv input is empty or headers are malformed (0 fields)",
        ));
    }
    // attempt to collect all records. if a csv::error occurs, handle it.
    let records = match rdr.records().collect::<Result<Vec<_>, csv::Error>>() {
        Ok(recs) => recs,
        Err(csv_err) => {
            // if the csv error is specifically for unequal record lengths,
            // we ensure it's mapped to io::errorkind::invaliddata.
            // the csv crate (version 1.3.1) should ideally handle this mapping correctly
            // via its `from<csv::error> for io::error` implementation.
            // this explicit check provides a safeguard or override if the observed behavior differs.
            if matches!(csv_err.kind(), csv::ErrorKind::UnequalLengths { .. }) {
                return Err(io::Error::new(io::ErrorKind::InvalidData, csv_err));
            } else {
                return Err(io::Error::from(csv_err)); // use default conversion for other csv errors
            }
        }
    };

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

#[cfg(test)]
mod tests {
    use super::*;
    use csv::StringRecord;
    use std::io::Cursor;

    #[test]
    fn test_infer_schema_simple() {
        let csv_data = "name,age,city\nAlice,30,New York\nBob,24,London";
        let reader = Cursor::new(csv_data);
        let (headers, types) = infer_schema(reader).unwrap();

        assert_eq!(headers, StringRecord::from(vec!["name", "age", "city"]));
        assert_eq!(
            types,
            vec![
                SqlType::Varchar(5), // alice
                SqlType::Integer,    // 30, 24
                SqlType::Varchar(8)  // new york
            ]
        );
    }

    #[test]
    fn test_infer_schema_mixed_types() {
        let csv_data = "id,value,timestamp_val\n1,10.5,2023-01-01 10:00:00\n2,20,2023-01-02 12:00:00";
        let reader = Cursor::new(csv_data);
        let (headers, types) = infer_schema(reader).unwrap();

        assert_eq!(headers, StringRecord::from(vec!["id", "value", "timestamp_val"]));
        // for "value" column: ["10.5", "20"] -> has_float=true, has_integer=true -> float
        assert_eq!(
            types,
            vec![
                SqlType::Integer,
                SqlType::Float,
                SqlType::Datetime
            ]
        );
    }

    #[test]
    fn test_infer_schema_with_date() {
        let csv_data = "event,date\nMeeting,2023-05-01\nConference,2023-06-15";
        let reader = Cursor::new(csv_data);
        let (headers, types) = infer_schema(reader).unwrap();

        assert_eq!(headers, StringRecord::from(vec!["event", "date"]));
        assert_eq!(
            types,
            vec![
                SqlType::Varchar(10), // conference
                SqlType::Date
            ]
        );
    }

    #[test]
    fn test_infer_schema_empty_values() {
        // empty strings are treated as nulls, allowing other values to determine the type.
        let csv_data = "name,age,score\nAlice,,100\nBob,24,\nCharlie,30,90.5";
        let reader = Cursor::new(csv_data);
        let (headers, types) = infer_schema(reader).unwrap();
        
        assert_eq!(headers, StringRecord::from(vec!["name", "age", "score"]));
        assert_eq!(
            types,
            vec![
                SqlType::Varchar(7), // charlie
                SqlType::Integer,    // age column: ["", "24", "30"] -> integer
                SqlType::Float       // score column: ["100", "", "90.5"] -> float
            ]
        );
    }

    #[test]
    fn test_infer_schema_only_headers() {
        let csv_data = "col1,col2,col3\n";
        let reader = Cursor::new(csv_data);
        let (headers, types) = infer_schema(reader).unwrap();

        assert_eq!(headers, StringRecord::from(vec!["col1", "col2", "col3"]));
        assert_eq!(
            types,
            vec![
                SqlType::Varchar(0),
                SqlType::Varchar(0),
                SqlType::Varchar(0)
            ]
        );
    }

    #[test]
    fn test_infer_schema_empty_input() {
        let csv_data = "";
        let reader = Cursor::new(csv_data);
        // expect an error because csv headers cannot be read from empty input
        assert!(infer_schema(reader).is_err());
    }

     #[test]
    fn test_infer_schema_malformed_csv_records() {
        // this test assumes that the csv crate will return an error for records
        // not matching header length, and that this error is propagated.
        // the current lib.rs code `records.push(result?);` will propagate csv::error.
        // for the function to return io::error, this would need mapping.
        // here, we test the propagation of the underlying csv::error.
        let csv_data = "header1,header2\nval1\nval3,val4"; // second record has too few fields
        let reader = Cursor::new(csv_data);
        let result = infer_schema(reader);
        assert!(result.is_err()); // expecting an error from the csv parsing
    }

    #[test]
    fn test_infer_schema_performance_many_rows() {
        use std::time::Instant;
        use std::io::Cursor;

        let num_rows = 1_000_000;
        let mut csv_data_str = String::with_capacity(num_rows * 50); // pre-allocate for rough estimate
        csv_data_str.push_str("id,name,value,timestamp,flag\n");

        let sample_rows_templates = [
            "{},Alice,10.5,2023-01-01 10:00:00,true\n",
            "{},Bob,20,2023-01-02 12:00:00,false\n",
            "{},Charlie,3000000000,2023-01-03 14:30:00,true\n",
            "{},David,,2023-01-04 16:00:00,false\n", // empty value for "value"
            "{},Eve,5.0,invalid-date,true\n",       // invalid date for "timestamp"
        ];

        for i in 0..num_rows {
            let id = i + 1;
            let row_template = sample_rows_templates[i % sample_rows_templates.len()];
            csv_data_str.push_str(&row_template.replace("{}", &id.to_string()));
        }

        let reader = Cursor::new(csv_data_str);

        let start_time = Instant::now();
        let result = infer_schema(reader);
        let duration = start_time.elapsed();

        assert!(result.is_ok(), "schema inference failed for {} rows: {:?}", num_rows, result.err());
        let (headers, types) = result.unwrap();

        // use `cargo test -- --nocapture` to see this output
        println!("\nperformance test: inferred schema for {} data rows in {:?}", num_rows, duration);
        // println!("headers: {:?}", headers);
        // println!("types: {:?}", types);

        let expected_headers = StringRecord::from(vec!["id", "name", "value", "timestamp", "flag"]);
        assert_eq!(headers, expected_headers);

        let expected_types = vec![
            SqlType::Integer,       // id (all unique integers)
            SqlType::Varchar(7),    // name (charlie)
            SqlType::Float,         // value (mix of int, bigint, float, empty strings -> float)
            SqlType::Varchar(19),   // timestamp (datetime format, forced varchar by "invalid-date")
            SqlType::Boolean,       // flag ("true", "false", etc.)
        ];
        assert_eq!(types, expected_types);
    }
}
