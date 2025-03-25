use std::fs::{File, OpenOptions};
use std::io::{self, Read, Write};
use std::path::Path;

use crate::bptree::Record;

pub fn load_records(file_path: &str) -> io::Result<Vec<Record>> {
    let mut records = Vec::new();

    if Path::new(file_path).exists() {
        let mut file = File::open(file_path)?;
        let mut data = String::new();
        file.read_to_string(&mut data)?;

        for line in data.lines() {
            let parts: Vec<&str> = line.split(',').collect();
            if parts.len() == 2 {
                let key = parts[0].parse::<i32>().map_err(|e| {
                    io::Error::new(
                        io::ErrorKind::InvalidData,
                        format!("Invalid key in file: {}", e),
                    )
                })?;
                let value = parts[1].to_string();
                records.push(Record { key, value });
            }
        }
    }

    Ok(records)
}

pub fn save_records(file_path: &str, records: &[Record]) -> io::Result<()> {
    let mut file = OpenOptions::new()
        .write(true)
        .truncate(true)
        .create(true)
        .open(file_path)?;

    for record in records {
        writeln!(file, "{},{}", record.key, record.value)?;
    }

    Ok(())
}