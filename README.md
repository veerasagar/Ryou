# Ryou - A Simple Database

## Using C

To run database

```
gcc db.c
```

To start database

```
./a.out test.db
```

To insert

```
db > insert 1 test test@test.com
```

To select

```
db > select
```

To delete

```
db > delete 1
```

To exit

```
db > .exit
```

## Using Rust

To run

```rust
cd Rust\database
cargo build
cargo run
```

```text
Enter database name:
mydatabase
Using database: mydatabase.db
B+ Tree Database (Order 4)
Commands:
  insert <key> <value>  - Insert a new record
  select                - List all records
  select <key>          - Find specific record
  delete <key>          - Delete a record
  exit                  - Quit the program
```

## For Linux users

To run

```bash
./run.sh
```
