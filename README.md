# ase-cli-tools
A CLI tool written in Rust for batch modification of Aseprite files.

(Yeah, I don't know either. I'm bad at naming things.)

Make sure to use `--recursive` when cloning so that the submodule is initialized.
e.g. `git clone --recursive https://github.com/CursedFlames/ase-cli-tools`

Currently only supports palette swapping. Usage:

```cargo run --release <palette file> <input file/dir> <output name>```

where `palette file` is a 2 pixel high Aseprite file with the top row containing source colors,
and the bottom row containing destination colors. All cels will be used, regardless of layer or frame.
(This is oddly specific, but I'm not sure how to approach making it more generalized.)

This will modify both pixels in the actual images, and colors in the files' palettes. (TODO: make this configurable)

Caveats:
* Only works on RGBA images, not greyscale or indexed
* Is alpha sensitive (so pixels with the same color but different alpha won't be updated).
I may add an option for alpha insensitivity later.
* If colors in file palettes have names, these will be retained even if the color is changed.

This program never overwrites/modifies existing files,
but you should keep backups of your files anyway just in case something breaks horribly.