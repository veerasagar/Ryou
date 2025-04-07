#!/bin/bash

echo "Please choose a language to run:"
echo "1. C"
echo "2. Rust"

read -p "Enter your choice (1 or 2): " choice

case $choice in
    1)
        echo "Running in C..."
        cd ./C && gcc db.c
        read -p "Enter your Database name: " db_name
        ./a.out $db_name.db
        ;;
    2)
        echo "Running in Rust..."
        cd ./Rust/database && cargo run
        ;;
    *)
        echo "Invalid choice. Please enter 1 or 2."
        ;;
esac
