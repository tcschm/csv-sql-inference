use std::fs::File;
use std::io::{self, BufReader};
use std::path::PathBuf;

use clap::Parser;
use csv_sql_inference::{
    generate_sql, generate_sqlmodel_python, infer_schema, python_generator::PkStrategy,
};

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Cli {
    /// path to the csv file
    #[arg(required = true)]
    file_path: PathBuf,

    /// generate python sqlmodel code instead of sql ddl
    #[arg(long)]
    python: bool,

    /// specify an existing column name to use as the primary key for python sqlmodel
    #[arg(long, group = "pk_option")]
    pk_column: Option<String>,

    /// specify a name for a new auto-generated identity primary key for python sqlmodel
    #[arg(long, group = "pk_option")]
    pk_create: Option<String>,
}

fn main() -> io::Result<()> {
    let cli = Cli::parse();

    let file = File::open(&cli.file_path)?;
    let reader = BufReader::new(file);

    let (headers, inferred_types) = infer_schema(reader)?;
    // derive table name from the file path, or use a default
    let table_name = cli
        .file_path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("my_table");

    if cli.python {
        let pk_strategy = if let Some(col_name) = cli.pk_column {
            PkStrategy::ExistingColumn(col_name)
        } else if let Some(col_name) = cli.pk_create {
            PkStrategy::CreateColumn(col_name)
        } else {
            PkStrategy::None
        };
        let python_code =
            generate_sqlmodel_python(table_name, &headers, &inferred_types, &pk_strategy);
        println!("{}", python_code);
    } else {
        let sql_statement = generate_sql(table_name, &headers, &inferred_types);
        println!("{}", sql_statement);
    }

    Ok(())
}
