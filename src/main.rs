use std::env;
use std::fs::File;
use std::io::{self, BufReader};
use csv_sql_inference::{infer_schema, generate_sql};

fn main() -> io::Result<()> {
    let args: Vec<String> = env::args().collect();
    if args.len() != 2 {
        // using eprintln! to print to stderr for error messages
        eprintln!("Usage: {} <path_to_csv_file>", args[0]);
        return Ok(());
    }

    let file_path = &args[1];
    let file = File::open(file_path)?;
    let reader = BufReader::new(file);

    let (headers, inferred_types) = infer_schema(reader)?;
    // derive table name from the file path, or use a default
    let table_name = std::path::Path::new(file_path)
        .file_stem().and_then(|s| s.to_str()).unwrap_or("my_table");
    let sql_statement = generate_sql(table_name, &headers, &inferred_types);

    println!("{}", sql_statement);

    Ok(())
}
