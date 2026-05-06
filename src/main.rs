use std::env;
use std::path::PathBuf;

use tinysql::storage::{ColType, Column, Storage, Value};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = env::args().skip(1);
    let db_path = PathBuf::from(args.next().ok_or("usage: tinysql <db> <command> ...")?);
    let cmd = args.next().ok_or("missing command")?;

    let mut st = Storage::open(&db_path)?;

    match cmd.as_str() {
        "init" => {
            // Example: tinysql db.bin init id:int name:text
            let cols: Vec<Column> = args
                .map(|pair| {
                    let (n, t) = pair.split_once(':').ok_or("col format name:int")?;
                    let ty = match t {
                        "int" => ColType::Int,
                        "text" => ColType::Text,
                        _ => return Err("type must be int or text".into()),
                    };
                    Ok(Column {
                        name: n.to_string(),
                        ty,
                    })
                })
                .collect::<Result<_, Box<dyn std::error::Error>>>()?;
            st.init_schema(cols)?;
            println!("schema written");
        }
        "insert" => {
            // Values as alternating tokens: 42 hello  (int then text for schema above)
            let schema = st.schema().ok_or("run init first")?.to_vec();
            let tokens: Vec<String> = args.collect();
            if tokens.len() != schema.len() {
                return Err(format!(
                    "expected {} values, got {}",
                    schema.len(),
                    tokens.len()
                )
                .into());
            }
            let mut values = Vec::new();
            for (col, tok) in schema.iter().zip(tokens) {
                match col.ty {
                    ColType::Int => {
                        let x: i64 = tok.parse()?;
                        values.push(Value::Int(x));
                    }
                    ColType::Text => values.push(Value::Text(tok)),
                }
            }
            st.append_row(&values)?;
            println!("inserted");
        }
        "scan" => {
            let rows = st.scan_rows()?;
            for row in rows {
                println!("{:?}", row);
            }
        }
        _ => return Err("commands: init | insert | scan".into()),
    }

    Ok(())
}