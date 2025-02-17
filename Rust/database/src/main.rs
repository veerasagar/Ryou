use std::fs::{File, OpenOptions};
use std::io::{self, Read, Seek, SeekFrom, Write};
use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};

const PAGE_SIZE: usize = 4096;
const TABLE_MAX_PAGES: usize = 400;
const ID_SIZE: usize = 4;
const USERNAME_SIZE: usize = 32;
const EMAIL_SIZE: usize = 255;
const ROW_SIZE: usize = ID_SIZE + USERNAME_SIZE + EMAIL_SIZE;

#[derive(Debug)]
enum ExecuteResult {
    Success,
    DuplicateKey,
    TableFull,
    NotFound,
}

#[derive(Debug)]
enum PrepareResult {
    Success,
    NegativeId,
    StringTooLong,
    SyntaxError,
    UnrecognizedStatement,
}

#[derive(Debug)]
enum StatementType {
    Insert,
    Select,
    Delete,
}

#[derive(Debug)]
struct Row {
    id: u32,
    username: [u8; USERNAME_SIZE + 1],
    email: [u8; EMAIL_SIZE + 1],
}

#[derive(Debug)]
struct Statement {
    stype: StatementType,
    row_to_insert: Row,
    row_id_to_delete: u32,
}

#[derive(Debug)]
struct Pager {
    file: File,
    file_length: u64,
    num_pages: u32,
    pages: Vec<Option<Box<[u8; PAGE_SIZE]>>>,
}

impl Pager {
    fn new(filename: &str) -> io::Result<Self> {
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .open(filename)?;

        let file_length = file.metadata()?.len();
        let num_pages = (file_length / PAGE_SIZE as u64) as u32;
        let mut pages = vec![None; TABLE_MAX_PAGES];

        if file_length == 0 {
            let mut page = Box::new([0u8; PAGE_SIZE]);
            initialize_leaf_node(&mut page);
            pages[0] = Some(page);
        }

        Ok(Pager {
            file,
            file_length,
            num_pages,
            pages,
        })
    }

    fn get_page(&mut self, page_num: usize) -> io::Result<&mut [u8; PAGE_SIZE]> {
        if page_num >= TABLE_MAX_PAGES {
            panic!("Page number out of bounds");
        }

        if self.pages[page_num].is_none() {
            let mut page = Box::new([0u8; PAGE_SIZE]);
            let offset = page_num as u64 * PAGE_SIZE as u64;

            if offset < self.file_length {
                self.file.seek(SeekFrom::Start(offset))?;
                self.file.read_exact(&mut *page)?;
            }

            self.pages[page_num] = Some(page);
        }

        Ok(self.pages[page_num].as_mut().unwrap())
    }

    fn flush_page(&mut self, page_num: usize) -> io::Result<()> {
        if let Some(page) = &self.pages[page_num] {
            let offset = page_num as u64 * PAGE_SIZE as u64;
            self.file.seek(SeekFrom::Start(offset))?;
            self.file.write_all(&**page)?;
        }
        Ok(())
    }
}

#[derive(Debug)]
struct Table {
    pager: Pager,
    root_page_num: usize,
}

impl Table {
    fn new(filename: &str) -> io::Result<Self> {
        let pager = Pager::new(filename)?;
        Ok(Table {
            pager,
            root_page_num: 0,
        })
    }

    fn start(&mut self) -> Cursor {
        let root_page_num = self.root_page_num;
        Cursor {
            table: self,
            page_num: root_page_num,
            cell_num: 0,
            end_of_table: false,
        }
    }

    fn find(&mut self, key: u32) -> Cursor {
        let root_page_num = self.root_page_num;
        Cursor {
            table: self,
            page_num: root_page_num,
            cell_num: 0,
            end_of_table: false,
        }
    }
}

#[derive(Debug)]
struct Cursor<'a> {
    table: &'a mut Table,
    page_num: usize,
    cell_num: usize,
    end_of_table: bool,
}

impl<'a> Cursor<'a> {
    fn advance(&mut self) {
        let page = self.table.pager.get_page(self.page_num).unwrap();
        let num_cells = u32::from_le_bytes(page[2..6].try_into().unwrap()) as usize;

        self.cell_num += 1;
        if self.cell_num >= num_cells {
            let next_page = u32::from_le_bytes(page[6..10].try_into().unwrap());
            if next_page == 0 {
                self.end_of_table = true;
            } else {
                self.page_num = next_page as usize;
                self.cell_num = 0;
            }
        }
    }

    fn value(&mut self) -> Option<Row> {
        let page = self.table.pager.get_page(self.page_num).unwrap();
        let num_cells = u32::from_le_bytes(page[2..6].try_into().unwrap()) as usize;

        if self.cell_num >= num_cells {
            None
        } else {
            let cell_offset = LEAF_NODE_HEADER_SIZE + self.cell_num * LEAF_NODE_CELL_SIZE;
            Some(deserialize_row(&page[cell_offset..cell_offset + ROW_SIZE]))
        }
    }
}

const LEAF_NODE_HEADER_SIZE: usize = 6;
const LEAF_NODE_CELL_SIZE: usize = ROW_SIZE + 4;
const LEAF_NODE_MAX_CELLS: usize = (PAGE_SIZE - LEAF_NODE_HEADER_SIZE) / LEAF_NODE_CELL_SIZE;

fn initialize_leaf_node(page: &mut [u8; PAGE_SIZE]) {
    page[0] = 0;
    page[1] = 1;
    page[2..6].copy_from_slice(&0u32.to_le_bytes());
}

fn serialize_row(row: &Row, destination: &mut [u8]) {
    let mut dst = io::Cursor::new(destination);
    dst.write_u32::<LittleEndian>(row.id).unwrap();

    let username_bytes = &row.username[..USERNAME_SIZE];
    dst.write_all(username_bytes).unwrap();

    let email_bytes = &row.email[..EMAIL_SIZE];
    dst.write_all(email_bytes).unwrap();
}

fn deserialize_row(src: &[u8]) -> Row {
    let mut cursor = io::Cursor::new(src);
    let mut username = [0u8; USERNAME_SIZE + 1];
    let mut email = [0u8; EMAIL_SIZE + 1];

    let id = cursor.read_u32::<LittleEndian>().unwrap();
    cursor.read_exact(&mut username[..USERNAME_SIZE]).unwrap();
    cursor.read_exact(&mut email[..EMAIL_SIZE]).unwrap();

    Row {
        id,
        username,
        email,
    }
}

fn execute_insert(statement: &Statement, table: &mut Table) -> ExecuteResult {
    let row = &statement.row_to_insert;
    let key_to_insert = row.id;

    let mut cursor = table.find(key_to_insert);
    let page = cursor.table.pager.get_page(cursor.page_num).unwrap();
    let num_cells = u32::from_le_bytes(page[2..6].try_into().unwrap()) as usize;

    if cursor.cell_num < num_cells {
        let key_at_index = u32::from_le_bytes(
            page[LEAF_NODE_HEADER_SIZE + cursor.cell_num * LEAF_NODE_CELL_SIZE..]
                [..4]
                .try_into()
                .unwrap(),
        );
        if key_at_index == key_to_insert {
            return ExecuteResult::DuplicateKey;
        }
    }

    if num_cells >= LEAF_NODE_MAX_CELLS {
        return ExecuteResult::TableFull;
    }

    let cell_offset = LEAF_NODE_HEADER_SIZE + cursor.cell_num * LEAF_NODE_CELL_SIZE;

    for i in (cursor.cell_num..num_cells).rev() {
        let src_offset = LEAF_NODE_HEADER_SIZE + i * LEAF_NODE_CELL_SIZE;
        let dst_offset = LEAF_NODE_HEADER_SIZE + (i + 1) * LEAF_NODE_CELL_SIZE;
        page.copy_within(src_offset..src_offset + LEAF_NODE_CELL_SIZE, dst_offset);
    }

    let mut cell_data = [0u8; ROW_SIZE];
    serialize_row(row, &mut cell_data);
    page[cell_offset..cell_offset + ROW_SIZE].copy_from_slice(&cell_data);

    let new_num_cells = (num_cells + 1).to_le_bytes();
    page[2..6].copy_from_slice(&new_num_cells);

    ExecuteResult::Success
}

fn execute_select(table: &mut Table) -> ExecuteResult {
    let mut cursor = table.start();

    while !cursor.end_of_table {
        if let Some(row) = cursor.value() {
            println!(
                "({}, {}, {})",
                row.id,
                std::str::from_utf8(&row.username[..USERNAME_SIZE]).unwrap(),
                std::str::from_utf8(&row.email[..EMAIL_SIZE]).unwrap()
            );
        }
        cursor.advance();
    }

    ExecuteResult::Success
}

fn execute_delete(statement: &Statement, table: &mut Table) -> ExecuteResult {
    let key_to_delete = statement.row_id_to_delete;

    let mut cursor = table.find(key_to_delete);
    let page = cursor.table.pager.get_page(cursor.page_num).unwrap();
    let num_cells = u32::from_le_bytes(page[2..6].try_into().unwrap()) as usize;

    if cursor.cell_num >= num_cells {
        return ExecuteResult::NotFound;
    }

    let key_at_index = u32::from_le_bytes(
        page[LEAF_NODE_HEADER_SIZE + cursor.cell_num * LEAF_NODE_CELL_SIZE..]
            [..4]
            .try_into()
            .unwrap(),
    );
    if key_at_index != key_to_delete {
        return ExecuteResult::NotFound;
    }

    for i in cursor.cell_num..num_cells - 1 {
        let src_offset = LEAF_NODE_HEADER_SIZE + (i + 1) * LEAF_NODE_CELL_SIZE;
        let dst_offset = LEAF_NODE_HEADER_SIZE + i * LEAF_NODE_CELL_SIZE;
        page.copy_within(src_offset..src_offset + LEAF_NODE_CELL_SIZE, dst_offset);
    }

    let new_num_cells = (num_cells - 1).to_le_bytes();
    page[2..6].copy_from_slice(&new_num_cells);

    ExecuteResult::Success
}

fn prepare_insert(input: &str) -> Result<Statement, PrepareResult> {
    let parts: Vec<&str> = input.split_whitespace().collect();
    if parts.len() != 4 {
        return Err(PrepareResult::SyntaxError);
    }

    let id = parts[1].parse::<u32>().map_err(|_| PrepareResult::SyntaxError)?;
    let username = parts[2].as_bytes();
    let email = parts[3].as_bytes();

    if username.len() > USERNAME_SIZE || email.len() > EMAIL_SIZE {
        return Err(PrepareResult::StringTooLong);
    }

    let mut row = Row {
        id,
        username: [0; USERNAME_SIZE + 1],
        email: [0; EMAIL_SIZE + 1],
    };
    row.username[..username.len()].copy_from_slice(username);
    row.email[..email.len()].copy_from_slice(email);

    Ok(Statement {
        stype: StatementType::Insert,
        row_to_insert: row,
        row_id_to_delete: 0,
    })
}

fn prepare_delete(input: &str) -> Result<Statement, PrepareResult> {
    let parts: Vec<&str> = input.split_whitespace().collect();
    if parts.len() != 2 {
        return Err(PrepareResult::SyntaxError);
    }

    let id = parts[1].parse::<u32>().map_err(|_| PrepareResult::SyntaxError)?;

    Ok(Statement {
        stype: StatementType::Delete,
        row_to_insert: Row {
            id: 0,
            username: [0; USERNAME_SIZE + 1],
            email: [0; EMAIL_SIZE + 1],
        },
        row_id_to_delete: id,
    })
}

fn main() -> io::Result<()> {
    let mut table = Table::new("mydb.db")?;

    loop {
        print!("db > ");
        io::stdout().flush()?;

        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        let input = input.trim();

        match input {
            ".exit" => break,
            "select" => {
                let result = execute_select(&mut table);
                match result {
                    ExecuteResult::Success => println!("Executed successfully"),
                    _ => println!("Error executing SELECT"),
                }
            }
            _ if input.starts_with("insert") => {
                match prepare_insert(input) {
                    Ok(statement) => {
                        let result = execute_insert(&statement, &mut table);
                        match result {
                            ExecuteResult::Success => println!("Executed successfully"),
                            ExecuteResult::DuplicateKey => println!("Error: Duplicate key"),
                            ExecuteResult::TableFull => println!("Error: Table full"),
                            _ => println!("Error executing INSERT"),
                        }
                    }
                    Err(err) => match err {
                        PrepareResult::SyntaxError => println!("Syntax error"),
                        PrepareResult::StringTooLong => println!("String too long"),
                        _ => println!("Preparation error"),
                    },
                }
            }
            _ if input.starts_with("delete") => {
                match prepare_delete(input) {
                    Ok(statement) => {
                        let result = execute_delete(&statement, &mut table);
                        match result {
                            ExecuteResult::Success => println!("Executed successfully"),
                            ExecuteResult::NotFound => println!("Error: Key not found"),
                            _ => println!("Error executing DELETE"),
                        }
                    }
                    Err(err) => match err {
                        PrepareResult::SyntaxError => println!("Syntax error"),
                        _ => println!("Preparation error"),
                    },
                }
            }
            _ => println!("Unrecognized command"),
        }
    }

    for page_num in 0..TABLE_MAX_PAGES {
        table.pager.flush_page(page_num)?;
    }

    Ok(())
}
