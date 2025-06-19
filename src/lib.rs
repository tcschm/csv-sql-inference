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
    let records: Vec<StringRecord> = rdr.records().collect::<Result<Vec<_>, _>>()?;

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
        // current infer_sql_type treats "" as a string that fails parsing for numbers/dates.
        // if other values make it an integer/float, it will be inferred as such.
        let csv_data = "name,age,score\nAlice,,100\nBob,24,\nCharlie,30,90.5";
        let reader = Cursor::new(csv_data);
        let (headers, types) = infer_schema(reader).unwrap();

        assert_eq!(headers, StringRecord::from(vec!["name", "age", "score"]));
        assert_eq!(
            types,
            vec![
                SqlType::Varchar(7), // charlie
                SqlType::Integer,    // age column: ["", "24", "30"] -> integer (due to "24", "30")
                SqlType::Float       // score column: ["100", "", "90.5"] -> float (due to "90.5")
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
}
