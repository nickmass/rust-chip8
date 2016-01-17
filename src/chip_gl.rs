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

        }
    }
}


impl Chip8Renderer for GliumRenderer {
    fn render(&mut self, screen: &[u8; 2048]) {
        use glium::texture::{RawImage2d, ClientFormat, texture2d};
        let mut screen_buf = [0xFF000000u32; 2048];

        for x in 0..2048 {
            if screen[x] != 0 { screen_buf[x] = 0xFFFFFFFFu32; }
        }

        let img = glium::texture::RawImage2d {
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
    }
}
