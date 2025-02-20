use rusqlite::{Connection, params, Result};
use std::io;
use std::path::Path;

const ORDER: usize = 4;

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
    conn: Connection,
}

impl BPlusTree {
    fn new(db_name: &str) -> Result<Self> {
        let filename = if db_name.ends_with(".db") {
            db_name.to_string()
        } else {
            format!("{}.db", db_name)
        };

        let conn = Connection::open(&filename)?;

        conn.execute(
            "CREATE TABLE IF NOT EXISTS records (
                key INTEGER PRIMARY KEY,
                value TEXT NOT NULL
            )",
            [],
        )?;

        let mut tree = BPlusTree {
            root: Node::new_leaf(),
            conn,
        };
        tree.load_records()?;

        println!("Using database: {}", filename);
        Ok(tree)
    }

    fn load_records(&mut self) -> Result<()> {
        // First collect all records using a separate scope
        let records: Vec<Record> = {
            let mut stmt = self.conn.prepare("SELECT key, value FROM records")?;
            let rows = stmt.query_map([], |row| {
                Ok(Record {
                    key: row.get(0)?,
                    value: row.get(1)?,
                })
            })?;
            
            // Collect while the statement is still in scope
            rows.collect::<Result<Vec<_>>>()?
        };
    
        // Now insert with the connection available
        for record in records {
            self.insert(record.key, record.value)?;
        }
    
        Ok(())
    }

    fn insert(&mut self, key: i32, value: String) -> Result<()> {
        self.conn.execute(
            "INSERT OR REPLACE INTO records (key, value) VALUES (?1, ?2)",
            params![key, value],
        )?;

        let record = Record { key, value };
        if let Some(split) = Self::insert_rec(&mut self.root, record) {
            let mut new_root = Node::new_internal();
            new_root.keys.push(split.key);
            new_root.children.push(self.root.clone());
            new_root.children.push(split.node);
            self.root = new_root;
        }

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

    fn delete(&mut self, key: i32) -> Result<bool> {
        self.conn.execute(
            "DELETE FROM records WHERE key = ?1",
            params![key],
        )?;

        Ok(Self::delete_rec(&mut self.root, key))
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

        while !current.is_leaf {
            current = &current.children[0];
        }

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

struct SplitResult {
    key: i32,
    node: Node,
}

fn main() -> Result<()> {
    println!("B+ Tree Database (Order {})", ORDER);
    println!("Enter database name (new or existing):");

    let mut db_name = String::new();
    io::stdin().read_line(&mut db_name).unwrap();
    let db_name = db_name.trim();

    let mut tree = BPlusTree::new(db_name)?;

    println!("\nAvailable commands:");
    println!("  insert <key> <value>  - Insert a new record");
    println!("  select                - List all records");
    println!("  select <key>          - Find specific record");
    println!("  delete <key>          - Delete a record");
    println!("  exit                  - Quit the program");

    loop {
        println!("\nEnter command:");
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
