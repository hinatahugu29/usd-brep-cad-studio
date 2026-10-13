use glam::Mat4;
use glow::HasContext;
use std::sync::Arc;

pub struct CadRenderer {
    gl: Arc<glow::Context>,
    mesh_program: glow::Program,
    line_program: glow::Program,
    mesh_vao: glow::VertexArray,
    mesh_vbo: glow::Buffer,
    line_vao: glow::VertexArray,
    line_vbo: glow::Buffer,
    grid_vao: glow::VertexArray,
    grid_vbo: glow::Buffer,
    grid_vertex_count: i32,
    triangle_vertex_count: i32,
    line_vertex_count: i32,
}

impl CadRenderer {
    pub fn new(gl: Arc<glow::Context>) -> Result<Self, String> {
        unsafe {
            // Compile Mesh Shader with Section Clipping support
            let mesh_vs = r#"#version 330 core
layout(location = 0) in vec3 a_pos;
layout(location = 1) in vec3 a_norm;
uniform mat4 u_mvp;
uniform mat4 u_model;
out vec3 v_norm;
out vec3 v_pos;
void main() {
    v_norm = normalize(mat3(u_model) * a_norm);
    v_pos = vec3(u_model * vec4(a_pos, 1.0));
    gl_Position = u_mvp * vec4(a_pos, 1.0);
}
"#;
            let mesh_fs = r#"#version 330 core
in vec3 v_norm;
in vec3 v_pos;
uniform vec3 u_color;
uniform vec3 u_light_dir;
uniform vec4 u_clip_plane;
uniform int u_use_clip;
out vec4 FragColor;
void main() {
    if (u_use_clip == 1) {
        if (dot(vec4(v_pos, 1.0), u_clip_plane) < 0.0) {
            discard;
        }
    }
    vec3 n = normalize(v_norm);
    vec3 l = normalize(u_light_dir);
    // Two-sided CAD lighting with key + fill light
    float diff1 = max(dot(n, l), 0.0);
    float diff2 = max(dot(-n, l), 0.0) * 0.4;
    float fill = max(dot(n, vec3(-l.x, -l.y, l.z)), 0.0) * 0.25;
    float ambient = 0.28;
    float light = ambient + diff1 * 0.65 + diff2 + fill;

    // Subtle specular highlight
    vec3 view_dir = normalize(vec3(0.0, 0.0, 1.0));
    vec3 half_dir = normalize(l + view_dir);
    float spec = pow(max(dot(n, half_dir), 0.0), 32.0) * 0.2;

    vec3 final_color = u_color * light + vec3(spec);
    FragColor = vec4(final_color, 1.0);
}
"#;
            let mesh_program = Self::create_program(&gl, mesh_vs, mesh_fs)?;

            // Compile Line / Wireframe Shader with Section Clipping
            let line_vs = r#"#version 330 core
layout(location = 0) in vec3 a_pos;
uniform mat4 u_mvp;
out vec3 v_pos;
void main() {
    v_pos = a_pos;
    gl_Position = u_mvp * vec4(a_pos, 1.0);
}
"#;
            let line_fs = r#"#version 330 core
in vec3 v_pos;
uniform vec3 u_line_color;
uniform vec4 u_clip_plane;
uniform int u_use_clip;
out vec4 FragColor;
void main() {
    if (u_use_clip == 1) {
        if (dot(vec4(v_pos, 1.0), u_clip_plane) < 0.0) {
            discard;
        }
    }
    FragColor = vec4(u_line_color, 1.0);
}
"#;
            let line_program = Self::create_program(&gl, line_vs, line_fs)?;

            // Mesh VAO & VBO
            let mesh_vao = gl.create_vertex_array().map_err(|e| e.to_string())?;
            let mesh_vbo = gl.create_buffer().map_err(|e| e.to_string())?;

            gl.bind_vertex_array(Some(mesh_vao));
            gl.bind_buffer(glow::ARRAY_BUFFER, Some(mesh_vbo));
            // pos: 3 floats, norm: 3 floats (stride = 6 * 4 = 24 bytes)
            gl.vertex_attrib_pointer_f32(0, 3, glow::FLOAT, false, 24, 0);
            gl.enable_vertex_attrib_array(0);
            gl.vertex_attrib_pointer_f32(1, 3, glow::FLOAT, false, 24, 12);
            gl.enable_vertex_attrib_array(1);

            // Line VAO & VBO
            let line_vao = gl.create_vertex_array().map_err(|e| e.to_string())?;
            let line_vbo = gl.create_buffer().map_err(|e| e.to_string())?;

            gl.bind_vertex_array(Some(line_vao));
            gl.bind_buffer(glow::ARRAY_BUFFER, Some(line_vbo));
            gl.vertex_attrib_pointer_f32(0, 3, glow::FLOAT, false, 12, 0);
            gl.enable_vertex_attrib_array(0);

            // Ground Grid VAO & VBO
            let grid_vao = gl.create_vertex_array().map_err(|e| e.to_string())?;
            let grid_vbo = gl.create_buffer().map_err(|e| e.to_string())?;
            let (grid_data, grid_count) = Self::generate_grid_lines(50.0, 20);

            gl.bind_vertex_array(Some(grid_vao));
            gl.bind_buffer(glow::ARRAY_BUFFER, Some(grid_vbo));
            gl.buffer_data_u8_slice(
                glow::ARRAY_BUFFER,
                bytemuck_slice(&grid_data),
                glow::STATIC_DRAW,
            );
            gl.vertex_attrib_pointer_f32(0, 3, glow::FLOAT, false, 12, 0);
            gl.enable_vertex_attrib_array(0);

            gl.bind_vertex_array(None);

            Ok(Self {
                gl,
                mesh_program,
                line_program,
                mesh_vao,
                mesh_vbo,
                line_vao,
                line_vbo,
                grid_vao,
                grid_vbo,
                grid_vertex_count: grid_count,
                triangle_vertex_count: 0,
                line_vertex_count: 0,
            })
        }
    }

    fn create_program(gl: &glow::Context, vs_src: &str, fs_src: &str) -> Result<glow::Program, String> {
        unsafe {
            let vs = gl.create_shader(glow::VERTEX_SHADER).map_err(|e| e.to_string())?;
            gl.shader_source(vs, vs_src);
            gl.compile_shader(vs);
            if !gl.get_shader_compile_status(vs) {
                return Err(format!("VS compile error: {}", gl.get_shader_info_log(vs)));
            }

            let fs = gl.create_shader(glow::FRAGMENT_SHADER).map_err(|e| e.to_string())?;
            gl.shader_source(fs, fs_src);
            gl.compile_shader(fs);
            if !gl.get_shader_compile_status(fs) {
                return Err(format!("FS compile error: {}", gl.get_shader_info_log(fs)));
            }

            let program = gl.create_program().map_err(|e| e.to_string())?;
            gl.attach_shader(program, vs);
            gl.attach_shader(program, fs);
            gl.link_program(program);
            if !gl.get_program_link_status(program) {
                return Err(format!("Program link error: {}", gl.get_program_info_log(program)));
            }
            gl.delete_shader(vs);
            gl.delete_shader(fs);

            Ok(program)
        }
    }

    fn generate_grid_lines(size: f32, steps: i32) -> (Vec<f32>, i32) {
        let mut lines = Vec::new();
        let step_size = (size * 2.0) / (steps as f32);
        for i in -steps..=steps {
            let p = i as f32 * step_size;
            // Lines parallel to X (along X at y=p, z=0)
            lines.extend_from_slice(&[-size, p, 0.0, size, p, 0.0]);
            // Lines parallel to Y (along Y at x=p, z=0)
            lines.extend_from_slice(&[p, -size, 0.0, p, size, 0.0]);
        }
        let count = (lines.len() / 3) as i32;
        (lines, count)
    }

    pub fn update_mesh_data(&mut self, vertex_bytes: &[u8], line_bytes: &[u8]) {
        unsafe {
            // Update Mesh VBO
            self.gl.bind_buffer(glow::ARRAY_BUFFER, Some(self.mesh_vbo));
            self.gl.buffer_data_u8_slice(glow::ARRAY_BUFFER, vertex_bytes, glow::DYNAMIC_DRAW);
            // Each vertex is 6 floats (24 bytes)
            self.triangle_vertex_count = (vertex_bytes.len() / 24) as i32;

            // Update Crease / Wireframe Lines VBO
            self.gl.bind_buffer(glow::ARRAY_BUFFER, Some(self.line_vbo));
            self.gl.buffer_data_u8_slice(glow::ARRAY_BUFFER, line_bytes, glow::DYNAMIC_DRAW);
            // Each line vertex is 3 floats (12 bytes)
            self.line_vertex_count = (line_bytes.len() / 12) as i32;

            self.gl.bind_buffer(glow::ARRAY_BUFFER, None);
        }
    }

    pub fn render(
        &self,
        viewport_x: i32,
        viewport_y: i32,
        viewport_w: i32,
        viewport_h: i32,
        mvp: &Mat4,
        model: &Mat4,
        body_color: [f32; 3],
        clip_plane: Option<[f32; 4]>,
        show_wireframe: bool,
        show_grid: bool,
    ) {
        unsafe {
            self.gl.viewport(viewport_x, viewport_y, viewport_w, viewport_h);
            self.gl.enable(glow::SCISSOR_TEST);
            self.gl.scissor(viewport_x, viewport_y, viewport_w, viewport_h);
            // Nord slate viewport background (#2e3440)
            self.gl.clear_color(0.1804, 0.2039, 0.2510, 1.0);
            self.gl.clear_depth_f32(1.0);
            self.gl.depth_mask(true);
            self.gl.clear(glow::COLOR_BUFFER_BIT | glow::DEPTH_BUFFER_BIT);
            self.gl.enable(glow::DEPTH_TEST);
            self.gl.depth_func(glow::LEQUAL);
            self.gl.enable(glow::CULL_FACE);
            self.gl.cull_face(glow::BACK);

            let mvp_slice = mvp.to_cols_array();
            let model_slice = model.to_cols_array();

            let use_clip = if clip_plane.is_some() { 1 } else { 0 };
            let clip_vals = clip_plane.unwrap_or([0.0, 0.0, 1.0, 0.0]);

            // 1. Render Grid
            if show_grid {
                self.gl.use_program(Some(self.line_program));
                let u_mvp = self.gl.get_uniform_location(self.line_program, "u_mvp");
                let u_color = self.gl.get_uniform_location(self.line_program, "u_line_color");
                let u_use_clip = self.gl.get_uniform_location(self.line_program, "u_use_clip");
                self.gl.uniform_matrix_4_f32_slice(u_mvp.as_ref(), false, &mvp_slice);
                self.gl.uniform_3_f32(u_color.as_ref(), 0.25, 0.28, 0.34);
                self.gl.uniform_1_i32(u_use_clip.as_ref(), 0); // Don't clip grid

                self.gl.bind_vertex_array(Some(self.grid_vao));
                self.gl.draw_arrays(glow::LINES, 0, self.grid_vertex_count);
            }

            // 2. Render Mesh Solids
            if self.triangle_vertex_count > 0 {
                // Polygon offset so edges don't Z-fight with surface
                self.gl.enable(glow::POLYGON_OFFSET_FILL);
                self.gl.polygon_offset(1.0, 1.0);

                self.gl.use_program(Some(self.mesh_program));
                let u_mvp = self.gl.get_uniform_location(self.mesh_program, "u_mvp");
                let u_model = self.gl.get_uniform_location(self.mesh_program, "u_model");
                let u_color = self.gl.get_uniform_location(self.mesh_program, "u_color");
                let u_light = self.gl.get_uniform_location(self.mesh_program, "u_light_dir");
                let u_clip = self.gl.get_uniform_location(self.mesh_program, "u_clip_plane");
                let u_use_clip = self.gl.get_uniform_location(self.mesh_program, "u_use_clip");

                self.gl.uniform_matrix_4_f32_slice(u_mvp.as_ref(), false, &mvp_slice);
                self.gl.uniform_matrix_4_f32_slice(u_model.as_ref(), false, &model_slice);
                // Active solid appearance color
                self.gl.uniform_3_f32(u_color.as_ref(), body_color[0], body_color[1], body_color[2]);
                // Studio lighting direction
                self.gl.uniform_3_f32(u_light.as_ref(), 0.55, 0.65, 0.75);
                // Section analysis clipping plane
                self.gl.uniform_1_i32(u_use_clip.as_ref(), use_clip);
                self.gl.uniform_4_f32(u_clip.as_ref(), clip_vals[0], clip_vals[1], clip_vals[2], clip_vals[3]);

                self.gl.bind_vertex_array(Some(self.mesh_vao));
                self.gl.draw_arrays(glow::TRIANGLES, 0, self.triangle_vertex_count);
                self.gl.disable(glow::POLYGON_OFFSET_FILL);
            }

            // 3. Render Feature / Crease Wireframe (crisp body edge #101012)
            if show_wireframe && self.line_vertex_count > 0 {
                self.gl.use_program(Some(self.line_program));
                let u_mvp = self.gl.get_uniform_location(self.line_program, "u_mvp");
                let u_color = self.gl.get_uniform_location(self.line_program, "u_line_color");
                let u_clip = self.gl.get_uniform_location(self.line_program, "u_clip_plane");
                let u_use_clip = self.gl.get_uniform_location(self.line_program, "u_use_clip");

                self.gl.uniform_matrix_4_f32_slice(u_mvp.as_ref(), false, &mvp_slice);
                // Crisp dark CAD edge line
                self.gl.uniform_3_f32(u_color.as_ref(), 0.063, 0.063, 0.071);
                self.gl.uniform_1_i32(u_use_clip.as_ref(), use_clip);
                self.gl.uniform_4_f32(u_clip.as_ref(), clip_vals[0], clip_vals[1], clip_vals[2], clip_vals[3]);

                self.gl.bind_vertex_array(Some(self.line_vao));
                self.gl.line_width(1.5);
                self.gl.draw_arrays(glow::LINES, 0, self.line_vertex_count);
            }

            self.gl.bind_vertex_array(None);
            self.gl.use_program(None);
            self.gl.disable(glow::DEPTH_TEST);
            self.gl.disable(glow::CULL_FACE);
            self.gl.disable(glow::SCISSOR_TEST);
        }
    }
}

impl Drop for CadRenderer {
    fn drop(&mut self) {
        unsafe {
            self.gl.delete_program(self.mesh_program);
            self.gl.delete_program(self.line_program);
            self.gl.delete_vertex_array(self.mesh_vao);
            self.gl.delete_buffer(self.mesh_vbo);
            self.gl.delete_vertex_array(self.line_vao);
            self.gl.delete_buffer(self.line_vbo);
            self.gl.delete_vertex_array(self.grid_vao);
            self.gl.delete_buffer(self.grid_vbo);
        }
    }
}

fn bytemuck_slice(slice: &[f32]) -> &[u8] {
    unsafe {
        std::slice::from_raw_parts(
            slice.as_ptr() as *const u8,
            slice.len() * std::mem::size_of::<f32>(),
        )
    }
}
