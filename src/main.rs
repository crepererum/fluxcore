#![feature(phase)]

// required for plugin stuff
extern crate serialize;

extern crate cgmath;
extern crate csv;
#[phase(plugin)] extern crate docopt_macros;
extern crate docopt;
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

docopt!(Args,"
Usage: fluxcore [options] FILE X Y
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

    let mut columns = TreeSet::new();
    columns.extend(reader.headers().unwrap().move_iter());

    let mut table = data::Table::new(path.as_str().unwrap().to_string(), columns);
    for row in reader.decode_iter::<Vec<f32>>() {
        table.push(row);
    }

    render::render(table, &args.arg_X, &args.arg_Y);
}
