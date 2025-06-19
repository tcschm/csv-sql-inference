use super::{SqlType, StringRecord};

/// generates a `create table` sql statement from a table name, headers, and inferred types.
pub fn generate_sql(table_name: &str, headers: &StringRecord, types: &[SqlType]) -> String {
    // quote the table name to handle names with spaces or special characters.
    let mut sql = format!("CREATE TABLE \"{}\" (\n", table_name);

    let columns: Vec<String> = headers
        .iter()
        .zip(types.iter())
        .map(|(header, sql_type)| {
            let type_str = match sql_type {
                SqlType::Integer => "INTEGER".to_string(),
                SqlType::BigInt => "BIGINT".to_string(),
                SqlType::Float => "FLOAT".to_string(),
                SqlType::Varchar(len) => format!("VARCHAR({})", len.max(&1)),
                SqlType::Date => "DATE".to_string(),
                SqlType::Boolean => "BOOLEAN".to_string(),
                SqlType::Datetime => "DATETIME".to_string(),
                SqlType::Text => "TEXT".to_string(),
            };
            // quote column names to handle spaces or special characters.
            format!("  \"{}\" {}", header.trim(), type_str)
        })
        .collect();

    sql.push_str(&columns.join(",\n"));
    sql.push_str("\n);");

    sql
}
