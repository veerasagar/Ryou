use std::fs::{File, OpenOptions};
use std::io::{self, Read, Write};
use std::path::Path;

const ORDER: usize = 4; // B+ tree order

#[derive(Debug, Clone)]
struct Record {
    key: i32,
    value: String,
}

#[derive(Clone)]
struct Node {
    keys: Vec<i32>,
    children: Vec<Node>,
    records: Vec<Record>,
    is_leaf: bool,
    next: Option<Box<Node>>,
}

impl Node {
    fn new_leaf() -> Self {
        Node {
            keys: Vec::new(),
            children: Vec::new(),
            records: Vec::new(),
            is_leaf: true,
            next: None,
        }
    }

    fn new_internal() -> Self {
        Node {
            keys: Vec::new(),
            children: Vec::new(),
            records: Vec::new(),
            is_leaf: false,
            next: None,
        }
    }
}

struct BPlusTree {
    root: Node,
    file_path: String, // Path to the .db file
}

impl BPlusTree {
    fn new(file_path: &str) -> io::Result<Self> {
        let mut tree = BPlusTree {
            root: Node::new_leaf(),
            file_path: file_path.to_string(),
        };

        // Load existing records if file exists
        if Path::new(file_path).exists() {
            tree.load_records()?;
        }

        Ok(tree)
    }

    fn load_records(&mut self) -> io::Result<()> {
        let mut file = File::open(&self.file_path)?;
        let mut data = String::new();
        file.read_to_string(&mut data)?;

        for line in data.lines() {
            let parts: Vec<&str> = line.split(',').collect();
            if parts.len() == 2 {
                let key = parts[0].parse::<i32>().unwrap();
                let value = parts[1].to_string();
                self.insert(key, value)?;
            }
        }

        Ok(())
    }

    fn save_records(&self) -> io::Result<()> {
        let mut file = OpenOptions::new()
            .write(true)
            .truncate(true)
            .create(true)
            .open(&self.file_path)?;

        let records = self.get_all_records();
        for record in records {
            writeln!(file, "{},{}", record.key, record.value)?;
        }

        Ok(())
    }

    fn insert(&mut self, key: i32, value: String) -> io::Result<()> {
        let record = Record { key, value };
        if let Some(split) = Self::insert_rec(&mut self.root, record) {
            let mut new_root = Node::new_internal();
            new_root.keys.push(split.key);
            new_root.children.push(self.root.clone());
            new_root.children.push(split.node);
            self.root = new_root;
        }

        // Save changes to the .db file
        self.save_records()?;

        Ok(())
    }

    fn insert_rec(node: &mut Node, record: Record) -> Option<SplitResult> {
        if node.is_leaf {
            let pos = node.keys.iter().position(|&k| k >= record.key).unwrap_or(node.keys.len());
            node.keys.insert(pos, record.key);
            node.records.insert(pos, record);

            if node.keys.len() > ORDER - 1 {
                let split_pos = node.keys.len() / 2;
                let split_key = node.keys[split_pos];

                let mut new_node = Node::new_leaf();
                new_node.keys = node.keys.drain(split_pos..).collect();
                new_node.records = node.records.drain(split_pos..).collect();
                new_node.next = node.next.take();
                node.next = Some(Box::new(new_node.clone()));

                Some(SplitResult { key: split_key, node: new_node })
            } else {
                None
            }
        } else {
            let pos = node.keys.iter().position(|&k| k > record.key).unwrap_or(node.keys.len());
            if let Some(split) = Self::insert_rec(&mut node.children[pos], record) {
                node.keys.insert(pos, split.key);
                node.children.insert(pos + 1, split.node);

                if node.keys.len() > ORDER - 1 {
                    let split_pos = node.keys.len() / 2;
                    let split_key = node.keys[split_pos];

                    let mut new_node = Node::new_internal();
                    new_node.keys = node.keys.drain(split_pos + 1..).collect();
                    new_node.children = node.children.drain(split_pos + 1..).collect();

                    Some(SplitResult { key: split_key, node: new_node })
                } else {
                    None
                }
            } else {
                None
            }
        }
    }

    fn search(&self, key: i32) -> Option<String> {
        let mut current = &self.root;
        while !current.is_leaf {
            let pos = current.keys.iter().position(|&k| k > key)
                .unwrap_or(current.keys.len());
            current = &current.children[pos];
        }

        current.keys.iter()
            .position(|&k| k == key)
            .map(|pos| current.records[pos].value.clone())
    }

    fn delete(&mut self, key: i32) -> io::Result<bool> {
        let deleted = Self::delete_rec(&mut self.root, key);

        // Save changes to the .db file
        if deleted {
            self.save_records()?;
        }

        Ok(deleted)
    }

    fn delete_rec(node: &mut Node, key: i32) -> bool {
        if node.is_leaf {
            if let Some(pos) = node.keys.iter().position(|&k| k == key) {
                node.keys.remove(pos);
                node.records.remove(pos);
                true
            } else {
                false
            }
        } else {
            let pos = node.keys.iter().position(|&k| k > key)
                .unwrap_or(node.keys.len());
            Self::delete_rec(&mut node.children[pos], key)
        }
    }

    fn get_all_records(&self) -> Vec<Record> {
        let mut records = Vec::new();
        let mut current = &self.root;

        // Find leftmost leaf
        while !current.is_leaf {
            current = &current.children[0];
        }

        // Collect all records from linked leaves
        loop {
            records.extend(current.records.clone());
            if let Some(next_node) = &current.next {
                current = next_node;
            } else {
                break;
            }
        }

        records
    }
}

// Define the SplitResult struct
struct SplitResult {
    key: i32,
    node: Node,
}

fn main() -> io::Result<()> {
    // Get database name from user
    println!("Enter database name:");
    let mut db_name = String::new();
    io::stdin().read_line(&mut db_name)?;
    let db_name = db_name.trim();
    let file_path = format!("{}.db", db_name);

    // Initialize B+ Tree with specified database
    let mut tree = BPlusTree::new(&file_path)?;
    println!("Using database: {}", file_path);
    println!("B+ Tree Database (Order {})", ORDER);
    println!("Commands:");
    println!("  insert <key> <value>  - Insert a new record");
    println!("  select                - List all records");
    println!("  select <key>          - Find specific record");
    println!("  delete <key>          - Delete a record");
    println!("  exit                  - Quit the program");

    // Command loop
    loop {
        let mut input = String::new();
        io::stdin().read_line(&mut input).unwrap();
        let parts: Vec<&str> = input.trim().split_whitespace().collect();

        match parts.as_slice() {
            ["insert", key, value] => {
                if let Ok(key) = key.parse::<i32>() {
                    if let Err(e) = tree.insert(key, value.to_string()) {
                        eprintln!("Error inserting record: {}", e);
                    } else {
                        println!("Inserted: {} => {}", key, value);
                    }
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
                }
            }
            ["delete", key] => {
                if let Ok(key) = key.parse::<i32>() {
                    match tree.delete(key) {
                        Ok(true) => println!("Deleted key {}", key),
                        Ok(false) => println!("Key {} not found", key),
                        Err(e) => eprintln!("Error deleting record: {}", e),
                    }
                }
            }
            ["exit"] => break,
            _ => println!("Invalid command"),
        }
    }

    Ok(())
}