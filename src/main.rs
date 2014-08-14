extern crate cgmath;
extern crate csv;
extern crate gl;
extern crate glfw;
extern crate hgl;
extern crate native;

use std::collections::TreeSet;
use std::path::Path;

mod data;
mod render;

#[start]
fn start(argc: int, argv: *const *const u8) -> int {
    native::start(argc, argv, main)
}

fn main() {
    let path = Path::new("./input.csv");
    let mut reader = csv::Decoder::from_file(&path);
    reader.has_headers(true);

    let mut columns = TreeSet::new();
    columns.extend(reader.headers().unwrap().move_iter());

    let mut table = data::Table::new(path.as_str().unwrap().to_string(), columns);
    for row in reader.decode_iter::<Vec<f32>>() {
        table.push(row);
    }

    render::render(table, &"x".to_string(), &"y".to_string());
}
