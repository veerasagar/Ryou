use std::io;
use std::time::{Instant, Duration};
use std::collections::HashMap;

use crate::btree::BTree;
use crate::storage::{load_records, save_records};
use crate::btree::Record;

pub fn start_cli() -> io::Result<()> {
    println!("Enter database name:");
    let mut db_name = String::new();
    io::stdin().read_line(&mut db_name)?;
    let db_name = db_name.trim();
    let file_path = format!("{}.db", db_name);

    let records = load_records(&file_path)?;
    let mut tree = BTree::new();

    for record in records {
        tree.insert(record.key, record.value);
    }

    println!("Using database: {}", file_path);
    println!("B-Tree Database (Order 4)");
    println!("Commands:");
    println!("  insert <key> <value>  - Insert a new record");
    println!("  select                - List all records");
    println!("  select <key>          - Find specific record");
    println!("  delete <key>          - Delete a record");
    println!("  analyze <key>         - Compare search performance across data structures");
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
            ["analyze", key] => {
                if let Ok(key) = key.parse::<i32>() {
                    analyze_performance(key)?;
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

fn analyze_performance(key: i32) -> io::Result<()> {
    println!("Loading spare database for performance analysis...");
    let spare_file_path = "spare.db";
    
    // Load records from spare database
    let records = match load_records(spare_file_path) {
        Ok(recs) => recs,
        Err(e) => {
            println!("Error loading spare database: {}", e);
            println!("Make sure 'spare.db' exists in the current directory.");
            return Ok(());
        }
    };
    
    if records.is_empty() {
        println!("No records found in spare database.");
        return Ok(());
    }
    
    println!("Loaded {} records from spare database", records.len());
    println!("Beginning performance analysis for key: {}", key);
    
    // Initialize data structures
    let mut btree = BTree::new();
    let mut hashtable: HashMap<i32, String> = HashMap::new();
    let mut array: Vec<Record> = Vec::new();
    
    // Populate data structures
    println!("Populating data structures...");
    for record in &records {
        btree.insert(record.key, record.value.clone());
        hashtable.insert(record.key, record.value.clone());
        array.push(Record {
            key: record.key,
            value: record.value.clone(),
        });
    }
    
    // Measure B-Tree search time
    println!("\nPerforming searches...");
    let btree_result = measure_btree_search(&btree, key);
    
    // Measure HashMap search time
    let hashtable_result = measure_hashtable_search(&hashtable, key);
    
    // Measure Array search time
    let array_result = measure_array_search(&array, key);
    
    // Print results
    println!("\n----- Performance Results -----");
    println!("Key: {}", key);
    println!("B-Tree search:      {:?} - Result: {}", btree_result.1, result_to_string(btree_result.0));
    println!("HashMap search:     {:?} - Result: {}", hashtable_result.1, result_to_string(hashtable_result.0));
    println!("Array linear search: {:?} - Result: {}", array_result.1, result_to_string(array_result.0));
    
    // Comparison analysis
    println!("\n----- Analysis -----");
    let mut times = vec![
        ("B-Tree", btree_result.1),
        ("HashMap", hashtable_result.1),
        ("Array", array_result.1),
    ];
    times.sort_by(|a, b| a.1.cmp(&b.1));
    
    println!("Fastest: {} ({:?})", times[0].0, times[0].1);
    println!("Slowest: {} ({:?})", times[2].0, times[2].1);
    
    if times[0].1.as_nanos() > 0 {
        let fastest_to_slowest = times[2].1.as_nanos() as f64 / times[0].1.as_nanos() as f64;
        println!("The slowest structure was {:.2}x slower than the fastest", fastest_to_slowest);
    }
    
    Ok(())
}

fn measure_btree_search(btree: &BTree, key: i32) -> (Option<String>, Duration) {
    let start = Instant::now();
    let result = btree.search(key);
    let duration = start.elapsed();
    (result, duration)
}

fn measure_hashtable_search(hashtable: &HashMap<i32, String>, key: i32) -> (Option<String>, Duration) {
    let start = Instant::now();
    let result = hashtable.get(&key).cloned();
    let duration = start.elapsed();
    (result, duration)
}

fn measure_array_search(array: &[Record], key: i32) -> (Option<String>, Duration) {
    let start = Instant::now();
    let result = array.iter()
        .find(|record| record.key == key)
        .map(|record| record.value.clone());
    let duration = start.elapsed();
    (result, duration)
}

fn result_to_string(result: Option<String>) -> String {
    match result {
        Some(value) => format!("Found \"{}\"", value),
        None => "Not found".to_string(),
    }
}