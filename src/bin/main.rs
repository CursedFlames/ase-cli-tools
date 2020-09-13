use ase;
use ase::ChunkData;
use std::{env, fs};

fn main() {
	let file_name = env::args().skip(1).next().unwrap();
	let mut file = fs::File::open(file_name).unwrap();
	let ase = ase::Aseprite::from_read(&mut file).unwrap();
	println!("{:#?}", ase.header);
	for frame in ase.frames.iter() {
		for chunk in frame.chunks.iter() {
			let data = &chunk.chunk_data;
			use ChunkData::*;
			match data {
				OldPaletteChunk4(_) => {
					println!("Found old palette chunk 4");
				},
				OldPaletteChunk11(_) => {

					println!("Found old palette chunk 11");
				},
				PaletteChunk(data) => {
					println!("Found palette chunk");
					println!("{:?}", data);
				},
				_ => {}
			}
		}
	}
}
