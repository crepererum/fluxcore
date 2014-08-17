use cgmath;
use cgmath::array::FixedArray;
use cgmath::matrix::Matrix;
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
use std::mem;
use textdrawer;

static VERTEX_SHADER: &'static str = "
#version 140

uniform float pointScale;
uniform mat4 transformation;
uniform float alphaScale;

in float position_x;
in float position_y;
out vec4 Color;
out vec2 Position;

void main() {
    gl_PointSize = pointScale;
    gl_Position = transformation * vec4(position_x, position_y, 0.0, 1.0);
    Color = vec4(1.0, 1.0, 1.0, alphaScale);
    Position = vec2(gl_Position.x, gl_Position.y);
}";

static FRAGMENT_SHADER: &'static str = "
#version 140

uniform float pointScale;
uniform float width;
uniform float height;
uniform float margin;

in vec4 Color;
in vec2 Position;
out vec4 out_color;

void main() {
    if (
               (gl_FragCoord.x < margin)
            || (gl_FragCoord.x >= width - margin)
            || (gl_FragCoord.y < margin)
            || (gl_FragCoord.y >= height - margin)) {
        discard;
    }

    float x = (Position.x + 1.0) / 2.0 * width;
    float y = (Position.y + 1.0) / 2.0 * height;
    float dx = x - gl_FragCoord.x;
    float dy = y - gl_FragCoord.y;
    float step1 = 0.5 * pointScale;
    float step0 = max(0.25 * pointScale, step1 - 2.0);
    float alpha = 1.0 - smoothstep(step0 * step0, step1 * step1, dx * dx + dy * dy);

    out_color = vec4(Color.r, Color.g, Color.b, Color.a * alpha);
}";

static HELP_TEXT: &'static str = "
== HELP ==
H: Toggle help
R: Reset view
Q/W: Decrease/Increase point size
A/S: Decrease/Increase alpha
Up/Down: Change Y dimension
Left/Right: Change X dimension
Mouse 1 + Drag: Move
Mouse 2 + Drag: Scale
";

static MARGIN: f32 = 75f32;
static TICK_DISTANCE: i32 = 60i32;
static FONT_SIZE: u32 = 16u32;
static LABEL_MARGIN: f64 = 24f64;
static TICK_LENGTH: f64 = 6f64;
static TICK_WIDTH: f64 = 0.5f64;

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

fn calc_projection(dimx: &Dimension, dimy: &Dimension) -> cgmath::matrix::Matrix4<f32> {
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

    cgmath::projection::ortho(
        xmin, xmax,
        ymin, ymax,
        0f32, 1f32
    )
}

struct UniformLocation {
    width: gl::types::GLint,
    height: gl::types::GLint,
    pointScale: gl::types::GLint,
    transformation: gl::types::GLint,
    margin: gl::types::GLint,
    alphaScale: gl::types::GLint,
}

struct Renderer {
    table: data::Table,
    glfw: glfw::Glfw,
    window: glfw::Window,
    events: comm::Receiver<(f64, glfw::WindowEvent)>,
    dimx: Dimension,
    dimy: Dimension,
    activeTransform: ActiveTransform,
    mouseX: f32,
    mouseY: f32,
    pointScale: f32,
    alphaScale: f32,
    projection: cgmath::matrix::Matrix4<f32>,
    ulocation: UniformLocation,
    vao: hgl::vao::Vao,
    program: hgl::program::Program,
    textdrawer: textdrawer::TextDrawer,
    gl2d: opengl_graphics::Gl,
    showHelp: bool,
}

impl Renderer {
    fn new(table: data::Table, column_x: &String, column_y: &String) -> Renderer {
        let width = 800i32;
        let height = 600i32;

        let glfw = glfw::init(glfw::FAIL_ON_ERRORS).unwrap();
        glfw.window_hint(glfw::ContextVersion(3, 1));

        let (window, events) = glfw.create_window(width as u32, height as u32, format!("fluxcore - {}", table.name()).as_slice(), glfw::Windowed).unwrap();
        window.set_all_polling(true);
        window.make_current();
        gl::load_with(|p| glfw.get_proc_address(p));

        gl::Viewport(0, 0, width, height);

        let vao = hgl::Vao::new();
        vao.bind();

        let program = hgl::Program::link([
            hgl::Shader::compile(VERTEX_SHADER, hgl::VertexShader),
            hgl::Shader::compile(FRAGMENT_SHADER, hgl::FragmentShader)
        ]).unwrap();
        let ulocation = UniformLocation{
            width: program.uniform("width"),
            height: program.uniform("height"),
            pointScale: program.uniform("pointScale"),
            transformation: program.uniform("transformation"),
            margin: program.uniform("margin"),
            alphaScale: program.uniform("alphaScale"),
        };
        program.bind_frag(0, "out_color");
        program.bind();

        let dimx = Dimension::new(width, &table, column_x);
        vao.enable_attrib(&program, "position_x", gl::FLOAT, 1, (1 * mem::size_of::<f32>()) as i32, 0);
        dimx.vbo.bind();

        let dimy = Dimension::new(height, &table, column_y);
        vao.enable_attrib(&program, "position_y", gl::FLOAT, 1, (1 * mem::size_of::<f32>()) as i32, 0);
        dimy.vbo.bind();

        let projection = calc_projection(&dimx, &dimy);

        Renderer {
            glfw: glfw,
            window: window,
            events: events,
            dimx: dimx,
            dimy: dimy,
            activeTransform: TransformNone,
            mouseX: 0f32,
            mouseY: 0f32,
            pointScale: 4f32,
            alphaScale: 1f32,
            projection: projection,
            ulocation: ulocation,
            vao: vao,
            program: program,
            table: table,
            textdrawer: textdrawer::TextDrawer::new("res/DejaVuSansCondensed-Bold.ttf".to_string(), FONT_SIZE),
            gl2d: opengl_graphics::Gl::new(),
            showHelp: false,
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

    fn handle_event(&mut self, event: glfw::WindowEvent) {
        match event {
            glfw::SizeEvent(w, h) => {
                self.dimx.renderLength = w;
                self.dimy.renderLength = h;
                gl::Viewport(0, 0, self.dimx.renderLength, self.dimy.renderLength);
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
            glfw::KeyEvent(key, _scancode, action, _mods) => {
                match (key, action) {
                    (glfw::KeyEscape, glfw::Press) => self.window.set_should_close(true),
                    (glfw::KeyW, glfw::Press) => self.pointScale *= 1.5f32,
                    (glfw::KeyQ, glfw::Press) => self.pointScale = 1f32.max(self.pointScale / 1.5f32),
                    (glfw::KeyA, glfw::Press) => self.alphaScale /= 1.5f32,
                    (glfw::KeyS, glfw::Press) => self.alphaScale = 1f32.min(self.alphaScale * 1.5f32),
                    (glfw::KeyH, glfw::Press) => self.showHelp = !self.showHelp,
                    (glfw::KeyR, glfw::Press) => {
                        self.pointScale = 4f32;
                        self.alphaScale = 1f32;
                        self.dimx.reset();
                        self.dimy.reset();
                    },
                    (glfw::KeyRight, glfw::Press) => {
                        {
                            let next = match self.table.columns().iter().skip_while(|&s| s != &self.dimx.name).skip(1).next() {
                                Some(element) => element,
                                None => self.table.columns().iter().next().unwrap()
                            };
                            let dim = Dimension::new(self.dimx.renderLength, &self.table, next);
                            self.vao.enable_attrib(&self.program, "position_x", gl::FLOAT, 1, (1 * mem::size_of::<f32>()) as i32, 0);
                            dim.vbo.bind();
                            self.dimx = dim;
                        }
                        self.projection = calc_projection(&self.dimx, &self.dimy);
                    },
                    (glfw::KeyLeft, glfw::Press) => {
                        {
                            let next = match self.table.columns().rev_iter().skip_while(|&s| s != &self.dimx.name).skip(1).next() {
                                Some(element) => element,
                                None => self.table.columns().rev_iter().next().unwrap()
                            };
                            let dim = Dimension::new(self.dimx.renderLength, &self.table, next);
                            self.vao.enable_attrib(&self.program, "position_x", gl::FLOAT, 1, (1 * mem::size_of::<f32>()) as i32, 0);
                            dim.vbo.bind();
                            self.dimx = dim;
                        }
                        self.projection = calc_projection(&self.dimx, &self.dimy);
                    },
                    (glfw::KeyDown, glfw::Press) => {
                        {
                            let next = match self.table.columns().iter().skip_while(|&s| s != &self.dimy.name).skip(1).next() {
                                Some(element) => element,
                                None => self.table.columns().iter().next().unwrap()
                            };
                            let dim = Dimension::new(self.dimy.renderLength, &self.table, next);
                            self.vao.enable_attrib(&self.program, "position_y", gl::FLOAT, 1, (1 * mem::size_of::<f32>()) as i32, 0);
                            dim.vbo.bind();
                            self.dimy = dim;
                        }
                        self.projection = calc_projection(&self.dimx, &self.dimy);
                    },
                    (glfw::KeyUp, glfw::Press) => {
                        {
                            let next = match self.table.columns().rev_iter().skip_while(|&s| s != &self.dimy.name).skip(1).next() {
                                Some(element) => element,
                                None => self.table.columns().rev_iter().next().unwrap()
                            };
                            let dim = Dimension::new(self.dimy.renderLength, &self.table, next);
                            self.vao.enable_attrib(&self.program, "position_y", gl::FLOAT, 1, (1 * mem::size_of::<f32>()) as i32, 0);
                            dim.vbo.bind();
                            self.dimy = dim;
                        }
                        self.projection = calc_projection(&self.dimx, &self.dimy);
                    },
                    _ => ()
                }
            }
            _ => ()
        }
    }

    fn renderloop(&mut self) {
        while !self.window.should_close() {
            self.glfw.poll_events();
            for (_time, event) in glfw::flush_messages(&self.events) {
                self.handle_event(event);
            }

            gl::ClearColor(0.1, 0.1, 0.1, 1.0);
            gl::Clear(gl::COLOR_BUFFER_BIT);
            gl::Enable(gl::VERTEX_PROGRAM_POINT_SIZE);
            gl::Enable(gl::BLEND);
            gl::BlendFunc(gl::SRC_ALPHA, gl::ONE_MINUS_SRC_ALPHA);

            self.vao.bind();
            self.program.bind();

            let translation = cgmath::matrix::Matrix4::<f32>::from_translation(
                &cgmath::vector::Vector3::<f32>::new(
                    self.dimx.d / (self.dimx.max - self.dimx.min) * 2.0,
                    self.dimy.d / (self.dimy.max - self.dimy.min) * 2.0,
                    0f32
                )
            );
            let scale = cgmath::matrix::Matrix4::<f32>::new(
                self.dimx.s, 0.0f32, 0.0f32, 0.0f32,
                0.0f32, self.dimy.s, 0.0f32, 0.0f32,
                0.0f32, 0.0f32, 1.0f32, 0.0f32,
                0.0f32, 0.0f32, 0.0f32, 1.0f32
            );
            let finalTransformation = translation.mul_m(&scale).mul_m(&self.projection);
            unsafe {
                gl::UniformMatrix4fv(self.ulocation.transformation, 1, gl::FALSE, mem::transmute(&finalTransformation.as_fixed()[0][0]));
            }

            gl::Uniform1f(self.ulocation.width, self.dimx.renderLength as f32);
            gl::Uniform1f(self.ulocation.height, self.dimy.renderLength as f32);
            gl::Uniform1f(self.ulocation.pointScale, self.pointScale);
            gl::Uniform1f(self.ulocation.alphaScale, self.alphaScale);
            gl::Uniform1f(self.ulocation.margin, MARGIN);

            self.vao.draw_array(hgl::Points, 0, self.table.len() as i32);

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

            self.window.swap_buffers();
        }
    }
}

pub fn render(table: data::Table, column_x: &String, column_y: &String) {
    let mut renderer = Renderer::new(table, column_x, column_y);
    renderer.renderloop();
}
