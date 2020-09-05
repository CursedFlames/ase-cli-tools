use ase;
use std::{env, fs};

fn main() {
	let file_name = env::args().skip(1).next().unwrap();
	let mut file = fs::File::open(file_name).unwrap();
	let ase = ase::Aseprite::from_read(&mut file).unwrap();
	println!("{:#?}", ase);
}
