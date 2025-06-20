use super::{SqlType, StringRecord};
use crate::utils::to_pascal_case;

#[derive(Debug, Clone, PartialEq)]
pub enum PkStrategy {
    ExistingColumn(String),
    CreateColumn(String),
    None,
}

/// generates python sqlmodel code from a table name, headers, and inferred types.
pub fn generate_sqlmodel_python(
    table_name: &str,
    headers: &StringRecord,
    types: &[SqlType],
    pk_strategy: &PkStrategy,
) -> String {
    let class_name = to_pascal_case(table_name);
    
    let mut py_code = String::new();
    // py_code.push_str("from typing import Optional\n"); // no longer needed for `type | none`
    py_code.push_str("from datetime import date, datetime\n");
    py_code.push_str("from sqlmodel import Field, SQLModel\n\n\n");

    py_code.push_str(&format!("class {}(SQLModel, table=True):\n", class_name));

    let mut pk_field_generated_or_identified = false;

    // handle --pk-create strategy first
    if let PkStrategy::CreateColumn(pk_name) = pk_strategy {
        let sanitized_pk_name = pk_name.trim().replace(' ', "_").to_lowercase();
        py_code.push_str(&format!(
            "    {}: int | None = Field(default=None, primary_key=True)\n",
            sanitized_pk_name
        ));
        pk_field_generated_or_identified = true;
    }

    for (i, header) in headers.iter().enumerate() {
        let original_header_sanitized = header.trim().replace(' ', "_").to_lowercase();

        // if --pk-create was used, and current header matches the created pk name, skip it
        if let PkStrategy::CreateColumn(pk_name_to_create) = pk_strategy {
            if original_header_sanitized == pk_name_to_create.trim().replace(' ', "_").to_lowercase() {
                // this column from csv is being shadowed by the explicitly created pk.
                // ideally, we'd warn the user or handle this more gracefully.
                // for now, we skip generating it from the csv data.
                continue;
            }
        }

        let sql_type = &types[i];
        let field_name = header.trim().replace(' ', "_").to_lowercase(); // basic sanitization

        let (py_type, mut field_params) = match sql_type {
            SqlType::Integer | SqlType::BigInt => ("int | None", "default=None".to_string()),
            SqlType::Float => ("float | None", "default=None".to_string()),
            SqlType::Char(len) => (
                "str | None",
                format!("default=None, max_length={}", (*len).max(1)),
            ),
            SqlType::Varchar(len) => (
                "str | None",
                format!("default=None, max_length={}", (*len).max(1)),
            ),
            SqlType::Date => ("date | None", "default=None".to_string()),
            SqlType::Boolean => ("bool | None", "default=None".to_string()),
            SqlType::Datetime => ("datetime | None", "default=None".to_string()),
        };

        // handle --pk-column strategy
        if let PkStrategy::ExistingColumn(pk_col_name) = pk_strategy {
            if original_header_sanitized == pk_col_name.trim().replace(' ', "_").to_lowercase() {
                if !field_params.is_empty() {
                    field_params.push_str(", ");
                }
                field_params.push_str("primary_key=True");
                // todo: consider adding a check here if the inferred type is suitable for a pk (e.g., not float)
                // and maybe add a comment if it's unusual (e.g. # warning: using float as pk)
                pk_field_generated_or_identified = true;
            }
        }

        py_code.push_str(&format!(
            "    {}: {} = Field({})\n",
            field_name, py_type, field_params
        ));
    }

    if !pk_field_generated_or_identified && !headers.is_empty() {
        // this condition means headers were present, fields were generated, but no pk was made.
        py_code.push_str("    # todo: review and define a primary_key=true field for this model.\n");
    } else if headers.is_empty() && !pk_field_generated_or_identified {
        py_code.push_str("    # no columns inferred, add fields manually\n    pass\n");
    } else if headers.is_empty() && matches!(pk_strategy, PkStrategy::CreateColumn(_)) {
        // only the --pk-create field was generated
        py_code.push_str("    pass # only primary key field was generated, add other fields\n");
    }

    py_code
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::SqlType; // ensure sqltype is in scope
    use csv::StringRecord;

    fn normalize_whitespace(s: &str) -> String {
        s.lines().map(|line| line.trim()).filter(|line| !line.is_empty()).collect::<Vec<_>>().join("\n")
    }

    #[test]
    fn test_generate_simple_model() {
        let table_name = "simple_users";
        let headers = StringRecord::from(vec!["name", "age"]);
        let types = vec![SqlType::Varchar(50), SqlType::Integer];
        let expected_python = r#"
from datetime import date, datetime
from sqlmodel import Field, SQLModel


class SimpleUsers(SQLModel, table=True):
    name: str | None = Field(default=None, max_length=50)
    age: int | None = Field(default=None)
    # todo: review and define a primary_key=true field for this model.
"#;
        let generated_python = generate_sqlmodel_python(table_name, &headers, &types, &PkStrategy::None);
        assert_eq!(normalize_whitespace(&generated_python), normalize_whitespace(expected_python));
    }

    #[test]
    fn test_generate_model_with_id_pk() {
        let table_name = "products_table";
        let headers = StringRecord::from(vec!["id", "product_name", "price"]);
        let types = vec![SqlType::Integer, SqlType::Varchar(100), SqlType::Float];
        let expected_python = r#"
from datetime import date, datetime
from sqlmodel import Field, SQLModel


class ProductsTable(SQLModel, table=True):
    id: int | None = Field(default=None, primary_key=True)
    product_name: str | None = Field(default=None, max_length=100)
    price: float | None = Field(default=None)
"#;
        let generated_python = generate_sqlmodel_python(table_name, &headers, &types, &PkStrategy::ExistingColumn("id".to_string()));
        assert_eq!(normalize_whitespace(&generated_python), normalize_whitespace(expected_python));
    }

    #[test]
    fn test_generate_model_with_pk_create() {
        let table_name = "items";
        let headers = StringRecord::from(vec!["item_name", "quantity"]);
        let types = vec![SqlType::Varchar(50), SqlType::Integer];
        let pk_strategy = PkStrategy::CreateColumn("item_id".to_string());
        let expected_python = r#"
from datetime import date, datetime
from sqlmodel import Field, SQLModel


class Items(SQLModel, table=True):
    item_id: int | None = Field(default=None, primary_key=True)
    item_name: str | None = Field(default=None, max_length=50)
    quantity: int | None = Field(default=None)
"#;
        let generated_python = generate_sqlmodel_python(table_name, &headers, &types, &pk_strategy);
        assert_eq!(normalize_whitespace(&generated_python), normalize_whitespace(expected_python));
    }

     #[test]
    fn test_generate_model_with_pk_create_shadows_csv_column() {
        let table_name = "events";
        // "event_id" is in csv, but we also ask to create "event_id" as pk
        let headers = StringRecord::from(vec!["event_id", "event_name", "location"]); 
        let types = vec![SqlType::Varchar(10), SqlType::Varchar(50), SqlType::Varchar(30)];
        let pk_strategy = PkStrategy::CreateColumn("event_id".to_string());
        let expected_python = r#"
from datetime import date, datetime
from sqlmodel import Field, SQLModel


class Events(SQLModel, table=True):
    event_id: int | None = Field(default=None, primary_key=True)
    event_name: str | None = Field(default=None, max_length=50)
    location: str | None = Field(default=None, max_length=30)
"#;
        // the event_id from csv (varchar(10)) should be skipped in favor of the created int pk.
        let generated_python = generate_sqlmodel_python(table_name, &headers, &types, &pk_strategy);
        assert_eq!(normalize_whitespace(&generated_python), normalize_whitespace(expected_python));
    }

    #[test]
    fn test_generate_model_no_pk_strategy_adds_comment() {
        let table_name = "logs";
        let headers = StringRecord::from(vec!["message", "level"]);
        let types = vec![SqlType::Varchar(200), SqlType::Char(5)];
        let generated_python = generate_sqlmodel_python(table_name, &headers, &types, &PkStrategy::None);
        assert!(generated_python.contains("# todo: review and define a primary_key=true field for this model."));
    }
    #[test]
    fn test_generate_model_all_types() {
        let table_name = "comprehensive_data";
        let headers = StringRecord::from(vec!["user_id", "score", "reg_date", "last_login", "is_active", "notes", "short_code"]);
        let types = vec![
            SqlType::BigInt,
            SqlType::Float,
            SqlType::Date,
            SqlType::Datetime,
            SqlType::Boolean,
            SqlType::Varchar(255),
            SqlType::Char(10),
        ];
        let expected_python = r#"
from datetime import date, datetime
from sqlmodel import Field, SQLModel


class ComprehensiveData(SQLModel, table=True):
    user_id: int | None = Field(default=None)
    score: float | None = Field(default=None)
    reg_date: date | None = Field(default=None)
    last_login: datetime | None = Field(default=None)
    is_active: bool | None = Field(default=None)
    notes: str | None = Field(default=None, max_length=255)
    short_code: str | None = Field(default=None, max_length=10)
    # todo: review and define a primary_key=true field for this model.
"#;
        let generated_python = generate_sqlmodel_python(table_name, &headers, &types, &PkStrategy::None);
        assert_eq!(normalize_whitespace(&generated_python), normalize_whitespace(expected_python));
    }

    #[test]
    fn test_generate_model_empty_columns() {
        let table_name = "empty_table";
        let headers = StringRecord::new();
        let types = vec![];
        let expected_python = r#"
from datetime import date, datetime
from sqlmodel import Field, SQLModel


class EmptyTable(SQLModel, table=True):
    # no columns inferred, add fields manually
    pass
"#;
        let generated_python = generate_sqlmodel_python(table_name, &headers, &types, &PkStrategy::None);
        assert_eq!(normalize_whitespace(&generated_python), normalize_whitespace(expected_python));
    }

    // test for a table name that needs pascal case conversion is implicitly covered
    // by other tests like test_generate_simple_model (simple_users -> SimpleUsers)
    // and test_generate_model_with_id_pk (products_table -> ProductsTable).
}