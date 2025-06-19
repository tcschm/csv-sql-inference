use csv_sql_inference::{generate_sql, infer_schema, SqlType};
use std::io::Cursor;

#[test]
fn test_simple_csv_to_sql_generation() {
    let csv_data = "name,age,city\nAlice,30,New York\nBob,24,London";
    let reader = Cursor::new(csv_data);
    let (headers, types) = infer_schema(reader).expect("schema inference failed for simple csv");

    // explicitly collect headers into a vec<&str> for comparison
    assert_eq!(headers.iter().collect::<Vec<&str>>(), &["name", "age", "city"][..]);
    assert_eq!(
        types,
        vec![
            SqlType::Varchar(5), // alice
            SqlType::Integer,    // 30
            SqlType::Varchar(8)  // new york
        ]
    );

    let sql = generate_sql("simple_table", &headers, &types);
    let expected_sql = "CREATE TABLE \"simple_table\" (\n  \"name\" VARCHAR(5),\n  \"age\" INTEGER,\n  \"city\" VARCHAR(8)\n);";
    assert_eq!(sql, expected_sql);
}

#[test]
fn test_mixed_types_csv_to_sql_generation() {
    let csv_data = "id,value,timestamp_val,description\n1,10.5,2023-01-01 10:00:00,first item\n2,20,2023-01-02 12:00:00,second item";
    let reader = Cursor::new(csv_data);
    let (headers, types) = infer_schema(reader).expect("schema inference failed for mixed types csv");

    // explicitly collect headers into a vec<&str> for comparison
    assert_eq!(headers.iter().collect::<Vec<&str>>(), &["id", "value", "timestamp_val", "description"][..]);
    assert_eq!(
        types,
        vec![
            SqlType::Integer,    // 1, 2
            SqlType::Float,      // 10.5, 20 (promotes to float)
            SqlType::Datetime,   // datetime strings
            SqlType::Varchar(11) // "second item"
        ]
    );

    let sql = generate_sql("mixed_table", &headers, &types);
    let expected_sql = "CREATE TABLE \"mixed_table\" (\n  \"id\" INTEGER,\n  \"value\" FLOAT,\n  \"timestamp_val\" DATETIME,\n  \"description\" VARCHAR(11)\n);";
    assert_eq!(sql, expected_sql);
}

#[test]
fn test_infer_schema_with_empty_csv_input() {
    let csv_data = "";
    let reader = Cursor::new(csv_data);
    let result = infer_schema(reader);
    // expecting an error because the csv input is completely empty
    assert!(result.is_err());
    if let Err(e) = result {
        assert_eq!(e.kind(), std::io::ErrorKind::InvalidInput);
        // you could also check the error message if it's stable, e.g.,
        // assert_eq!(e.to_string().to_lowercase(), "csv input is empty");
    }
}

#[test]
fn test_csv_with_only_headers() {
    let csv_data = "col_a,col_b,col_c\n";
    let reader = Cursor::new(csv_data);
    let (headers, types) = infer_schema(reader).expect("schema inference failed for headers-only csv");

    // explicitly collect headers into a vec<&str> for comparison
    assert_eq!(headers.iter().collect::<Vec<&str>>(), &["col_a", "col_b", "col_c"][..]);
    assert_eq!(
        types,
        vec![
            SqlType::Varchar(0),
            SqlType::Varchar(0),
            SqlType::Varchar(0)
        ]
    );

    let sql = generate_sql("headers_only_table", &headers, &types);
    // note: varchar(0) might not be valid in all sql dialects,
    // but generate_sql ensures at least varchar(1).
    let expected_sql = "CREATE TABLE \"headers_only_table\" (\n  \"col_a\" VARCHAR(1),\n  \"col_b\" VARCHAR(1),\n  \"col_c\" VARCHAR(1)\n);";
    assert_eq!(sql, expected_sql);
}

#[test]
fn test_malformed_csv_different_column_counts() {
    // this csv has a second data row with fewer columns than the header
    let csv_data = "header1,header2,header3\nval1,val2,val3\nshort_val1,short_val2";
    let reader = Cursor::new(csv_data);
    let result = infer_schema(reader);

    // the csv crate's record parsing should fail and be mapped to an io::error
    assert!(result.is_err());
    if let Err(e) = result {
        // the error kind from csv parsing (unequal lengths) is mapped to invaliddata
        assert_eq!(e.kind(), std::io::ErrorKind::InvalidData);
    }
}

#[test]
fn test_table_name_generation_from_main_logic() {
    // this test mimics the table name generation logic from main.rs
    let file_path = "data/my_data_set.csv";
    let table_name_derived = std::path::Path::new(file_path)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("default_table");
    assert_eq!(table_name_derived, "my_data_set");

    let file_path_no_ext = "my_other_data";
     let table_name_derived_no_ext = std::path::Path::new(file_path_no_ext)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("default_table");
    assert_eq!(table_name_derived_no_ext, "my_other_data");
}