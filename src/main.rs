use std::env;
use std::fs::File;
use std::io::{self, BufReader};
use csv_sql_inference::{infer_schema, generate_sql};

fn main() -> io::Result<()> {
    let args: Vec<String> = env::args().collect();
    if args.len() != 2 {
        eprintln!("Usage: {} <path_to_csv_file>", args[0]);
        return Ok(());
    }

    let file_path = &args[1];
    let file = File::open(file_path)?;
    let reader = BufReader::new(file);

    let (headers, inferred_types) = infer_schema(reader)?;
    let table_name = "my_table";
    let sql_statement = generate_sql(table_name, &headers, &inferred_types);

    println!("{}", sql_statement);

    Ok(())
}
