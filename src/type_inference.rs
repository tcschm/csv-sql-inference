use chrono::{NaiveDate, NaiveDateTime};

#[derive(Debug, Clone, PartialEq)]
pub enum SqlType {
    Integer,
    BigInt,
    Float,
    Varchar(usize),
    Date,
    Datetime,
    Text,
}

pub fn infer_sql_type(column_data: &[&str]) -> SqlType {
    let mut has_float = false;
    let mut has_integer = false;
    let mut max_len = 0;

    for value in column_data {
        max_len = max_len.max(value.len());
        if value.parse::<i32>().is_ok() {
            has_integer = true;
        } else if value.parse::<f32>().is_ok() {
            has_float = true;
        } else if NaiveDate::parse_from_str(value, "%Y-%m-%d").is_ok() {
            return SqlType::Date;
        } else if NaiveDateTime::parse_from_str(value, "%Y-%m-%d %H:%M:%S").is_ok() {
            return SqlType::Datetime;
        }
    }

    if has_float {
        SqlType::Float
    } else if has_integer {
        SqlType::Integer
    } else {
        SqlType::Varchar(max_len)
    }
}
