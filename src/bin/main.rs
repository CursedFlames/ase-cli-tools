use std::{env, fs};
use std::collections::HashMap;

use ase;
use ase::{Cel, CelChunk, ChunkData, ColorDepth, RGBA256, Pixels, Aseprite};
use ase::Cel::{RawCel, CompressedImage};
use flate2::Compression;
use flate2::write::ZlibEncoder;
use simple_logger::SimpleLogger;

use log::{info, warn};
use std::path::Path;

// TODO maybe we want a way to ignore alpha?
type PaletteMap = HashMap<RGBA256, RGBA256>;

fn transform_pixels<F>(chunk: &mut CelChunk, func: F)
		where F: FnOnce(&mut Pixels) {
	let cel = &mut chunk.cel;
	match cel {
		RawCel { ref mut pixels, .. } => {
			func(pixels);
		},
		CompressedImage { ref mut zlib_compressed_data, .. } => {
			let mut pixels = Cel::decompress_pixels(&zlib_compressed_data, &ColorDepth::RGBA);
			func(&mut pixels);
			match Cel::compress_pixels(&pixels) {
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
}

fn palette_swap_ase(ase: &mut Aseprite, palette: &PaletteMap) {
	for frame in ase.frames.iter_mut() {
		for chunk in frame.chunks.iter_mut() {
			let data = &mut chunk.chunk_data;
			use ChunkData::*;
			match data {
				OldPaletteChunk4(_) | OldPaletteChunk11(_) => {
					warn!("Found deprecated palette chunk, skipping");
				},
				PaletteChunk(data) => {
					println!("Found palette chunk");
					println!("{:?}", data);
				},
				CelChunk(data) => {
					transform_pixels(data, |p| {
						match p {
							Pixels::RGBA(p) => {
								for pixel in p.iter_mut() {
									if let Some(new) = palette.get(&pixel) {
										*pixel = new.clone();
									}
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
}

fn ase_to_palettemap(ase: &Aseprite) -> PaletteMap {
	let mut palette = HashMap::new();
	for frame in ase.frames.iter() {
		for chunk in frame.chunks.iter() {
			let data = &chunk.chunk_data;
			use ChunkData::*;
			match data {
				CelChunk(data) => {
					let cel = &data.cel;
					match cel {
						RawCel {..} | CompressedImage {..} => {
							let width = cel.w().unwrap() as usize;
							let height = cel.h().unwrap() as usize;
							if width < 1 || height < 2 {
								break;
							}
							if let Some(pixels) = cel.pixels(&ColorDepth::RGBA) {
								use ase::Pixels::RGBA;
								match pixels {
									RGBA(pixels) => {
										for x in 0usize..width {
											let from = pixels[x];
											let to = pixels[x+width];
											palette.insert(from, to);
										}
									},
									_ => {}
								}
							}
						},
						_ => {}
					}
				},
				_ => {}
			}
		}
	}
	palette
}

fn main() {
	SimpleLogger::new().init().unwrap_or_else(|e| eprintln!("Failed to create logger: {}", e));

	// TODO will probably want to switch to an actual argument parsing library soon
	let mut args = env::args().skip(1);
	let palette_path = args.next().expect("Missing first argument (palette file)");
	let input_path = args.next().expect("Missing second argument (input file or directory)");
	let input_path = Path::new(&input_path);
	let output_path = args.next().expect("Missing third argument (output file or directory)");

	let palette = {
		let mut file = fs::File::open(palette_path).expect("Failed to load palette file");
		let ase = Aseprite::from_read(&mut file).expect("Failed to load palette file");
		ase_to_palettemap(&ase)
	};

	if input_path.is_dir() {
		// TODO
	} else {
		let mut ase = fs::File::open(input_path).expect("Failed to load input file or directory");
		let mut ase = ase::Aseprite::from_read(&mut ase).expect("Failed to load input file");
		palette_swap_ase(&mut ase, &palette);
		// TODO prevent overwriting
		let mut file = fs::File::create(output_path).unwrap();
		ase.write(&mut file);
	}
}
