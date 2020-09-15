use std::{env, fs};
use std::collections::HashMap;

use ase;
use ase::{Cel, CelChunk, ChunkData, ColorDepth, RGBA256, Pixels};
use ase::Cel::{RawCel, CompressedImage};
use flate2::Compression;
use flate2::write::ZlibEncoder;
use simple_logger::SimpleLogger;

use log::{info, warn};

// TODO maybe we want a way to ignore alpha?
type PaletteMap = HashMap<RGBA256, RGBA256>;

fn transform_pixels<F>(chunk: &mut CelChunk, func: F)
		where F: FnOnce(&mut Pixels) {
	let cel = &mut chunk.cel;
	println!("{:?}", cel);
	match cel {
		RawCel { ref mut pixels, .. } => {
			func(pixels);
		},
		CompressedImage { ref mut zlib_compressed_data, .. } => {
			let mut pixels = Cel::decompress_pixels(&zlib_compressed_data, &ColorDepth::RGBA);
			func(&mut pixels);
			let v = Vec::new();
			let mut z = ZlibEncoder::new(v, Compression::default());
			pixels.write(&mut z);
			match z.finish() {
				Ok(data) => {
					*zlib_compressed_data = data;
				},
				Err(e) => {
					warn!("Error during zlib compression, skipping cel:\n{:?}", e);
				}
			}
		},
		// Ignore linked cels
		_ => {}
	};
	println!("{:?}", cel);
}

fn main() {
	SimpleLogger::new().init().unwrap_or_else(|e| eprintln!("Failed to create logger: {}", e));

	let mut args = env::args().skip(1);
	let file_name = args.next().unwrap();
	let out_name = args.next().unwrap();
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
					// palette_swap_cel(data);
					transform_pixels(data, |p| {
						match p {
							Pixels::RGBA(p) => {
								for pixel in p.iter_mut() {
									pixel.r /= 2;
									pixel.g /= 2;
									pixel.b /= 2;
								}
							}
							_ => {}
						}
					});
				}
				_ => {}
			}
		}
	}
	let mut file = fs::File::create(out_name).unwrap();
	ase.write(&mut file);
}
