**!ABANDONED! This project won't be continued, see [fluxcore_ng](https://github.com/crepererum/fluxcore_ng) for a new try. !ABANDONED!**

# fluxcore
This is a high performance CSV renderer. It supports float and integer values and provides a scatter plot including a color dimension and configurable dot sizes, alpha scaling and fast dimension switching.

## Requirements
To run fluxcore, you'll need the following:

 - Linux 64bit (others systems might work, but I've never tested this)
 - OpenGL 3.1 drivers

## Build
In addition to the runtime requirements, you also need [Rust](http://www.rust-lang.org/) to compile fluxcore. Because the language isn't ready yet, nightlies are required. Furthermore [Cargo](http://crates.io/), the Rust build tool, needs to be installed. I recommend to use rustup to install both:

    curl -s https://static.rust-lang.org/rustup.sh | sudo sh

After this short installation process is finished, use Cargo to fetch all dependencies and build everything:

    cargo build

Add the `--release` option for faster binaries, but be warned: The Rust compiler sometimes produces buggy executables during the optimization process. I won't accept bug reports for these builds before Rust 1.0!

## FAQ
There are some very obvious questions and some great answers.

### Why (buggy, new) Rust?
Because I want to learn a new language, which tries to solve many outstanding problems of high performance programming. I really love C++, which produces fine-tuned results and enables you to control resources and has a pretty nice syntax. But I also got tired off all these undefined and legacy stuff. And Rusts move-by-default and everything-is-checked style is gorgeous :smile:

### Why a hand-written OpenGL renderer instead of TeX, LibreOffice, gnuplot, ...?
Because it's fast and interactive. Did you ever tried to plot 100k data elements using these tools?

### Is float/f32 precise enough?
When you scan through the source code you'll recognize that I use 32-bit floating points as internal format. I never reached a case where this resulted into problems. But the true reason for this is that 64bit data isn't supported by core OpenGl < 4.1. And the extension isn't implemented by Mesa yet. Feel free to recheck the status [here](http://cgit.freedesktop.org/mesa/mesa/tree/docs/GL3.txt) under `GL 4.0 - GL_ARB_gpu_shader_fp64` and `GL 4.1 - GL_ARB_vertex_attrib_64bit`. I could convert all data arrays before uploading it to the GPU, but this makes the system inflexible for future expansions.

### Where is the documentation?
Start the executable using `--help` to get the command line help and press H during the rendering to get the key mapping. There is no source code documentation now because I don't have enough resources, sorry.

### What next?
I have some pretty nice ideas for this, but that depends on my spare time :wink:

