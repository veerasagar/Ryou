use std::io;

use crate::bptree::BPlusTree;
use crate::storage::{load_records, save_records};

pub fn start_cli() -> io::Result<()> {
    println!("Enter database name:");
    let mut db_name = String::new();
    io::stdin().read_line(&mut db_name)?;
    let db_name = db_name.trim();
    let file_path = format!("{}.db", db_name);

    let records = load_records(&file_path)?;
    let mut tree = BPlusTree::new();

    for record in records {
        tree.insert(record.key, record.value);
    }

    println!("Using database: {}", file_path);
    println!("B+ Tree Database (Order 4)");
    println!("Commands:");
    println!("  insert <key> <value>  - Insert a new record");
    println!("  select                - List all records");
    println!("  select <key>          - Find specific record");
    println!("  delete <key>          - Delete a record");
    println!("  exit                  - Quit the program");

    loop {
        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        let parts: Vec<&str> = input.trim().split_whitespace().collect();

        match parts.as_slice() {
            ["insert", key, value] => {
                if let Ok(key) = key.parse::<i32>() {
                    tree.insert(key, value.to_string());
                    save_records(&file_path, &tree.get_all_records())?;
                    println!("Inserted: {} => {}", key, value);
                } else {
                    eprintln!("Invalid key");
                }
            }
            ["select"] => {
                let records = tree.get_all_records();
                if records.is_empty() {
                    println!("No records found");
                } else {
                    println!("All records:");
                    for record in records {
                        println!("- {} => {}", record.key, record.value);
                    }
                }
            }
            ["select", key] => {
                if let Ok(key) = key.parse::<i32>() {
                    if let Some(value) = tree.search(key) {
                        println!("Found: {} => {}", key, value);
                    } else {
                        println!("Key {} not found", key);
                    }
                } else {
                    eprintln!("Invalid key");
                }
            }
            ["delete", key] => {
                if let Ok(key) = key.parse::<i32>() {
                    let deleted = tree.delete(key);
                    if deleted {
                        save_records(&file_path, &tree.get_all_records())?;
                        println!("Deleted key {}", key);
                    } else {
                        println!("Key {} not found", key);
                    }
                } else {
                    eprintln!("Invalid key");
                }
            }
            ["exit"] => break,
            _ => println!("Invalid command"),
        }
    }

    Ok(())
}