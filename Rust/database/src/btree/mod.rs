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
    values: Vec<String>,
    is_leaf: bool,
}

impl Node {
    fn new_leaf() -> Self {
        Node {
            keys: Vec::new(),
            children: Vec::new(),
            values: Vec::new(),
            is_leaf: true,
        }
    }

    fn new_internal() -> Self {
        Node {
            keys: Vec::new(),
            children: Vec::new(),
            values: Vec::new(),
            is_leaf: false,
        }
    }
}

pub struct BTree {
    root: Node,
}

impl BTree {
    pub fn new() -> Self {
        BTree {
            root: Node::new_leaf(),
        }
    }

    pub fn insert(&mut self, key: i32, value: String) {
        if let Some(split) = Self::insert_rec(&mut self.root, key, value) {
            let mut new_root = Node::new_internal();
            new_root.keys.push(split.key);
            new_root.values.push(split.value);
            new_root.children.push(self.root.clone());
            new_root.children.push(split.node);
            self.root = new_root;
        }
    }

    fn insert_rec(node: &mut Node, key: i32, value: String) -> Option<SplitResult> {
        // Find the position to insert
        let pos = node
            .keys
            .iter()
            .position(|&k| k >= key)
            .unwrap_or(node.keys.len());
        
        // If we found the exact key, just update the value
        if pos < node.keys.len() && node.keys[pos] == key {
            node.values[pos] = value;
            return None;
        }
        
        if node.is_leaf {
            // Insert in leaf node
            node.keys.insert(pos, key);
            node.values.insert(pos, value);
        } else {
            // Insert in internal node by recursively inserting into appropriate child
            if let Some(split) = Self::insert_rec(&mut node.children[pos], key, value) {
                node.keys.insert(pos, split.key);
                node.values.insert(pos, split.value);
                node.children.insert(pos + 1, split.node);
            } else {
                return None;
            }
        }

        // Check if node needs to be split
        if node.keys.len() > ORDER - 1 {
            let split_pos = node.keys.len() / 2;
            let split_key = node.keys[split_pos];
            let split_value = node.values[split_pos].clone();

            let mut new_node = if node.is_leaf {
                Node::new_leaf()
            } else {
                Node::new_internal()
            };

            // Move half keys and values to the new node
            new_node.keys = node.keys.drain(split_pos + 1..).collect();
            new_node.values = node.values.drain(split_pos + 1..).collect();

            // For internal nodes, also move children
            if !node.is_leaf {
                new_node.children = node.children.drain(split_pos + 1..).collect();
            }

            // Remove the middle key/value that will move up to the parent
            node.keys.pop();
            node.values.pop();

            Some(SplitResult {
                key: split_key,
                value: split_value,
                node: new_node,
            })
        } else {
            None
        }
    }

    pub fn search(&self, key: i32) -> Option<String> {
        Self::search_rec(&self.root, key)
    }

    fn search_rec(node: &Node, key: i32) -> Option<String> {
        let pos = node
            .keys
            .iter()
            .position(|&k| k >= key)
            .unwrap_or(node.keys.len());

        // If we found the key
        if pos < node.keys.len() && node.keys[pos] == key {
            return Some(node.values[pos].clone());
        }

        // If this is a leaf and we didn't find the key, it doesn't exist
        if node.is_leaf {
            return None;
        }

        // Search in the appropriate child
        Self::search_rec(&node.children[pos], key)
    }

    pub fn delete(&mut self, key: i32) -> bool {
        let result = Self::delete_rec(&mut self.root, key);
        
        // If the root has no keys and is not a leaf, make its only child the new root
        if self.root.keys.is_empty() && !self.root.is_leaf {
            if !self.root.children.is_empty() {
                self.root = self.root.children.remove(0);
            }
        }
        
        result
    }

    fn delete_rec(node: &mut Node, key: i32) -> bool {
        // Find position of key or where it should be
        let pos = node
            .keys
            .iter()
            .position(|&k| k >= key)
            .unwrap_or(node.keys.len());

        // Case 1: Key found in this node
        if pos < node.keys.len() && node.keys[pos] == key {
            if node.is_leaf {
                // Simply remove key and value from leaf
                node.keys.remove(pos);
                node.values.remove(pos);
                return true;
            } else {
                // Handle deletion from internal node
                return Self::delete_from_internal_node(node, pos);
            }
        }
        
        // Case 2: Key not found in this node
        if node.is_leaf {
            // Key not in tree
            return false;
        } else {
            // Try to delete from child
            let min_keys = (ORDER - 1) / 2;
            let child_needs_rebalance = node.children[pos].keys.len() <= min_keys;
            
            // Ensure child has enough keys before recursing
            if child_needs_rebalance {
                Self::ensure_child_has_min_keys(node, pos);
            }
            
            // If the child at pos was merged, we need to adjust pos
            let pos = if pos > 0 && pos >= node.children.len() {
                pos - 1
            } else {
                pos
            };
            
            Self::delete_rec(&mut node.children[pos], key)
        }
    }
    
    fn delete_from_internal_node(node: &mut Node, pos: usize) -> bool {
        let key = node.keys[pos];
        
        // Case 1: If predecessor child has at least min_keys + 1 keys, replace with predecessor
        if node.children[pos].keys.len() > (ORDER - 1) / 2 {
            let (pred_key, pred_value) = Self::get_predecessor(&mut node.children[pos]);
            node.keys[pos] = pred_key;
            node.values[pos] = pred_value;
            return Self::delete_rec(&mut node.children[pos], pred_key);
        }
        
        // Case 2: If successor child has at least min_keys + 1 keys, replace with successor
        if node.children[pos + 1].keys.len() > (ORDER - 1) / 2 {
            let (succ_key, succ_value) = Self::get_successor(&mut node.children[pos + 1]);
            node.keys[pos] = succ_key;
            node.values[pos] = succ_value;
            return Self::delete_rec(&mut node.children[pos + 1], succ_key);
        }
        
        // Case 3: If both children have min_keys, merge them and delete
        Self::merge_children(node, pos);
        return Self::delete_rec(&mut node.children[pos], key);
    }
    
    fn get_predecessor(node: &mut Node) -> (i32, String) {
        let mut current = node;
        while !current.is_leaf {
            let last_idx = current.children.len() - 1;
            current = &mut current.children[last_idx];
        }
        let last_idx = current.keys.len() - 1;
        (current.keys[last_idx], current.values[last_idx].clone())
    }
    
    fn get_successor(node: &mut Node) -> (i32, String) {
        let mut current = node;
        while !current.is_leaf {
            current = &mut current.children[0];
        }
        (current.keys[0], current.values[0].clone())
    }
    
    fn ensure_child_has_min_keys(node: &mut Node, child_pos: usize) {
        let min_keys = (ORDER - 1) / 2;
        
        // Try to borrow from left sibling
        if child_pos > 0 {
            let left_has_extra = node.children[child_pos - 1].keys.len() > min_keys;
            
            if left_has_extra {
                Self::borrow_from_left(node, child_pos);
                return;
            }
        }
        
        // Try to borrow from right sibling
        if child_pos < node.children.len() - 1 {
            let right_has_extra = node.children[child_pos + 1].keys.len() > min_keys;
            
            if right_has_extra {
                Self::borrow_from_right(node, child_pos);
                return;
            }
        }
        
        // Merge with a sibling if borrowing is not possible
        if child_pos > 0 {
            Self::merge_children(node, child_pos - 1);
        } else {
            Self::merge_children(node, child_pos);
        }
    }
    
    fn borrow_from_left(node: &mut Node, child_pos: usize) {
        // Get parent key/value
        let parent_key = node.keys[child_pos - 1];
        let parent_value = node.values[child_pos - 1].clone();
        
        // Use split_at_mut to get mutable references to both children
        let (left_slice, right_slice) = node.children.split_at_mut(child_pos);
        let left = &mut left_slice[left_slice.len() - 1]; // Last element in left_slice
        let right = &mut right_slice[0]; // First element in right_slice
        
        // Move parent key/value down to right child
        right.keys.insert(0, parent_key);
        right.values.insert(0, parent_value);
        
        // If internal node, move child as well
        if !right.is_leaf {
            let last_child = left.children.pop().unwrap();
            right.children.insert(0, last_child);
        }
        
        // Move last key/value from left sibling up to parent
        let last_idx = left.keys.len() - 1;
        node.keys[child_pos - 1] = left.keys.remove(last_idx);
        node.values[child_pos - 1] = left.values.remove(last_idx);
    }
    
    fn borrow_from_right(node: &mut Node, child_pos: usize) {
        // Get parent key/value
        let parent_key = node.keys[child_pos];
        let parent_value = node.values[child_pos].clone();
        
        // Use split_at_mut to get mutable references to both children
        let (left_slice, right_slice) = node.children.split_at_mut(child_pos + 1);
        let left = &mut left_slice[left_slice.len() - 1]; // Last element in left_slice
        let right = &mut right_slice[0]; // First element in right_slice
        
        // Move parent key/value down to left child
        left.keys.push(parent_key);
        left.values.push(parent_value);
        
        // If internal node, move child as well
        if !left.is_leaf {
            let first_child = right.children.remove(0);
            left.children.push(first_child);
        }
        
        // Move first key/value from right sibling up to parent
        node.keys[child_pos] = right.keys.remove(0);
        node.values[child_pos] = right.values.remove(0);
    }
    
    fn merge_children(node: &mut Node, left_pos: usize) {
        // Get parent key/value to be merged down
        let parent_key = node.keys[left_pos];
        let parent_value = node.values[left_pos].clone();
        
        // Clone the right node before removing it
        let right_node = node.children[left_pos + 1].clone();
        
        // Remove right child and parent key/value
        node.keys.remove(left_pos);
        node.values.remove(left_pos);
        node.children.remove(left_pos + 1);
        
        // Access the left child after the removals
        let left = &mut node.children[left_pos];
        
        // Add parent key/value to left child
        left.keys.push(parent_key);
        left.values.push(parent_value);
        
        // Add all keys/values from right child
        left.keys.extend(right_node.keys);
        left.values.extend(right_node.values);
        
        // If internal node, also merge children
        if !left.is_leaf {
            left.children.extend(right_node.children);
        }
    }

    pub fn get_all_records(&self) -> Vec<Record> {
        let mut records = Vec::new();
        Self::collect_records(&self.root, &mut records);
        records
    }
    
    fn collect_records(node: &Node, records: &mut Vec<Record>) {
        for i in 0..node.keys.len() {
            if !node.is_leaf {
                Self::collect_records(&node.children[i], records);
            }
            records.push(Record {
                key: node.keys[i],
                value: node.values[i].clone(),
            });
        }
        
        // Process the last child for internal nodes
        if !node.is_leaf && !node.children.is_empty() {
            Self::collect_records(&node.children[node.children.len() - 1], records);
        }
    }

    
}

const ORDER: usize = 4;

struct SplitResult {
    key: i32,
    value: String,
    node: Node,
}