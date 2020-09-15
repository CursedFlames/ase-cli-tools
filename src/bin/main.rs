use std::{env, fs, io};
use std::collections::HashMap;
use std::ffi::OsStr;
use std::path::Path;

use ase;
use ase::{Aseprite, Cel, CelChunk, ChunkData, ColorDepth, Pixels, RGBA256};
use ase::Cel::{CompressedImage, RawCel};
use log::{debug, error, info, warn};
use simple_logger::SimpleLogger;
use walkdir::WalkDir;

// TODO maybe we want a way to ignore alpha?
type PaletteMap = HashMap<RGBA256, RGBA256>;

fn is_ase_extension(ext: Option<&OsStr>) -> bool {
	if let Some(ext) = ext {
		return ext == "aseprite" || ext == "ase"
	}
	false
}

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
					warn!("Error during zlib compression, skipping cel:\n{}", e);
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
					// warn!("Found deprecated palette chunk, skipping");
				},
				PaletteChunk(data) => {
					for entry in data.palette_entries.iter_mut() {
						if let Some(new) = palette.get(&entry.color) {
							entry.color = new.clone();
						}
					}
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
	debug!("Palette map:\n{:?}", palette);
	palette
}

fn load_palette(path: &Path) -> Result<PaletteMap, io::Error> {
	let mut file = fs::File::open(path)?;
	let ase = Aseprite::from_read(&mut file)?;
	Ok(ase_to_palettemap(&ase))
}

fn palette_swap_path(palette: &PaletteMap, input: &Path, output: &Path) -> Result<(), io::Error> {
	let mut ase = fs::File::open(input)?;
	let mut ase = ase::Aseprite::from_read(&mut ase)?;
	palette_swap_ase(&mut ase, &palette);
	let mut file = fs::OpenOptions::new().create_new(true).write(true).open(output)?;
	ase.write(&mut file)?;
	Ok(())
}

fn main() {
	SimpleLogger::new().init().unwrap_or_else(|e| eprintln!("Failed to create logger: {}", e));

	// TODO will probably want to switch to an actual argument parsing library soon
	let mut args = env::args().skip(1);
	let palette_path = args.next().expect("Missing first argument (palette file)");
	let palette_path = Path::new(&palette_path);
	let input_path = args.next().expect("Missing second argument (input file or directory)");
	let input_path = Path::new(&input_path);
	let output_path = args.next().expect("Missing third argument (output file or directory)");
	let output_path = Path::new(&output_path);
	if output_path.exists() {
		error!("Output file/directory already exists, please remove it first.");
		return;
	}

	let palette = match load_palette(&palette_path) {
		Ok(palette) => palette,
		Err(e) => {
			error!("Failed to load palette swap image:\n{}", e);
			return;
		}
	};

	if input_path.is_dir() {
		for entry in WalkDir::new(input_path).into_iter()
				.filter_map(|e| e.ok()).filter(|e| is_ase_extension(e.path().extension())) {
			let path = entry.path();
			// TODO shouldn't use this expect here
			let output_path = output_path.join(path.strip_prefix(input_path).expect("Failed to strip path prefix"));

			let output_folder = output_path.parent();
			if let Some(output_folder) = output_folder {
				info!("Making output folder {}", output_folder.display());
				if let Err(e) = fs::create_dir_all(output_folder) {
					warn!("Error making output directory {}:\n{}", output_folder.display(), e);
				}
			}

			if let Err(e) = palette_swap_path(&palette, &path, &output_path) {
				warn!("Error processing file {}:\n{}", input_path.display(), e);
			} else {
				info!("Outputted file {}", output_path.display());
			}
		}
	} else {
		if let Err(e) = palette_swap_path(&palette, &input_path, &output_path) {
			warn!("Error processing file {}:\n{}", input_path.display(), e);
		} else {
			info!("Outputted file {}", output_path.display());
		}
	}
}
