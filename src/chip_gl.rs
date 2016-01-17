use glium;
use glium::{DisplayBuild, Surface};
use traits::*;

#[derive(Copy, Clone)]
struct Vertex {
    position: [f32; 2],
    tex_coords: [f32; 2],
}

implement_vertex!(Vertex, position, tex_coords);

pub struct GliumRenderer {
    display: Box<glium::Display>,
    indicies: Box<glium::index::NoIndices>,
    program: Box<glium::Program>,
    vertex_buffer: Box<glium::VertexBuffer<Vertex>>,
    closed: bool,
    pressed_keys: [bool;16],
}

impl GliumRenderer {
    pub fn new() -> GliumRenderer {
        let display = glium::glutin::WindowBuilder::new()
            .with_dimensions(64*8, 32*8)
            .with_title(format!("Rust Chip8"))
            .build_glium()
            .unwrap();
        
        let top_right = Vertex { position: [1.0, 1.0], tex_coords: [1.0, 0.0] };
        let top_left = Vertex { position: [-1.0, 1.0], tex_coords: [0.0, 0.0] };
        let bottom_left = Vertex { position: [-1.0, -1.0],  tex_coords: [0.0, 1.0] };
        let bottom_right = Vertex { position: [1.0, -1.0], tex_coords: [1.0, 1.0] };

        let shape = vec![top_right, top_left, bottom_left, bottom_right];

        let vertex_buffer = glium::VertexBuffer::new(&display, &shape).unwrap();
        let indicies = glium::index::NoIndices(glium::index::PrimitiveType::TriangleFan);

        let vertex_shader_src = r#"
            #version 140

            in vec2 position;
            in vec2 tex_coords;

            out vec2 v_tex_coords;

            void main() {
                v_tex_coords = tex_coords;
                gl_Position = vec4(position, 0.0, 1.0);
            }
        "#;

        let fragment_shader_src = r#"
            #version 140

            in vec2 v_tex_coords;
            out vec4 color;

            uniform sampler2D tex;

            void main() {
                color = texture(tex, v_tex_coords);
            }
        "#;

        let program = glium::Program::from_source(&display, vertex_shader_src, fragment_shader_src, None).unwrap();

        GliumRenderer {
            display: Box::new(display),
            indicies: Box::new(indicies),
            program: Box::new(program),
            vertex_buffer: Box::new(vertex_buffer),
            closed: false,
            pressed_keys: [false;16],
        }
    }

    fn process_events(&mut self) {
        for ev in self.display.poll_events() {
            match ev {
                glium::glutin::Event::Closed => self.closed = true,
                glium::glutin::Event::KeyboardInput(state, x, key_opt) => {
                    let pressed = state == glium::glutin::ElementState::Pressed;
                    if let Some(key) = key_opt {
                        use glium::glutin::VirtualKeyCode;
                        match key {
                            VirtualKeyCode::Key1 => self.pressed_keys[1] = pressed,
                            VirtualKeyCode::Key2 => self.pressed_keys[2] = pressed,
                            VirtualKeyCode::Key3 => self.pressed_keys[3] = pressed,
                            VirtualKeyCode::Key4 => self.pressed_keys[0xC] = pressed,
                            VirtualKeyCode::Q => self.pressed_keys[4] = pressed,
                            VirtualKeyCode::W => self.pressed_keys[5] = pressed,
                            VirtualKeyCode::E => self.pressed_keys[6] = pressed,
                            VirtualKeyCode::R => self.pressed_keys[0xD] = pressed,
                            VirtualKeyCode::A => self.pressed_keys[7] = pressed,
                            VirtualKeyCode::S => self.pressed_keys[8] = pressed,
                            VirtualKeyCode::D => self.pressed_keys[9] = pressed,
                            VirtualKeyCode::F => self.pressed_keys[0xE] = pressed,
                            VirtualKeyCode::Z => self.pressed_keys[0xA] = pressed,
                            VirtualKeyCode::X => self.pressed_keys[0] = pressed,
                            VirtualKeyCode::C => self.pressed_keys[0xB] = pressed,
                            VirtualKeyCode::V => self.pressed_keys[0xF] = pressed,
                            _ => {} 
                        }
                    }
                }, 
                _ => {}
            }
        }
    }
}


impl Chip8System for GliumRenderer {
    fn render(&mut self, screen: &[u8; 2048]) {
        use glium::texture::{RawImage2d, ClientFormat, texture2d};
        let mut screen_buf = [0xFF000000u32; 2048];

        for x in 0..2048 {
            if screen[x] != 0 { screen_buf[x] = 0xFFFFFFFFu32; }
        }

        let img = RawImage2d {
            data: ::std::borrow::Cow::Owned(screen_buf.to_vec()),
            width: 64,
            height: 32,
            format: ClientFormat::U8U8U8U8,
        };

        let tex = texture2d::Texture2d::new(&*self.display, img).unwrap();

        let uniforms = uniform! {
            tex: glium::uniforms::Sampler::new(&tex)
                .magnify_filter(glium::uniforms::MagnifySamplerFilter::Nearest),
        };

        let mut target = self.display.draw(); 
        target.clear_color(0.0, 0.0, 0.0, 1.0);
        target.draw(&*self.vertex_buffer, &*self.indicies, &*self.program, &uniforms, &Default::default()).unwrap();
        target.finish().unwrap();
        self.process_events();
    }
    
    fn get_input(&mut self) -> Option<u8> {
        self.process_events();
        for x in 0..16u8 {
            if self.pressed_keys[x as usize] { return Some(x); }
        }
        None
    }

    fn is_closed(&mut self) -> bool {
        self.closed
    }
}
