use std::{env, fs};
use std::collections::HashMap;

use ase;
use ase::{Cel, ChunkData, ColorDepth, RGBA256, CelChunk};
use ase::Cel::RawCel;
use simple_logger::SimpleLogger;

use log::{info, warn};

// TODO maybe we want a way to ignore alpha?
type PaletteMap = HashMap<RGBA256, RGBA256>;

/// Apply a palette swap to the pixels in a cel
/// Requires that the cel is in RGBA format, otherwise everything will probably break horribly.
/// If the cel is a LinkedCel, has no effect.
fn palette_swap_cel(/*palette: &PaletteMap, */chunk: &mut CelChunk) {
	let cel = &mut chunk.cel;
	// TODO can't mutate Cel's pixels in place; we'll have to return a new Cel
	use Cel::*;
	match cel {
		RawCel {..} | CompressedImage {..} => {
			match cel.pixels(ColorDepth.RGBA) {
				None => {
					// TODO actually useful warn message
					warn!("Failed to get pixels from cel");
				},
				Some(pixels) => {

				}
			}
		},
		_ => {}
	}
}

fn main() {
	SimpleLogger::new().init().unwrap_or_else(|e| eprintln!("Failed to create logger: {}", e));

	let file_name = env::args().skip(1).next().unwrap();
	let mut file = fs::File::open(file_name).unwrap();
	let mut ase = ase::Aseprite::from_read(&mut file).unwrap();
	drop(file);
	println!("{:#?}", ase.header);
	for frame in ase.frames.iter_mut() {
		for chunk in frame.chunks.iter_mut() {
			let data = &mut chunk.chunk_data;
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
				CelChunk(data) => {
					println!("Found cel chunk");
					// let cel = &mut data.cel;
					// println!("{:?}", cel);
					palette_swap_cel(data);
				}
				_ => {}
			}
		}
	}
}
