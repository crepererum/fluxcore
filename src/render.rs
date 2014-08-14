use cgmath;
use cgmath::array::FixedArray;
use cgmath::matrix::Matrix;
use data;
use gl;
use glfw;
use glfw::Context;
use hgl;
use std::comm;
use std::mem;

static VERTEX_SHADER: &'static str = "
#version 140

uniform float pointScale;
uniform mat4 transformation;

in float position_x;
in float position_y;
out vec4 Color;
out vec2 Position;

void main() {
    gl_PointSize = pointScale;
    gl_Position = transformation * vec4(position_x, position_y, 0.0, 1.0);
    Color = vec4(1.0, 1.0, 1.0, 1.0);
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

static MARGIN: f32 = 50f32;

fn range_vec(vec: &Vec<f32>) -> (f32, f32) {
    let min = vec.tail().iter().fold(vec[0] + 0.0, |a, &b| a.min(b));
    let max = vec.tail().iter().fold(vec[0] + 0.0, |a, &b| a.max(b));
    (min, max)
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
}

impl Dimension {
    fn new(renderLength: i32, data: &Vec<f32>) -> Dimension {
        let (min, max) = range_vec(data);
        let vbo = hgl::Vbo::from_data(data.as_slice(), hgl::StaticDraw);
        Dimension{
            renderLength: renderLength,
            d: 0f32,
            s: (1f32 - 2f32 * MARGIN / renderLength as f32) * 0.9f32,
            vbo: vbo,
            min: min,
            max: max,
        }
    }

    fn reset(&mut self) {
        self.d = 0f32;
        self.s = (1f32 - 2f32 * MARGIN / self.renderLength as f32) * 0.9f32;
    }
}

struct UniformLocation {
    width: gl::types::GLint,
    height: gl::types::GLint,
    pointScale: gl::types::GLint,
    transformation: gl::types::GLint,
    margin: gl::types::GLint,
}

struct Renderer {
    glfw: glfw::Glfw,
    window: glfw::Window,
    events: comm::Receiver<(f64, glfw::WindowEvent)>,
    dimx: Dimension,
    dimy: Dimension,
    activeTransform: ActiveTransform,
    mouseX: f32,
    mouseY: f32,
    pointScale: f32,
    projection: cgmath::matrix::Matrix4<f32>,
    ulocation: UniformLocation,
    vao: hgl::vao::Vao,
    size: gl::types::GLsizei
}

impl Renderer {
    fn new(table: data::Table, column_x: &String, column_y: &String) -> Renderer {
        let width = 800i32;
        let height = 600i32;

        let glfw = glfw::init(glfw::FAIL_ON_ERRORS).unwrap();
        glfw.window_hint(glfw::ContextVersion(3, 1));

        let (window, events) = glfw.create_window(width as u32, height as u32, format!("fluxcore2 - {}", table.name()).as_slice(), glfw::Windowed).unwrap();
        window.set_all_polling(true);
        window.make_current();
        gl::load_with(|p| glfw.get_proc_address(p));

        gl::Viewport(0, 0, width, height);
        gl::Enable(gl::VERTEX_PROGRAM_POINT_SIZE);
        gl::Enable(gl::BLEND);
        gl::BlendFunc(gl::SRC_ALPHA, gl::ONE_MINUS_SRC_ALPHA);

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
        };
        program.bind_frag(0, "out_color");
        program.bind();

        let dimx = Dimension::new(width, table.get(column_x).unwrap());
        vao.enable_attrib(&program, "position_x", gl::FLOAT, 1, (1 * mem::size_of::<f32>()) as i32, 0);
        dimx.vbo.bind();

        let dimy = Dimension::new(height, table.get(column_y).unwrap());
        vao.enable_attrib(&program, "position_y", gl::FLOAT, 1, (1 * mem::size_of::<f32>()) as i32, 0);
        dimy.vbo.bind();

        let projection = cgmath::projection::ortho(
            dimx.min, dimx.max,
            dimy.min, dimy.max,
            0f32, 1f32
        );

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
            projection: projection,
            ulocation: ulocation,
            vao: vao,
            size: table.len() as i32,
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

                        self.dimx.d += xdiff;
                        self.dimy.d += ydiff;
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
                    (glfw::KeyQ, glfw::Press) => self.pointScale *= 1.5f32,
                    (glfw::KeyW, glfw::Press) => self.pointScale = 1f32.max(self.pointScale / 1.5f32),
                    (glfw::KeyR, glfw::Press) => {
                        self.pointScale = 4f32;
                        self.dimx.reset();
                        self.dimy.reset();
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

            let translation = cgmath::matrix::Matrix4::<f32>::from_translation(
                &cgmath::vector::Vector3::<f32>::new(
                    self.dimx.d,
                    self.dimy.d,
                    0f32
                )
            );
            let scale = cgmath::matrix::Matrix4::<f32>::new(
                self.dimx.s, 0.0f32, 0.0f32, 0.0f32,
                0.0f32, self.dimy.s, 0.0f32, 0.0f32,
                0.0f32, 0.0f32, 1.0f32, 0.0f32,
                0.0f32, 0.0f32, 0.0f32, 1.0f32
            );
            let finalTransformation = self.projection.mul_m(&translation).mul_m(&scale);
            unsafe {
                gl::UniformMatrix4fv(self.ulocation.transformation, 1, gl::FALSE, mem::transmute(&finalTransformation.as_fixed()[0][0]));
            }

            gl::Uniform1f(self.ulocation.width, self.dimx.renderLength as f32);
            gl::Uniform1f(self.ulocation.height, self.dimy.renderLength as f32);
            gl::Uniform1f(self.ulocation.pointScale, self.pointScale);
            gl::Uniform1f(self.ulocation.margin, MARGIN);

            self.vao.draw_array(hgl::Points, 0, self.size);

            self.window.swap_buffers();
        }
    }
}

pub fn render(table: data::Table, column_x: &String, column_y: &String) {
    let mut renderer = Renderer::new(table, column_x, column_y);
    renderer.renderloop();
}
