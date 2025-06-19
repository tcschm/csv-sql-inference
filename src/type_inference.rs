use chrono::{NaiveDate, NaiveDateTime};

const DATE_FORMAT: &str = "%Y-%m-%d";
const DATETIME_FORMAT: &str = "%Y-%m-%d %H:%M:%S";

#[derive(Debug, Clone, PartialEq)]
pub enum SqlType {
    Integer,
    BigInt,
    Float,
    Char(usize),
    Varchar(usize),
    Date,
    Boolean,
    Datetime,
}

// infers the strictest possible sql type that can represent all non-empty string values in a column.
// the function iterates through each value, attempting to parse it into several predefined types.
// it maintains flags for whether all values encountered so far could fit into integer (i32),
// bigint (i64), float (f64), boolean ("true", "false", "t", "f", "1", "0"), 
// date (yyyy-mm-dd), or datetime (yyyy-mm-dd hh:mm:ss).
// empty strings ("") are skipped for type checking, effectively treating them as nullable.
// if values are mixed such that no single specific type (other than varchar) applies to all 
// non-empty values, the column will be inferred as varchar.
//
// the hierarchy for type determination, from strictest to most general, is:
// 1. boolean: if all non-empty values are "true", "false", "t", "f", "1", "0" (case-insensitive).
//    this means columns containing only "1"s and "0"s (and empty strings) will be typed as boolean.
// 2. integer: if not boolean, and all non-empty values parse as i32.
// 3. bigint: if not integer, and all non-empty values parse as i64.
// 4. float: if not bigint, and all non-empty values parse as f64.
// 5. datetime: if not float, and all non-empty values parse as datetime ("%y-%m-%d %h:%m:%s").
// 6. date: if not datetime, and all non-empty values parse as date ("%y-%m-%d").
// 7. char(n): if not any of the above, and all non-empty values have the exact same length n.
// 8. varchar(n): otherwise, with length determined by the longest string encountered.
// if a column is empty or contains only empty strings, it's inferred as varchar(0).
pub fn infer_sql_type(column_data: &[&str]) -> SqlType {
    if column_data.is_empty() {
        return SqlType::Varchar(0);
    }

    let mut max_len = 0;
    let mut first_non_empty_value_len: Option<usize> = None;
    let mut all_non_empty_have_same_len = true;
    let mut all_integers = true;
    let mut all_bigints = true;
    let mut all_floats = true;
    let mut all_dates = true;
    let mut all_datetimes = true;
    let mut all_booleans = true;
    let mut has_only_empty_strings = true; // track if all values encountered are empty

    for value_str in column_data {
        max_len = max_len.max(value_str.len());

        if value_str.is_empty() {
            // we allow nullable
            continue;
        }
        has_only_empty_strings = false;

        // check for consistent length among non-empty strings
        if all_non_empty_have_same_len { // only update if still considered true
            if let Some(len) = first_non_empty_value_len {
                if value_str.len() != len {
                    all_non_empty_have_same_len = false;
                }
            } else {
                first_non_empty_value_len = Some(value_str.len());
            }
        }

        if all_integers && value_str.parse::<i32>().is_err() {
            all_integers = false;
        }
        if all_bigints && value_str.parse::<i64>().is_err() {
            all_bigints = false;
        }
        if all_floats && value_str.parse::<f64>().is_err() {
            all_floats = false;
        }
        if all_dates && NaiveDate::parse_from_str(value_str, DATE_FORMAT).is_err() {
            all_dates = false;
        }
        if all_datetimes && NaiveDateTime::parse_from_str(value_str, DATETIME_FORMAT).is_err() {
            all_datetimes = false;
        }
        if all_booleans {
            let lower_val = value_str.to_lowercase();
            if !matches!(lower_val.as_str(), "true" | "false" | "t" | "f" | "1" | "0") {
                all_booleans = false;
            }
        }
    }

    if has_only_empty_strings {
        // if the column had data rows, but all of them were empty strings.
        SqlType::Varchar(max_len) // max_len will be 0 if all strings were indeed empty.
    } else if all_booleans { // check boolean first to ensure "1"s and "0"s become boolean
        SqlType::Boolean
    } else if all_integers {
        SqlType::Integer
    } else if all_bigints {
        SqlType::BigInt
    } else if all_floats {
        SqlType::Float
    } else if all_datetimes { // check datetime before date as datetime is more specific
        SqlType::Datetime
    } else if all_dates {
        SqlType::Date
    } else if all_non_empty_have_same_len && !has_only_empty_strings {
        // if all non-empty strings have the same length, and it's not a more specific type.
        // first_non_empty_value_len is guaranteed to be some if !has_only_empty_strings
        // and all_non_empty_have_same_len remained true.
        SqlType::Char(first_non_empty_value_len.unwrap_or(0))
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
        assert_eq!(infer_sql_type(&["-10", "0", "999"]), SqlType::Integer);
    }

    #[test]
    fn test_infer_bigint() {
        assert_eq!(infer_sql_type(&["1", "2", "3000000000"]), SqlType::BigInt); // 3 bil > i32 max
        assert_eq!(infer_sql_type(&["-5000000000", "0"]), SqlType::BigInt);
        // all integers are also bigints, but integer is stricter
        assert_eq!(infer_sql_type(&["1", "2", "3"]), SqlType::Integer);
    }

    #[test]
    fn test_infer_float() {
        assert_eq!(infer_sql_type(&["1.0", "2.5", "3.14"]), SqlType::Float);
        assert_eq!(infer_sql_type(&["-0.5", "1e5", "2.0"]), SqlType::Float);
    }

    #[test]
    fn test_infer_float_mixed_with_int() {
        assert_eq!(infer_sql_type(&["1", "2.5", "3"]), SqlType::Float);
        assert_eq!(infer_sql_type(&["10000000000", "2.5"]), SqlType::Float); // bigint and float
    }

    #[test]
    fn test_infer_date_strict() {
        // all values must be dates
        assert_eq!(
            infer_sql_type(&["2023-01-01", "2024-02-15"]),
            SqlType::Date
        );
        // mixed with non-date becomes varchar
        assert_eq!(
            infer_sql_type(&["2023-01-01", "text", "123"]),
            SqlType::Varchar(10) // "2023-01-01" is longest
        );
    }

    #[test]
    fn test_infer_datetime_strict() {
        // all values must be datetimes
        assert_eq!(
            infer_sql_type(&["2023-01-01 10:00:00", "2024-02-15 23:59:59"]),
            SqlType::Datetime
        );
        // mixed with non-datetime becomes varchar
        assert_eq!(
            infer_sql_type(&["2023-01-01 10:00:00", "text", "123"]),
            SqlType::Varchar(19) // "2023-01-01 10:00:00" is longest
        );
    }

    #[test]
    fn test_infer_mixed_date_and_datetime_is_varchar() {
        // with strict parsing for all elements, a mix of date and datetime strings becomes varchar
        assert_eq!(
            infer_sql_type(&["2023-01-01", "2023-01-01 12:00:00"]),
            SqlType::Varchar(19)
        );
        assert_eq!(
            infer_sql_type(&["2023-01-01 12:00:00", "2023-01-01"]),
            SqlType::Varchar(19)
        );
    }

    #[test]
    fn test_infer_varchar() {
        // "hello" (len 5) and "world" (len 5) would be char(5).
        // for varchar, lengths must differ or type must not be char.
        assert_eq!(infer_sql_type(&["hello", "world!"]), SqlType::Varchar(6)); // different lengths
        assert_eq!(infer_sql_type(&["apple", "banana", "kiwi"]), SqlType::Varchar(6));
    }

    #[test]
    fn test_infer_varchar_for_mixed_types() {
        // mixed types that don't all conform to a numeric or date/datetime type become varchar
        assert_eq!(infer_sql_type(&["1", "world"]), SqlType::Varchar(5));
        assert_eq!(infer_sql_type(&["1.1", "world"]), SqlType::Varchar(5));
        assert_eq!(infer_sql_type(&["2023-01-01", "world"]), SqlType::Varchar(10));
        // if only text or unparseable
        assert_eq!(infer_sql_type(&["text", "world"]), SqlType::Varchar(5));
    }

    #[test]
    fn test_infer_empty_column_data() {
        assert_eq!(infer_sql_type(&[]), SqlType::Varchar(0));
    }

    #[test]
    fn test_infer_column_with_empty_strings() {
        // empty strings are treated as nulls and do not dictate the type if other values are present.
        // if all values are empty strings, it's varchar(0).
        assert_eq!(infer_sql_type(&["", ""]), SqlType::Varchar(0));
        assert_eq!(infer_sql_type(&["a", ""]), SqlType::Char(1)); // "a" is length 1, "" is null -> char(1)
        // "1" and "" -> boolean because "1" is a valid boolean, "" is null
        assert_eq!(infer_sql_type(&["1", ""]), SqlType::Boolean);
        // "1.0" and "" -> float
        assert_eq!(infer_sql_type(&["1.0", ""]), SqlType::Float);
        // "2023-01-01" and "" -> date
        assert_eq!(infer_sql_type(&["2023-01-01", ""]), SqlType::Date);
        // "true" and "" -> boolean
        assert_eq!(infer_sql_type(&["true", ""]), SqlType::Boolean);
    }
    
    #[test]
    fn test_infer_char() {
        assert_eq!(infer_sql_type(&["abc", "def", "ghi"]), SqlType::Char(3));
        assert_eq!(infer_sql_type(&["fixed", "fixed"]), SqlType::Char(5));
        assert_eq!(infer_sql_type(&["hello", "world"]), SqlType::Char(5)); // both length 5
    }

    #[test]
    fn test_infer_char_with_empty_strings() {
        assert_eq!(infer_sql_type(&["ab", "", "cd", "", "ef"]), SqlType::Char(2));
        assert_eq!(infer_sql_type(&["", "xyz", ""]), SqlType::Char(3));
        assert_eq!(infer_sql_type(&["a", "", "b"]), SqlType::Char(1));
    }

    #[test]
    fn test_char_does_not_override_specific_types() {
        assert_eq!(infer_sql_type(&["1", "2", "3"]), SqlType::Integer); // not char(1)
        assert_eq!(infer_sql_type(&["T", "F"]), SqlType::Boolean); // not char(1)
        assert_eq!(infer_sql_type(&["2023-01-01", "2023-01-02"]), SqlType::Date); // not char(10)
    }

    #[test]
    fn test_infer_boolean() {
        assert_eq!(infer_sql_type(&["true", "false", "TRUE", "FALSE"]), SqlType::Boolean);
        assert_eq!(infer_sql_type(&["t", "f", "T", "F"]), SqlType::Boolean);
        assert_eq!(infer_sql_type(&["1", "0"]), SqlType::Boolean);
        assert_eq!(infer_sql_type(&["1", "0", "true"]), SqlType::Boolean); // mixed non-numeric with numeric-bools
        assert_eq!(infer_sql_type(&["true", "false", ""]), SqlType::Boolean); // with empty string
        assert_eq!(infer_sql_type(&["true", "not_bool"]), SqlType::Varchar(8));
        assert_eq!(infer_sql_type(&["true", "false", "0", "four"]), SqlType::Varchar(5));
    }

    #[test]
    fn test_infer_invalid_date_as_varchar() {
        assert_eq!(infer_sql_type(&["2023-13-01"]), SqlType::Char(10)); // invalid month
        assert_eq!(infer_sql_type(&["not-a-date"]), SqlType::Char(10));
    }
}
