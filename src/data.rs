use std::collections::{TreeMap, TreeSet};

struct Column {
    name: String,
    data: Vec<f32>,
}

pub struct Table {
    name: String,
    colnames: TreeSet<String>,
    columns: TreeMap<String, Column>,
}

impl Table {
    pub fn new(name: String, columns: TreeSet<String>) -> Table {
        assert!(!columns.is_empty());

        let mut columnMap = TreeMap::new();
        for c in columns.iter() {
            columnMap.insert(c.clone(), Column{name: c.clone(), data: Vec::new()});
        }

        Table {
            name: name,
            colnames: columns,
            columns: columnMap
        }
    }

    pub fn push(&mut self, row: &Vec<f32>) {
        assert!(row.len() == self.colnames.len());
        for (name, data) in self.colnames.iter().zip(row.iter()) {
            self.columns.find_mut(name).unwrap().data.push(data.clone())
        }
    }

    pub fn get<'a>(&'a self, column: &String) -> Option<&'a Vec<f32>> {
        match self.columns.find(column) {
            Some(c) => Some(&(c.data)),
            None => None
        }
    }

    pub fn len(&self) -> uint {
        self.columns.iter().next().unwrap().val1().data.len()
    }

    pub fn name<'a>(&'a self) -> &'a String {
        &self.name
    }

    pub fn columns<'a>(&'a self) -> &'a TreeSet<String> {
        &self.colnames
    }
}

