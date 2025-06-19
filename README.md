# CSV to SQL Schema Inference and DDL Generator

A command-line tool written in Rust that infers SQL data types from a CSV file and generates a `CREATE TABLE` SQL DDL statement. It aims to determine the strictest possible SQL type for each column that can accommodate all its values.

## features

- infers sql types for columns: `integer`, `bigint`, `float`, `char(n)`, `varchar(n)`, `date`, `datetime`, `boolean`.
- Determines the strictest type that fits all values in a column.
    - e.g., a column with `1`, `2`, `3000000000` will be `BIGINT`.
    - e.g., a column with `1`, `2.0` will be `FLOAT`.
    - e.g., a column with `apple`, `grape` (both length 5, and not a more specific type like boolean/date etc.) will be `CHAR(5)`.
    - e.g., a column with `apple`, `banana` (lengths 5 and 6) will be `VARCHAR(6)`.
    - e.g., a column with `1`, `text` (different types, varying lengths if applicable) will be `VARCHAR`.
- handles empty strings: empty strings are treated as `null` and do not influence the type inference for the non-empty values in the column. if all values in a column are empty, it defaults to `varchar(0)` (which becomes `varchar(1)` in sql due to `max(1)`).
- Generates `CREATE TABLE` SQL DDL statements.
- Table name is derived from the input CSV filename (e.g., `my_data.csv` becomes table `my_data`).
- Column names are taken directly from the CSV header.
- Utilizes parallel processing for type inference across columns using Rayon for improved performance on multi-core CPUs.

## Installation

1.  **Ensure you have Rust and Cargo installed.**
    If not, follow the instructions at rust-lang.org.

2.  **Clone the repository:**
    ```bash
    git clone <your-repository-url>
    cd csv-sql-inference
    ```

3.  **Build the project:**
    ```bash
    cargo build --release
    ```
    The executable will be located at `target/release/csv_sql_inference`.

## Usage

Run the tool from the command line, providing the path to your CSV file:

```bash
./target/release/csv_sql_inference <path_to_csv_file>
```

or, if you've added the `target/release` directory to your path:

```bash
csv_sql_inference <path_to_csv_file>
```

the generated `create table` sql statement will be printed to standard output.

### example

given a csv file named `products.csv`:

```csv
id,product_name,quantity,price,entry_date,last_updated
1,apple,10,0.50,2023-01-01,2024-05-01 10:00:00
2,banana,,1.20,2023-01-02,2024-05-02 11:30:15
3,orange,5,0.75,invalid-date,2024-05-03 09:00:00
4,grape,20000000000,0.05,2023-03-10,2024-05-04 14:20:05
```

running the command:

```bash
./target/release/csv_sql_inference products.csv
```

would produce output similar to this (exact varchar lengths depend on the longest string in each respective column):

```sql
CREATE TABLE "products" (
  "id" integer,
  "product_name" varchar(6),
  "quantity" bigint,
  "price" float,
  "entry_date" varchar(12),
  "last_updated" datetime
);
```

**explanation of example output:**
- `"id"`: all are integers.
- `"product_name"`: mixed strings, so `varchar`. length determined by "orange" (6).
- `"quantity"`: contains an empty string and a very large number. the empty string forces `varchar`. "20000000000" is 11 chars.
- `"price"`: all are valid floats.
- `"entry_date"`: contains "invalid-date", so it becomes `varchar`. length determined by "invalid-date" (12).
- `"last_updated"`: all are valid datetime strings.

## running tests

to run the unit and integration tests:

```bash
cargo test
```
