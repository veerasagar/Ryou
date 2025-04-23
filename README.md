# Ryou - A Simple Database

## Using C

To run database

```c
gcc db.c
```

To start database

```c
./a.out test.db
```

To insert

```c
db > insert 1 test test@test.com
```

To select

```c
db > select
```

To delete

```c
db > delete 1
```

To exit

```c
db > .exit
```

## Using Rust

To run

```rust
cd Rust\database
cargo build
cargo run
cargo run -- --cli
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

![Alt text](https://github.com/veerasagar/Ryou/blob/main/static/img.png)
