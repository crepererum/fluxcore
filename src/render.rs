use cgmath;
use cgmath::FixedArray;
use cgmath::Matrix;
use data;
use gl;
use glfw;
use glfw::Context;
use graphics;
use graphics::{AddLine, AddRoundBorder, AddColor, Draw, RelativeTransform2d};
use hgl;
use opengl_graphics;
use std::comm;
use std::f32;
use std::io;
use std::mem;
use std::ptr;
use std::time;
use textdrawer;

static FONT_DATA: &'static [u8] = include_bin!("../res/DejaVuSansCondensed-Bold.ttf");
static LIB_SHADER_GRADIENT: &'static str = include_str!("../res/gradient.lib.glsl");
static VERTEX_SHADER_POINTS: &'static str = include_str!("../res/points.vertex.glsl");
static FRAGMENT_SHADER_POINTS: &'static str = include_str!("../res/points.fragment.glsl");
static VERTEX_SHADER_TEXTURE: &'static str = include_str!("../res/texture.vertex.glsl");
static FRAGMENT_SHADER_TEXTURE: &'static str = include_str!("../res/texture.fragment.glsl");
static VERTEX_SHADER_LEGEND: &'static str = include_str!("../res/legend.vertex.glsl");
static FRAGMENT_SHADER_LEGEND: &'static str = include_str!("../res/legend.fragment.glsl");
static HELP_TEXT: &'static str = include_str!("../res/help.txt");

static VERTEX_DATA_TEXTURE: [gl::types::GLfloat, ..12] = [
    -1.0, -1.0,
    1.0, 1.0,
    1.0, -1.0,
    -1.0, -1.0,
    1.0, -1.0,
    1.0, 1.0,
];

static MARGIN: f32 = 130f32;
static TICK_DISTANCE: i32 = 60i32;
static FONT_SIZE: u32 = 16u32;
static LABEL_MARGIN: f64 = 50f64;
static INFO_MARGIN: f64 = 2f64;
static TICK_LENGTH: f64 = 6f64;
static TICK_WIDTH: f64 = 0.5f64;
static PAUSE_MS: i32 = 20;

fn range_vec(vec: &Vec<f32>) -> (f32, f32) {
    let min = vec.tail().iter().fold(vec[0] + 0.0, |a, &b| a.min(b));
    let max = vec.tail().iter().fold(vec[0] + 0.0, |a, &b| a.max(b));
    (min, max)
}

fn nice_num(x: f32, round: bool) -> f32 {
    let exp = x.log10().floor() as i32;
    let f = x / 10f32.powi(exp);

    let nf = if round {
        if f < 1.5f32 {
            1f32
        } else if f < 3f32 {
            2f32
        } else if f < 7f32 {
            5f32
        } else {
            10f32
        }
    } else {
        if f < 1f32 {
            1f32
        } else if f < 2f32 {
            2f32
        } else if f < 5f32 {
            5f32
        } else {
            10f32
        }
    };

    nf * 10f32.powi(exp)
}

fn std_scale(renderLength: i32) -> f32 {
    1f32 - 2f32 * MARGIN / renderLength as f32
}

enum ActiveTransform {
    TransformMove,
    TransformScale,
    TransformNone,
}

struct Dimension {
    renderLength: i32,
    d: f32,
    s: f32,
    vbo: hgl::buffer::Vbo,
    min: f32,
    max: f32,
    name: String,
}

impl Dimension {
    fn new(renderLength: i32, table: &data::Table, name: &String) -> Dimension {
        let data = table.get(name).unwrap();
        let (min, max) = range_vec(data);
        let vbo = hgl::Vbo::from_data(data.as_slice(), hgl::StaticDraw);
        Dimension{
            renderLength: renderLength,
            d: 0f32,
            s: std_scale(renderLength),
            vbo: vbo,
            min: min,
            max: max,
            name: name.clone()
        }
    }

    fn reset(&mut self) {
        self.d = 0f32;
        self.s = std_scale(self.renderLength);
    }

    fn calc_axis_markers(&self, pixelsPerTick: i32) -> (u32, f32, f32, Vec<f32>) {
        // precalc projection
        let minVar = (self.min - self.d / std_scale(self.renderLength) - (self.max + self.min) / 2f32) / self.s * std_scale(self.renderLength) + (self.max + self.min) / 2f32;
        let maxVar = (self.max - self.d / std_scale(self.renderLength) - (self.max + self.min) / 2f32) / self.s * std_scale(self.renderLength) + (self.max + self.min) / 2f32;

        // calc ticks, borders, steps
        let ntick = ((self.renderLength as f32 - 2f32 * MARGIN) / pixelsPerTick as f32) as i32;
        let range = nice_num(maxVar - minVar, false);
        let d = nice_num(range / (ntick - 1) as f32, true);
        let graphMin = (minVar / d).floor() * d;
        let graphMax = (maxVar / d).ceil() * d;
        let nfrac = [0u32, -d.log10().floor() as u32].iter().max().unwrap().clone();

        // generate markers
        let mut markers: Vec<f32> = Vec::new();
        cfor!{let mut m = graphMin; m < graphMax + 0.5f32 * d; (m += d) {
            let marker = if m < minVar {
                minVar
            } else if m > maxVar {
                maxVar
            } else {
                m
            };
            markers.push(marker);
        }}

        (nfrac, minVar, maxVar, markers)
    }
}

fn calc_projection(dimx: &Dimension, dimy: &Dimension, dimz: &Dimension) -> cgmath::Matrix4<f32> {
    let (xmin, xmax) = if dimx.min.is_nan() || dimx.max.is_nan() || dimx.min == dimx.max {
        (-1f32, 1f32)
    } else {
        (dimx.min, dimx.max)
    };
    let (ymin, ymax) = if dimy.min.is_nan() || dimy.max.is_nan() || dimy.min == dimy.max {
        (-1f32, 1f32)
    } else {
        (dimy.min, dimy.max)
    };
    let (zmin, zmax) = if dimz.min.is_nan() || dimz.max.is_nan() || dimz.min == dimz.max {
        (-1f32, 1f32)
    } else {
        (dimz.min, dimz.max)
    };

    let mut result = cgmath::ortho(
        xmin, xmax,
        ymin, ymax,
        zmin, zmax
    );

    // fix z projection
    result.as_mut_fixed()[2][3] = 0f32;
    result.as_mut_fixed()[3][2] *= -1f32;

    result
}

struct UniformLocationPoints {
    width: gl::types::GLint,
    height: gl::types::GLint,
    pointScale: gl::types::GLint,
    transformation: gl::types::GLint,
    margin: gl::types::GLint,
}

struct UniformLocationTexture {
    count: gl::types::GLint,
    alpha: gl::types::GLint,
    fboTexture: gl::types::GLint,
}

struct UniformLocationLegend {
    width: gl::types::GLint,
    height: gl::types::GLint,
    margin: gl::types::GLint,
}

struct Renderer {
    table: data::Table,
    glfw: glfw::Glfw,
    window: glfw::Window,
    events: comm::Receiver<(f64, glfw::WindowEvent)>,
    dimx: Dimension,
    dimy: Dimension,
    dimz: Dimension,
    dimzDelta: f32,
    dimzScale: f32,
    activeTransform: ActiveTransform,
    mouseX: f32,
    mouseY: f32,
    pointScale: f32,
    alphaScale: f32,
    projection: cgmath::Matrix4<f32>,
    ulocationPoints: UniformLocationPoints,
    ulocationTexture: UniformLocationTexture,
    ulocationLegend: UniformLocationLegend,
    vaoPoints: hgl::vao::Vao,
    vaoTexture: hgl::vao::Vao,
    vboTexture: hgl::buffer::Vbo,
    programPoints: hgl::program::Program,
    programTexture: hgl::program::Program,
    programLegend: hgl::program::Program,
    textdrawer: textdrawer::TextDrawer,
    gl2d: opengl_graphics::Gl,
    showHelp: bool,
    changed: bool,
    framebuffer: gl::types::GLuint,
    texture: gl::types::GLuint,
}

impl Renderer {
    fn new(table: data::Table, column_x: &String, column_y: &String, column_z: &String) -> Renderer {
        let width = 800i32;
        let height = 600i32;

        let glfw = glfw::init(glfw::FAIL_ON_ERRORS).unwrap();
        glfw.window_hint(glfw::ContextVersion(3, 1));

        let (window, events) = glfw.create_window(width as u32, height as u32, format!("fluxcore - {}", table.name()).as_slice(), glfw::Windowed).unwrap();
        window.set_all_polling(true);
        window.make_current();
        gl::load_with(|p| glfw.get_proc_address(p));

        gl::Viewport(0, 0, width, height);

        let vaoPoints = hgl::Vao::new();
        vaoPoints.bind();

        let programPoints = hgl::Program::link([
            hgl::Shader::compile(VERTEX_SHADER_POINTS, hgl::VertexShader),
            hgl::Shader::compile(LIB_SHADER_GRADIENT, hgl::VertexShader),
            hgl::Shader::compile(FRAGMENT_SHADER_POINTS, hgl::FragmentShader)
        ]).unwrap();
        let ulocationPoints = UniformLocationPoints{
            width: programPoints.uniform("width"),
            height: programPoints.uniform("height"),
            pointScale: programPoints.uniform("pointScale"),
            transformation: programPoints.uniform("transformation"),
            margin: programPoints.uniform("margin"),
        };
        programPoints.bind_frag(0, "out_color");
        programPoints.bind();

        let dimx = Dimension::new(width, &table, column_x);
        vaoPoints.enable_attrib(&programPoints, "position_x", gl::FLOAT, 1, (1 * mem::size_of::<f32>()) as i32, 0);
        dimx.vbo.bind();

        let dimy = Dimension::new(height, &table, column_y);
        vaoPoints.enable_attrib(&programPoints, "position_y", gl::FLOAT, 1, (1 * mem::size_of::<f32>()) as i32, 0);
        dimy.vbo.bind();

        let mut dimz = Dimension::new(width, &table, column_z);
        vaoPoints.enable_attrib(&programPoints, "position_z", gl::FLOAT, 1, (1 * mem::size_of::<f32>()) as i32, 0);
        dimz.vbo.bind();

        let vaoTexture = hgl::Vao::new();
        vaoTexture.bind();

        let programTexture = hgl::Program::link([
            hgl::Shader::compile(VERTEX_SHADER_TEXTURE, hgl::VertexShader),
            hgl::Shader::compile(FRAGMENT_SHADER_TEXTURE, hgl::FragmentShader)
        ]).unwrap();
        let ulocationTexture = UniformLocationTexture{
            count: programTexture.uniform("count"),
            alpha: programTexture.uniform("alpha"),
            fboTexture: programTexture.uniform("fbo_texture"),
        };
        programTexture.bind_frag(0, "out_color");
        programTexture.bind();

        let vboTexture = hgl::Vbo::from_data(VERTEX_DATA_TEXTURE.as_slice(), hgl::StaticDraw);
        vaoTexture.enable_attrib(&programTexture, "v_coord", gl::FLOAT, 2, (1 * mem::size_of::<f32>()) as i32, 0);
        vboTexture.bind();

        let programLegend = hgl::Program::link([
            hgl::Shader::compile(VERTEX_SHADER_LEGEND, hgl::VertexShader),
            hgl::Shader::compile(FRAGMENT_SHADER_LEGEND, hgl::FragmentShader),
            hgl::Shader::compile(LIB_SHADER_GRADIENT, hgl::FragmentShader)
        ]).unwrap();
        let ulocationLegend = UniformLocationLegend{
            width: programLegend.uniform("width"),
            height: programLegend.uniform("height"),
            margin: programLegend.uniform("margin"),
        };
        programLegend.bind_frag(0, "out_color");
        programLegend.bind();

        vaoTexture.enable_attrib(&programLegend, "v_coord", gl::FLOAT, 2, (1 * mem::size_of::<f32>()) as i32, 0);
        vboTexture.bind();

        let projection = calc_projection(&dimx, &dimy, &dimz);

        let mut framebuffer = 0;
        unsafe {
            gl::GenFramebuffers(1, &mut framebuffer);
        }
        gl::BindFramebuffer(gl::FRAMEBUFFER, framebuffer);

        let mut texture = 0;
        unsafe {
            gl::GenTextures(1, &mut texture);
        }
        gl::BindTexture(gl::TEXTURE_2D, texture);
        unsafe {
            gl::TexImage2D(gl::TEXTURE_2D, 0, gl::RGBA32F as i32, dimx.renderLength, dimy.renderLength, 0, gl::RGBA, gl::FLOAT, ptr::null());
        }
        gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MAG_FILTER, gl::NEAREST as i32);
        gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MIN_FILTER, gl::NEAREST as i32);
        gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_WRAP_S, gl::CLAMP_TO_EDGE as i32);
        gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_WRAP_T, gl::CLAMP_TO_EDGE as i32);

        gl::FramebufferTexture(gl::FRAMEBUFFER, gl::COLOR_ATTACHMENT0, texture, 0);
        let drawBuffers = [gl::COLOR_ATTACHMENT0];
        unsafe {
            gl::DrawBuffers(drawBuffers.len() as i32, mem::transmute(&drawBuffers[0]));
        }
        gl::BindFramebuffer(gl::FRAMEBUFFER, 0);

        Renderer {
            glfw: glfw,
            window: window,
            events: events,
            dimx: dimx,
            dimy: dimy,
            dimz: dimz,
            dimzDelta: 0f32,
            dimzScale: 1f32,
            activeTransform: TransformNone,
            mouseX: 0f32,
            mouseY: 0f32,
            pointScale: 4f32,
            alphaScale: 1f32,
            projection: projection,
            ulocationPoints: ulocationPoints,
            ulocationTexture: ulocationTexture,
            ulocationLegend: ulocationLegend,
            vaoPoints: vaoPoints,
            vaoTexture: vaoTexture,
            vboTexture: vboTexture,
            programPoints: programPoints,
            programTexture: programTexture,
            programLegend: programLegend,
            table: table,
            textdrawer: textdrawer::TextDrawer::new(FONT_DATA, FONT_SIZE),
            gl2d: opengl_graphics::Gl::new(),
            showHelp: false,
            changed: true,
            framebuffer: framebuffer,
            texture: texture,
        }
    }

    fn draw_x_axis(&mut self, c: &graphics::Context<(),[f32, ..4]>) {
        let line = c.line(MARGIN as f64, MARGIN as f64, self.dimx.renderLength as f64 - MARGIN as f64, MARGIN as f64)
            .round_border_radius(1.0);


        line.draw(&mut self.gl2d);
        line.trans(0f64, self.dimy.renderLength as f64 - 2f64 * MARGIN as f64)
            .draw(&mut self.gl2d);

        let text_c1 = c.trans((self.dimx.renderLength as f64 / 2f64).floor(), LABEL_MARGIN);
        let text_c2 = c.trans((self.dimx.renderLength as f64 / 2f64).floor(), self.dimy.renderLength as f64 - LABEL_MARGIN);
        let text = self.dimx.name.clone();
        self.textdrawer.render(&text_c1, &mut self.gl2d, &text, textdrawer::Center, textdrawer::Top);
        self.textdrawer.render(&text_c2, &mut self.gl2d, &text, textdrawer::Center, textdrawer::Bottom);

        let (nfrac, mmin, mmax, marksers) = self.dimx.calc_axis_markers(TICK_DISTANCE);
        for m in marksers.iter() {
            let pos = (MARGIN + (m - mmin) / (mmax - mmin) * (self.dimx.renderLength as f32 - 2f32 * MARGIN)).floor();
            let marker_text = f32::to_str_digits(m.clone(), nfrac as uint + 1);
            let marker_c1 = c.trans(pos as f64, MARGIN as f64 - 10f64)
                .rot_deg(270f64);
            let marker_c2 = c.trans(pos as f64, self.dimy.renderLength as f64 - MARGIN as f64 + 10f64)
                .rot_deg(90f64);

            self.textdrawer.render(&marker_c1, &mut self.gl2d, &marker_text, textdrawer::Left, textdrawer::Middle);
            self.textdrawer.render(&marker_c2, &mut self.gl2d, &marker_text, textdrawer::Left, textdrawer::Middle);

            c.line(pos as f64, MARGIN as f64 - TICK_LENGTH, pos as f64, MARGIN as f64)
                .round_border_radius(TICK_WIDTH)
                .draw(&mut self.gl2d);
            c.line(pos as f64, self.dimy.renderLength as f64 - MARGIN as f64 + TICK_LENGTH, pos as f64, self.dimy.renderLength as f64 - MARGIN as f64)
                .round_border_radius(TICK_WIDTH)
                .draw(&mut self.gl2d);
        }
    }

    fn draw_y_axis(&mut self, c: &graphics::Context<(),[f32, ..4]>) {
        let line = c.line(MARGIN as f64, MARGIN as f64, MARGIN as f64, self.dimy.renderLength as f64 - MARGIN as f64)
            .round_border_radius(1.0);

        line.draw(&mut self.gl2d);
        line.trans(self.dimx.renderLength as f64 - 2f64 * MARGIN as f64, 0f64)
            .draw(&mut self.gl2d);

        let text_c1 = c.trans(LABEL_MARGIN, (self.dimy.renderLength as f64 / 2f64).floor())
            .rot_deg(270f64);
        let text_c2 = c.trans(self.dimx.renderLength as f64 - LABEL_MARGIN, (self.dimy.renderLength as f64 / 2f64).floor())
            .rot_deg(90f64);
        let text = self.dimy.name.clone();
        self.textdrawer.render(&text_c1, &mut self.gl2d, &text, textdrawer::Center, textdrawer::Top);
        self.textdrawer.render(&text_c2, &mut self.gl2d, &text, textdrawer::Center, textdrawer::Top);

        let (nfrac, mmin, mmax, marksers) = self.dimy.calc_axis_markers(TICK_DISTANCE);
        for m in marksers.iter() {
            let pos = (MARGIN + (1.0 - (m - mmin) / (mmax - mmin)) * (self.dimy.renderLength as f32 - 2f32 * MARGIN)).floor();
            let marker_text = f32::to_str_digits(m.clone(), nfrac as uint + 1);
            let marker_c1 = c.trans(MARGIN as f64 - 10f64, pos as f64);
            let marker_c2 = c.trans(self.dimx.renderLength as f64 - MARGIN as f64 + 10f64, pos as f64);

            self.textdrawer.render(&marker_c1, &mut self.gl2d, &marker_text, textdrawer::Right, textdrawer::Middle);
            self.textdrawer.render(&marker_c2, &mut self.gl2d, &marker_text, textdrawer::Left, textdrawer::Middle);

            c.line(MARGIN as f64 - TICK_LENGTH, pos as f64, MARGIN as f64, pos as f64)
                .round_border_radius(TICK_WIDTH)
                .draw(&mut self.gl2d);
            c.line(self.dimx.renderLength as f64 - MARGIN as f64 + TICK_LENGTH, pos as f64, self.dimx.renderLength as f64 - MARGIN as f64, pos as f64)
                .round_border_radius(TICK_WIDTH)
                .draw(&mut self.gl2d);
        }
    }

    fn draw_z_axis(&mut self, c: &graphics::Context<(),[f32, ..4]>) {
        let line = c.line(MARGIN as f64, self.dimy.renderLength as f64 - MARGIN as f64 / 5f64, self.dimz.renderLength as f64 - MARGIN as f64, self.dimy.renderLength as f64 - MARGIN as f64 / 5f64)
            .round_border_radius(1.0);


        line.draw(&mut self.gl2d);

        self.textdrawer.render(&c.trans(INFO_MARGIN, self.dimy.renderLength as f64 - INFO_MARGIN), &mut self.gl2d, &format!("z: {}", self.dimz.name), textdrawer::Left, textdrawer::Bottom);

        let (nfrac, mmin, mmax, marksers) = self.dimz.calc_axis_markers(TICK_DISTANCE);
        for m in marksers.iter() {
            let pos = (MARGIN + (m - mmin) / (mmax - mmin) * (self.dimz.renderLength as f32 - 2f32 * MARGIN)).floor();
            let marker_text = f32::to_str_digits(m.clone(), nfrac as uint + 1);
            let marker_c = c.trans(pos as f64, self.dimy.renderLength as f64 - MARGIN as f64 / 5f64 - 10f64);

            self.textdrawer.render(&marker_c, &mut self.gl2d, &marker_text, textdrawer::Center, textdrawer::Bottom);

            c.line(pos as f64, self.dimy.renderLength as f64 - MARGIN as f64 / 5f64 - TICK_LENGTH, pos as f64, self.dimy.renderLength as f64 - MARGIN as f64 / 5f64)
                .round_border_radius(TICK_WIDTH)
                .draw(&mut self.gl2d);
        }
    }

    fn handle_event(&mut self, event: glfw::WindowEvent) {
        match event {
            glfw::SizeEvent(w, h) => {
                self.dimx.renderLength = w;
                self.dimy.renderLength = h;
                self.dimz.renderLength = w;
                gl::Viewport(0, 0, self.dimx.renderLength, self.dimy.renderLength);

                gl::BindTexture(gl::TEXTURE_2D, self.texture);
                unsafe {
                    gl::TexImage2D(gl::TEXTURE_2D, 0, gl::RGBA32F as i32, self.dimx.renderLength, self.dimy.renderLength, 0, gl::RGBA, gl::FLOAT, ptr::null());
                }
                gl::BindTexture(gl::TEXTURE_2D, 0);
            },
            glfw::CursorPosEvent(xpos, ypos) => {
                match self.activeTransform {
                    TransformMove => {
                        let xdiff = (xpos as f32 - self.mouseX) / self.dimx.renderLength as f32;
                        let ydiff = (self.mouseY - ypos as f32) / self.dimy.renderLength as f32;

                        self.dimx.d += xdiff * (self.dimx.max - self.dimx.min);
                        self.dimy.d += ydiff * (self.dimy.max - self.dimy.min);
                    },
                    TransformScale => {
                        let x1 = self.mouseX - self.dimx.renderLength as f32 / 2.0f32;
                        let x2 = xpos as f32 - self.dimx.renderLength as f32 / 2.0f32;
                        let y1 = self.mouseY - self.dimy.renderLength as f32 / 2.0f32;
                        let y2 = ypos as f32 - self.dimy.renderLength as f32 / 2.0f32;


                        self.dimx.d = self.dimx.d / self.dimx.s;
                        self.dimy.d = self.dimy.d / self.dimy.s;
                        self.dimx.s *= x2 / x1;
                        self.dimy.s *= y2 / y1;
                        self.dimx.d = self.dimx.d * self.dimx.s;
                        self.dimy.d = self.dimy.d * self.dimy.s;
                    },
                    TransformNone => ()
                }
                self.mouseX = xpos as f32;
                self.mouseY = ypos as f32;
            },
            glfw::MouseButtonEvent(button, action, _mods) => {
                match (button, action, self.activeTransform) {
                    (glfw::MouseButton1, glfw::Press, TransformNone) => {
                        self.activeTransform = TransformMove;
                    },
                    (glfw::MouseButton1, glfw::Release, TransformMove) => {
                        self.activeTransform = TransformNone;
                    },
                    (glfw::MouseButton2, glfw::Press, TransformNone) => {
                        self.activeTransform = TransformScale;
                    },
                    (glfw::MouseButton2, glfw::Release, TransformScale) => {
                        self.activeTransform = TransformNone;
                    },
                    _ => ()
                }
            },
            glfw::ScrollEvent(dx, dy) => {
                if dx > 0.0 {
                    self.dimz.d -= 0.05 * (self.dimz.max - self.dimz.min) * self.dimz.s;
                    self.dimzDelta += 0.05 * (self.dimz.max - self.dimz.min) * self.dimzScale;
                } else if dx < 0.0 {
                    self.dimz.d += 0.05 * (self.dimz.max - self.dimz.min) * self.dimz.s;
                    self.dimzDelta -= 0.05 * (self.dimz.max - self.dimz.min) * self.dimzScale;
                }
                if dy > 0.0 {
                    self.dimz.d = self.dimz.d / self.dimz.s;
                    self.dimzDelta = self.dimzDelta / self.dimzScale;

                    self.dimz.s *= 1.05;
                    self.dimzScale *= 1.05;

                    self.dimz.d = self.dimz.d * self.dimz.s;
                    self.dimzDelta = self.dimzDelta * self.dimzScale;
                } else if dy < 0.0 {
                    self.dimz.d = self.dimz.d / self.dimz.s;
                    self.dimzDelta = self.dimzDelta / self.dimzScale;

                    self.dimz.s /= 1.05;
                    self.dimzScale /= 1.05;

                    self.dimz.d = self.dimz.d * self.dimz.s;
                    self.dimzDelta = self.dimzDelta * self.dimzScale;
                }
            }
            glfw::KeyEvent(key, _scancode, action, _mods) => {
                match (key, action) {
                    (glfw::KeyEscape, glfw::Press) => self.window.set_should_close(true),
                    (glfw::KeyW, glfw::Press) => self.pointScale *= 1.5f32,
                    (glfw::KeyQ, glfw::Press) => self.pointScale = 1f32.max(self.pointScale / 1.5f32),
                    (glfw::KeyA, glfw::Press) => self.alphaScale = 0f32.max(self.alphaScale - 0.02f32),
                    (glfw::KeyS, glfw::Press) => self.alphaScale = 1f32.min(self.alphaScale + 0.02f32),
                    (glfw::KeyH, glfw::Press) => self.showHelp = !self.showHelp,
                    (glfw::KeyR, glfw::Press) => {
                        self.pointScale = 4f32;
                        self.alphaScale = 1f32;
                        self.dimx.reset();
                        self.dimy.reset();
                        self.dimz.reset();
                        self.dimzDelta = 0f32;
                        self.dimzScale = 1f32;
                    },
                    (glfw::KeyRight, glfw::Press) => {
                        {
                            let next = match self.table.columns().iter().skip_while(|&s| s != &self.dimx.name).skip(1).next() {
                                Some(element) => element,
                                None => self.table.columns().iter().next().unwrap()
                            };
                            let dim = Dimension::new(self.dimx.renderLength, &self.table, next);
                            self.vaoPoints.enable_attrib(&self.programPoints, "position_x", gl::FLOAT, 1, (1 * mem::size_of::<f32>()) as i32, 0);
                            dim.vbo.bind();
                            self.dimx = dim;
                        }
                        self.projection = calc_projection(&self.dimx, &self.dimy, &self.dimz);
                    },
                    (glfw::KeyLeft, glfw::Press) => {
                        {
                            let next = match self.table.columns().rev_iter().skip_while(|&s| s != &self.dimx.name).skip(1).next() {
                                Some(element) => element,
                                None => self.table.columns().rev_iter().next().unwrap()
                            };
                            let dim = Dimension::new(self.dimx.renderLength, &self.table, next);
                            self.vaoPoints.enable_attrib(&self.programPoints, "position_x", gl::FLOAT, 1, (1 * mem::size_of::<f32>()) as i32, 0);
                            dim.vbo.bind();
                            self.dimx = dim;
                        }
                        self.projection = calc_projection(&self.dimx, &self.dimy, &self.dimz);
                    },
                    (glfw::KeyDown, glfw::Press) => {
                        {
                            let next = match self.table.columns().iter().skip_while(|&s| s != &self.dimy.name).skip(1).next() {
                                Some(element) => element,
                                None => self.table.columns().iter().next().unwrap()
                            };
                            let dim = Dimension::new(self.dimy.renderLength, &self.table, next);
                            self.vaoPoints.enable_attrib(&self.programPoints, "position_y", gl::FLOAT, 1, (1 * mem::size_of::<f32>()) as i32, 0);
                            dim.vbo.bind();
                            self.dimy = dim;
                        }
                        self.projection = calc_projection(&self.dimx, &self.dimy, &self.dimz);
                    },
                    (glfw::KeyUp, glfw::Press) => {
                        {
                            let next = match self.table.columns().rev_iter().skip_while(|&s| s != &self.dimy.name).skip(1).next() {
                                Some(element) => element,
                                None => self.table.columns().rev_iter().next().unwrap()
                            };
                            let dim = Dimension::new(self.dimy.renderLength, &self.table, next);
                            self.vaoPoints.enable_attrib(&self.programPoints, "position_y", gl::FLOAT, 1, (1 * mem::size_of::<f32>()) as i32, 0);
                            dim.vbo.bind();
                            self.dimy = dim;
                        }
                        self.projection = calc_projection(&self.dimx, &self.dimy, &self.dimz);
                    },
                    (glfw::KeyPageDown, glfw::Press) => {
                        {
                            let next = match self.table.columns().iter().skip_while(|&s| s != &self.dimz.name).skip(1).next() {
                                Some(element) => element,
                                None => self.table.columns().iter().next().unwrap()
                            };
                            let dim = Dimension::new(self.dimz.renderLength, &self.table, next);
                            self.vaoPoints.enable_attrib(&self.programPoints, "position_z", gl::FLOAT, 1, (1 * mem::size_of::<f32>()) as i32, 0);
                            dim.vbo.bind();
                            self.dimz = dim;
                            self.dimzDelta = 0f32;
                            self.dimzScale = 1f32;
                        }
                        self.projection = calc_projection(&self.dimx, &self.dimy, &self.dimz);
                    },
                    (glfw::KeyPageUp, glfw::Press) => {
                        {
                            let next = match self.table.columns().rev_iter().skip_while(|&s| s != &self.dimz.name).skip(1).next() {
                                Some(element) => element,
                                None => self.table.columns().rev_iter().next().unwrap()
                            };
                            let dim = Dimension::new(self.dimz.renderLength, &self.table, next);
                            self.vaoPoints.enable_attrib(&self.programPoints, "position_z", gl::FLOAT, 1, (1 * mem::size_of::<f32>()) as i32, 0);
                            dim.vbo.bind();
                            self.dimz = dim;
                            self.dimzDelta = 0f32;
                            self.dimzScale = 1f32;
                        }
                        self.projection = calc_projection(&self.dimx, &self.dimy, &self.dimz);
                    },
                    _ => ()
                }
            }
            _ => ()
        }
    }

    fn redraw(&mut self) {
        // draw to texture
        gl::BindFramebuffer(gl::FRAMEBUFFER, self.framebuffer);
        gl::Viewport(0, 0, self.dimx.renderLength, self.dimy.renderLength);
        gl::ClearColor(0.0, 0.0, 0.0, 0.0);
        gl::Clear(gl::COLOR_BUFFER_BIT);
        gl::Enable(gl::VERTEX_PROGRAM_POINT_SIZE);
        gl::Enable(gl::BLEND);
        gl::BlendFuncSeparate(gl::SRC_ALPHA, gl::ONE, gl::ONE, gl::ONE);

        self.vaoPoints.bind();
        self.programPoints.bind();

        let translation = cgmath::Matrix4::<f32>::from_translation(
            &cgmath::Vector3::<f32>::new(
                self.dimx.d / (self.dimx.max - self.dimx.min) * 2.0,
                self.dimy.d / (self.dimy.max - self.dimy.min) * 2.0,
                self.dimzDelta / (self.dimz.max - self.dimz.min) * 2.0
            )
        );
        let scale = cgmath::Matrix4::<f32>::new(
            self.dimx.s, 0.0f32, 0.0f32, 0.0f32,
            0.0f32, self.dimy.s, 0.0f32, 0.0f32,
            0.0f32, 0.0f32, self.dimzScale, 0.0f32,
            0.0f32, 0.0f32, 0.0f32, 1.0f32
        );
        let finalTransformation = translation.mul_m(&scale).mul_m(&self.projection);
        unsafe {
            gl::UniformMatrix4fv(self.ulocationPoints.transformation, 1, gl::FALSE, mem::transmute(&finalTransformation.as_fixed()[0][0]));
        }

        gl::Uniform1f(self.ulocationPoints.width, self.dimx.renderLength as f32);
        gl::Uniform1f(self.ulocationPoints.height, self.dimy.renderLength as f32);
        gl::Uniform1f(self.ulocationPoints.pointScale, self.pointScale);
        gl::Uniform1f(self.ulocationPoints.margin, MARGIN);

        self.vaoPoints.draw_array(hgl::Points, 0, self.table.len() as i32);

        // render to texture to viewport
        self.vaoTexture.bind();
        self.programTexture.bind();

        gl::BindFramebuffer(gl::FRAMEBUFFER, 0);
        gl::ClearColor(0.1, 0.1, 0.1, 1.0);
        gl::Clear(gl::COLOR_BUFFER_BIT);
        gl::Enable(gl::BLEND);
        gl::BlendFunc(gl::SRC_ALPHA, gl::ONE_MINUS_SRC_ALPHA);

        gl::BindTexture(gl::TEXTURE_2D, self.texture);
        gl::Uniform1i(self.ulocationTexture.fboTexture, 0);
        gl::Uniform1f(self.ulocationTexture.count, self.table.len() as f32);
        gl::Uniform1f(self.ulocationTexture.alpha, self.alphaScale);
        self.vaoTexture.draw_array(hgl::Triangles, 0, VERTEX_DATA_TEXTURE.len() as i32 / 2);

        // draw legend (reuse vaoTexture)
        self.programLegend.bind();
        gl::Uniform1f(self.ulocationLegend.width, self.dimx.renderLength as f32);
        gl::Uniform1f(self.ulocationLegend.height, self.dimy.renderLength as f32);
        gl::Uniform1f(self.ulocationLegend.margin, MARGIN as f32);
        self.vaoTexture.draw_array(hgl::Triangles, 0, VERTEX_DATA_TEXTURE.len() as i32 / 2);

        gl::BindVertexArray(0);
        gl::UseProgram(0);
        self.gl2d.clear_shader();
        let c = graphics::Context::abs(self.dimx.renderLength as f64, self.dimy.renderLength as f64)
            .rgb(0.23, 0.80, 0.62);

        if self.showHelp {
            let help_c = c.trans((self.dimx.renderLength as f64 / 2f64).floor(), (self.dimy.renderLength as f64 / 2f64).floor());
            self.textdrawer.render(&help_c, &mut self.gl2d, &HELP_TEXT.to_string(), textdrawer::Center, textdrawer::Middle);
        }
        self.draw_x_axis(&c);
        self.draw_y_axis(&c);
        self.draw_z_axis(&c);

        self.textdrawer.render(&c.trans(self.dimx.renderLength as f64 - INFO_MARGIN, INFO_MARGIN), &mut self.gl2d, &format!("#objects: {}", self.table.len()), textdrawer::Right, textdrawer::Top);

        self.window.swap_buffers();
    }

    fn renderloop(&mut self) {
        while !self.window.should_close() {
            self.glfw.poll_events();
            for (_time, event) in glfw::flush_messages(&self.events) {
                self.handle_event(event);
                self.changed = true;
            }

            if self.changed {
                self.redraw();
                self.changed = false;
            } else {
                io::timer::sleep(time::duration::Duration::milliseconds(PAUSE_MS));
            }
        }
    }
}

#[unsafe_destructor]
impl Drop for Renderer {
    fn drop(&mut self) {
        self.window.make_current();
        unsafe {
            gl::DeleteTextures(1, &self.texture);
            gl::DeleteFramebuffers(1, &self.framebuffer);
        }
    }
}

pub fn render(table: data::Table, column_x: &String, column_y: &String, column_z: &String) {
    let mut renderer = Renderer::new(table, column_x, column_y, column_z);
    renderer.renderloop();
}
