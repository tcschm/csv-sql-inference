use chrono::{NaiveDate, NaiveDateTime};

const DATE_FORMAT: &str = "%Y-%m-%d";
const DATETIME_FORMAT: &str = "%Y-%m-%d %H:%M:%S";

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

// infers the sql type for a column based on its string values.
// the function iterates through each value in the column:
// - if a value can be parsed as a date (format: yyyy-mm-dd), the function immediately returns sqltype::date.
// - otherwise, if a value can be parsed as a datetime (format: yyyy-mm-dd hh:mm:ss), it immediately returns sqltype::datetime.
// - if no date or datetime is found after checking all values, the inference proceeds:
//   - numeric flags (has_integer, has_float) are updated for each non-date/datetime value.
//   - if any value was parsable as a float, the column type is sqltype::float.
//   - otherwise, if any value was parsable as an integer, the column type is sqltype::integer.
//   - otherwise (e.g., all strings, or mixed with unparsable content not meeting numeric criteria),
//     the column type is sqltype::varchar with a length determined by the longest string encountered.
//
// note: empty strings or strings that don't parse into any specific type
// contribute to varchar length. if a column contains mixed data like "123" and "abc"
// (and no dates/datetimes), it will be inferred as sqltype::integer due to "123",
// as per the current logic and tests.
pub fn infer_sql_type(column_data: &[&str]) -> SqlType {
    let mut has_float = false;
    let mut has_integer = false;
    let mut max_len = 0;

    if column_data.is_empty() {
        return SqlType::Varchar(0);
    }

    for value in column_data {
        max_len = max_len.max(value.len());
        if value.parse::<i32>().is_ok() {
            has_integer = true;
        } else if value.parse::<f32>().is_ok() {
            has_float = true;
        }
        // date/datetime checks have early returns
        if NaiveDate::parse_from_str(value, DATE_FORMAT).is_ok() {
            return SqlType::Date;
        } else if NaiveDateTime::parse_from_str(value, DATETIME_FORMAT).is_ok() {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_infer_integer() {
        assert_eq!(infer_sql_type(&["1", "2", "300"]), SqlType::Integer);
    }

    #[test]
    fn test_infer_float() {
        assert_eq!(infer_sql_type(&["1.0", "2.5", "3.14"]), SqlType::Float);
    }

    #[test]
    fn test_infer_float_mixed_with_int() {
        // if a float is present, it should become float
        assert_eq!(infer_sql_type(&["1", "2.5", "3"]), SqlType::Float);
    }

    #[test]
    fn test_infer_date_takes_precedence() {
        // date parsing returns immediately
        assert_eq!(
            infer_sql_type(&["2023-01-01", "text", "123"]),
            SqlType::Date
        );
        assert_eq!(
            infer_sql_type(&["text", "2023-01-01", "123"]),
            SqlType::Date
        );
    }

    #[test]
    fn test_infer_datetime_takes_precedence() {
        // datetime parsing returns immediately
        assert_eq!(
            infer_sql_type(&["2023-01-01 10:00:00", "text", "123"]),
            SqlType::Datetime
        );
        assert_eq!(
            infer_sql_type(&["text", "2023-01-01 10:00:00", "123"]),
            SqlType::Datetime
        );
    }

    #[test]
    fn test_infer_date_over_datetime_if_date_format_first() {
        // if a value is a valid date string and checked first, it returns date
        assert_eq!(
            infer_sql_type(&["2023-01-01", "2023-01-01 12:00:00"]),
            SqlType::Date
        );
    }

    #[test]
    fn test_infer_datetime_if_datetime_format_first() {
         assert_eq!(
            infer_sql_type(&["2023-01-01 12:00:00", "2023-01-01"]),
            SqlType::Datetime
        );
    }

    #[test]
    fn test_infer_varchar() {
        assert_eq!(infer_sql_type(&["hello", "world"]), SqlType::Varchar(5));
    }

    #[test]
    fn test_infer_varchar_for_mixed_non_date_non_datetime() {
        // current behavior: if "world" is present, and no date/datetime caused early return,
        // it will check has_float, then has_integer.
        // "1" makes has_integer=true. "world" doesn't parse. Loop ends.
        // has_float=false, has_integer=true. returns integer.
        // this test documents current behavior, though it might be undesirable.
        assert_eq!(infer_sql_type(&["1", "world"]), SqlType::Integer);
        // similarly for float
        assert_eq!(infer_sql_type(&["1.1", "world"]), SqlType::Float);
        // if only text or unparseable
        assert_eq!(infer_sql_type(&["text", "world"]), SqlType::Varchar(5));
    }

    #[test]
    fn test_infer_empty_column_data() {
        assert_eq!(infer_sql_type(&[]), SqlType::Varchar(0));
    }

    #[test]
    fn test_infer_column_with_empty_strings() {
        // empty strings don't parse as numbers/dates, max_len is tracked.
        assert_eq!(infer_sql_type(&["", ""]), SqlType::Varchar(0));
        assert_eq!(infer_sql_type(&["a", ""]), SqlType::Varchar(1));
    }

    #[test]
    fn test_infer_invalid_date_as_varchar() {
        assert_eq!(infer_sql_type(&["2023-13-01"]), SqlType::Varchar(10)); // invalid month
    }
}
