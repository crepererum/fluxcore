#![feature(default_type_params)]
#![feature(phase)]
#![feature(unsafe_destructor)]

// required for plugin stuff
extern crate serialize;

#[phase(plugin)] extern crate cfor;
extern crate cgmath;
extern crate csv;
#[phase(plugin)] extern crate docopt_macros;
extern crate docopt;
extern crate freetype;
extern crate gl;
extern crate glfw;
extern crate graphics;
extern crate hgl;
extern crate native;
extern crate opengl_graphics;

use std::collections::HashMap;
use std::collections::TreeSet;
use std::io::stdio;
use std::iter::FromIterator;
use std::num::Float;
use std::path::Path;
use std::vec::Vec;

mod data;
mod render;
mod textdrawer;

#[start]
fn start(argc: int, argv: *const *const u8) -> int {
    native::start(argc, argv, main)
}

docopt!(Args,"
Usage: fluxcore [options] FILE [X Y Z]
       fluxcore (--help)

Options:
    --separator SEPARATOR   Sets seperator.
    -h, --help              Print help.
")

fn main() {
    let args: Args = docopt::FlagParser::parse().unwrap_or_else(|e| e.exit());

    let path = Path::new(args.arg_FILE);
    let mut reader = csv::Decoder::from_file(&path);
    reader.has_headers(true);
    if !args.flag_separator.is_empty() {
        let mut s = args.flag_separator.clone();
        reader.separator(s.shift_char().unwrap());
    }

    let columns: TreeSet<String> = FromIterator::from_iter(reader.headers().unwrap().move_iter());
    let mut table = data::Table::new(path.as_str().unwrap().to_string(), columns);

    let mut positions: HashMap<uint, uint> = HashMap::new();
    for (orig_pos, orig_value) in reader.headers().unwrap().iter().enumerate() {
        let (target_pos, _target_value) = table.columns().iter().enumerate().find(|x| x.val1() == orig_value).unwrap();
        positions.insert(target_pos, orig_pos);
    }

    let mut n: uint = 0;
    let mut rowVec: Vec<f32> = Vec::new();
    rowVec.grow(positions.len(), &0f32);
    for row in reader.decode_iter::<Vec<Option<f32>>>() {
        assert!(row.len() == rowVec.len());
        for x in range(0, row.len()) {
            let value = row[positions.find(&x).unwrap().clone()];
            *rowVec.get_mut(x) = match value {
                Some(value) => value,
                None => Float::nan()
            };
        }
        table.push(&rowVec);

        if n % 100 == 0 {
            print!("\rParsed {} lines", n);
            stdio::flush();
        }
        n += 1;
    }
    println!("\rParsed {} lines", n);

    println!("Render!");
    let dimx = if args.arg_X.is_empty() {
        table.columns().iter().next().unwrap().clone()
    } else {
        args.arg_X
    };
    let dimy = if args.arg_Y.is_empty() {
        match table.columns().iter().skip(1).next() {
            Some(s) => s,
            None => &dimx
        }.clone()
    } else {
        args.arg_Y
    };
    let dimz = if args.arg_Z.is_empty() {
        match table.columns().iter().skip(2).next() {
            Some(s) => s,
            None => &dimy
        }.clone()
    } else {
        args.arg_Z
    };
    render::render(table, &dimx, &dimy, &dimz);
}
