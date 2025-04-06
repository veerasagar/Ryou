from flask import Flask, request, render_template, redirect, url_for, flash
import os

app = Flask(__name__)
app.secret_key = 'your_secret_key'

# Database file – this is analogous to your Rust .db file.
DB_FILE = 'mydb.db'

def get_all_records():
    records = []
    if os.path.exists(DB_FILE):
        with open(DB_FILE, 'r') as f:
            for line in f:
                parts = line.strip().split(',')
                if len(parts) == 2:
                    # Store key as integer and value as string
                    try:
                        records.append({'key': int(parts[0]), 'value': parts[1]})
                    except ValueError:
                        pass
    return records

def save_record(key, value):
    # Append a new record to the database file.
    with open(DB_FILE, 'a') as f:
        f.write(f"{key},{value}\n")

def delete_record(key):
    if not os.path.exists(DB_FILE):
        return False
    records = get_all_records()
    new_records = [r for r in records if r['key'] != key]
    if len(new_records) == len(records):
        return False  # key not found
    with open(DB_FILE, 'w') as f:
        for r in new_records:
            f.write(f"{r['key']},{r['value']}\n")
    return True

@app.route('/')
def index():
    records = get_all_records()
    return render_template('index.html', records=records)

@app.route('/insert', methods=['POST'])
def insert():
    key = request.form.get('key')
    value = request.form.get('value')
    if not key or not value:
        flash("Both key and value are required.")
        return redirect(url_for('index'))
    try:
        key_int = int(key)
    except ValueError:
        flash("Key must be an integer.")
        return redirect(url_for('index'))
    save_record(key_int, value)
    flash(f"Inserted record: {key_int} => {value}")
    return redirect(url_for('index'))

@app.route('/delete/<int:key>', methods=['POST'])
def delete(key):
    if delete_record(key):
        flash(f"Deleted record with key: {key}")
    else:
        flash(f"Record with key {key} not found.")
    return redirect(url_for('index'))

if __name__ == '__main__':
    app.run(debug=True)