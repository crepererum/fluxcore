use freetype;
use graphics;
use graphics::{AddImage, Draw, RelativeTransform2d};
use opengl_graphics;
use std::collections;

pub enum AnchorHor {
    Left,
    Center,
    Right,
}

pub enum AnchorVert {
    Top,
    Middle,
    Bottom,
}

struct Character {
    glyph: freetype::Glyph,
    bitmap_glyph: freetype::BitmapGlyph,
    texture: opengl_graphics::Texture,
}

pub struct TextDrawer {
    freetype: freetype::Library,
    fontface: freetype::Face,
    size: u32,
    characterBuffer: collections::hashmap::HashMap<char, Character>,
}

impl TextDrawer {
    pub fn new(fontfile: String, size: u32) -> TextDrawer {
        let freetype = freetype::Library::init().unwrap();
        let fontface = freetype.new_face(fontfile.as_slice(), 0).unwrap();
        fontface.set_pixel_sizes(0, size).unwrap();

        TextDrawer {
            freetype: freetype,
            fontface: fontface,
            size: size,
            characterBuffer: collections::hashmap::HashMap::new(),
        }
    }

    fn load_character(&mut self, ch: char) {
        self.fontface.load_char(ch as u64, freetype::face::Default).unwrap();
        let glyph = self.fontface.glyph().get_glyph().unwrap();
        let bitmap_glyph = glyph.to_bitmap(freetype::render_mode::Normal, None).unwrap();
        let bitmap = bitmap_glyph.bitmap();
        let texture = opengl_graphics::Texture::from_memory_alpha(bitmap.buffer(), bitmap.width() as u32, bitmap.rows() as u32).unwrap();

        self.characterBuffer.insert(ch, Character {
            glyph: glyph,
            bitmap_glyph: bitmap_glyph,
            texture: texture,
        });
    }

    fn render_raw(&mut self, c: &graphics::Context<(),[f32, ..4]>, gl2d: &mut opengl_graphics::Gl, text: &String, draw: bool) -> (i32, i32) {
        let mut x = 0;
        let mut y = 0;

        for ch in text.as_slice().chars() {
            if !self.characterBuffer.contains_key(&ch) {
                self.load_character(ch);
            }

            let character = self.characterBuffer.get(&ch);

            if draw {
                c.trans((x + character.bitmap_glyph.left()) as f64, (y - character.bitmap_glyph.top()) as f64)
                    .image(&character.texture)
                    .draw(gl2d);
            }

            // A 16.16 vector that gives the glyph's advance width.
            x += (character.glyph.advance().x >> 16) as i32;
            y += (character.glyph.advance().y >> 16) as i32;
        }

        (x, y)
    }

    pub fn render(&mut self, c: &graphics::Context<(),[f32, ..4]>, gl2d: &mut opengl_graphics::Gl, text: &String, hor: AnchorHor, vert: AnchorVert) {
        let (width, _height) = self.render_raw(c, gl2d, text, false);
        let dx = match hor {
            Left => 0f64,
            Center => (-width as f64 / 2f64).floor(),
            Right => (-width as f64).floor()
        };
        let dy = match vert {
            Top => 0f64,
            Middle => (self.size as f64 / 2f64).floor(),
            Bottom => self.size as f64
        };
        self.render_raw(&c.trans(dx, dy), gl2d, text, true);
    }
}
