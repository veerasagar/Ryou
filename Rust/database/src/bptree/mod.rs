use std::io;

#[derive(Debug, Clone)]
pub struct Record {
    pub key: i32,
    pub value: String,
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

pub struct BPlusTree {
    root: Node,
}

impl BPlusTree {
    pub fn new() -> Self {
        BPlusTree {
            root: Node::new_leaf(),
        }
    }

    pub fn insert(&mut self, key: i32, value: String) {
        let record = Record { key, value };
        if let Some(split) = Self::insert_rec(&mut self.root, record) {
            let mut new_root = Node::new_internal();
            new_root.keys.push(split.key);
            new_root.children.push(self.root.clone());
            new_root.children.push(split.node);
            self.root = new_root;
        }
    }

    fn insert_rec(node: &mut Node, record: Record) -> Option<SplitResult> {
        if node.is_leaf {
            let pos = node
                .keys
                .iter()
                .position(|&k| k >= record.key)
                .unwrap_or(node.keys.len());
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

                Some(SplitResult {
                    key: split_key,
                    node: new_node,
                })
            } else {
                None
            }
        } else {
            let pos = node
                .keys
                .iter()
                .position(|&k| k > record.key)
                .unwrap_or(node.keys.len());
            if let Some(split) = Self::insert_rec(&mut node.children[pos], record) {
                node.keys.insert(pos, split.key);
                node.children.insert(pos + 1, split.node);

                if node.keys.len() > ORDER - 1 {
                    let split_pos = node.keys.len() / 2;
                    let split_key = node.keys[split_pos];

                    let mut new_node = Node::new_internal();
                    new_node.keys = node.keys.drain(split_pos + 1..).collect();
                    new_node.children = node.children.drain(split_pos + 1..).collect();

                    Some(SplitResult {
                        key: split_key,
                        node: new_node,
                    })
                } else {
                    None
                }
            } else {
                None
            }
        }
    }

    pub fn search(&self, key: i32) -> Option<String> {
        let mut current = &self.root;
        while !current.is_leaf {
            let pos = current
                .keys
                .iter()
                .position(|&k| k > key)
                .unwrap_or(current.keys.len());
            current = &current.children[pos];
        }

        current
            .keys
            .iter()
            .position(|&k| k == key)
            .map(|pos| current.records[pos].value.clone())
    }

    pub fn delete(&mut self, key: i32) -> bool {
        Self::delete_rec(&mut self.root, key)
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
            let pos = node
                .keys
                .iter()
                .position(|&k| k > key)
                .unwrap_or(node.keys.len());
            Self::delete_rec(&mut node.children[pos], key)
        }
    }

    pub fn get_all_records(&self) -> Vec<Record> {
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

const ORDER: usize = 4;

struct SplitResult {
    key: i32,
    node: Node,
}