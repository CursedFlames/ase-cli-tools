use std::{fs, io};
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
			if let ChunkData::CelChunk(data) = data {
				let cel = &data.cel;
				if let RawCel {..} | CompressedImage {..} = cel {
					let width = cel.w().unwrap() as usize;
					let height = cel.h().unwrap() as usize;
					if width < 1 || height < 2 {
						continue;
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
				}
			}
		}
	}
	// TODO maybe figure out how to format this more nicely?
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

	let arg_matches = clap::App::new("Ase CLI Utils")
		.version("0.1.1")
		.author("CursedFlames")
		.subcommand(clap::SubCommand::with_name("paletteswap")
			.arg(clap::Arg::with_name("palette")
				.help("Path of the palette file. The palette file should be a 2 pixel high aseprite file, with the top row as source colors and the bottom row as destination colors.")
				.required(true)
				.index(1))
			.arg(clap::Arg::with_name("input")
				.help("Path of the input file or directory")
				.required(true)
				.index(2))
			.arg(clap::Arg::with_name("output")
				.help("Path to create the output file or directory")
				.required(true)
				.index(3)))
		.get_matches();

	match arg_matches.subcommand() {
		("paletteswap", Some(args)) => {
			let palette_path = Path::new(args.value_of("palette").unwrap());
			let input_path = Path::new(args.value_of("input").unwrap());
			let output_path = Path::new(args.value_of("output").unwrap());
			cmd_palette_swap(palette_path, input_path, output_path);
		}
		_ => {}
	}
}

fn cmd_palette_swap(palette_path: &Path, input_path: &Path, output_path: &Path) {
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
		// Note that the iter is collect()ed before the loop,
		// so that output files aren't read as inputs if output_path is inside input_path
		for entry in WalkDir::new(input_path).into_iter()
				.filter_map(|e| e.ok()).filter(|e| is_ase_extension(e.path().extension()))
				.collect::<Vec<_>>() {
			let path = entry.path();
			// TODO shouldn't use this expect here
			let output_path = output_path.join(path.strip_prefix(input_path).expect("Failed to strip path prefix"));

			let output_folder = output_path.parent();
			if let Some(output_folder) = output_folder {
				if !output_folder.exists() {
					info!("Making output folder {}", output_folder.display());
					if let Err(e) = fs::create_dir_all(output_folder) {
						warn!("Error making output directory {}:\n{}", output_folder.display(), e);
					}
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
