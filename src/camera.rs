use glam::{Mat4, Vec3, Vec4};

#[derive(Debug, Clone)]
pub struct Camera {
    pub target: Vec3,
    pub yaw: f32,       // Horizontal angle (radians)
    pub pitch: f32,     // Vertical angle (radians)
    pub distance: f32,
    pub fov: f32,       // Field of view in radians
}

impl Default for Camera {
    fn default() -> Self {
        Self {
            target: Vec3::ZERO,
            yaw: 45.0f32.to_radians(),
            pitch: 30.0f32.to_radians(),
            distance: 60.0,
            fov: 40.0f32.to_radians(),
        }
    }
}

impl Camera {
    pub fn eye_position(&self) -> Vec3 {
        let x = self.distance * self.pitch.cos() * self.yaw.sin();
        let y = self.distance * self.pitch.sin();
        let z = self.distance * self.pitch.cos() * self.yaw.cos();
        self.target + Vec3::new(x, y, z)
    }

    pub fn view_matrix(&self) -> Mat4 {
        let eye = self.eye_position();
        let f = (self.target - eye).normalize_or_zero();
        let s = f.cross(Vec3::Y).normalize_or_zero();
        let u = s.cross(f);

        Mat4::from_cols(
            Vec4::new(s.x, u.x, -f.x, 0.0),
            Vec4::new(s.y, u.y, -f.y, 0.0),
            Vec4::new(s.z, u.z, -f.z, 0.0),
            Vec4::new(-s.dot(eye), -u.dot(eye), f.dot(eye), 1.0),
        )
    }

    pub fn proj_matrix(&self, aspect: f32) -> Mat4 {
        let near = (self.distance * 0.01).max(0.1);
        let far = (self.distance * 20.0).max(500.0);
        let f = 1.0 / (self.fov * 0.5).tan();

        Mat4::from_cols(
            Vec4::new(f / aspect, 0.0, 0.0, 0.0),
            Vec4::new(0.0, f, 0.0, 0.0),
            Vec4::new(0.0, 0.0, (far + near) / (near - far), -1.0),
            Vec4::new(0.0, 0.0, (2.0 * far * near) / (near - far), 0.0),
        )
    }

    pub fn orbit(&mut self, delta_x: f32, delta_y: f32) {
        self.yaw -= delta_x * 0.008;
        self.pitch += delta_y * 0.008;
        let limit = 89.0f32.to_radians();
        self.pitch = self.pitch.clamp(-limit, limit);
    }

    pub fn pan(&mut self, delta_x: f32, delta_y: f32) {
        let forward = (self.target - self.eye_position()).normalize_or_zero();
        let right = forward.cross(Vec3::Y).normalize_or_zero();
        let up = right.cross(forward).normalize_or_zero();

        let pan_speed = self.distance * 0.0015;
        self.target -= right * (delta_x * pan_speed);
        self.target += up * (delta_y * pan_speed);
    }

    pub fn zoom(&mut self, factor: f32) {
        self.distance = (self.distance * factor).clamp(1.0, 5000.0);
    }

    pub fn fit_to_bbox(&mut self, min: [f32; 3], max: [f32; 3]) {
        let min_v = Vec3::from_array(min);
        let max_v = Vec3::from_array(max);
        self.target = (min_v + max_v) * 0.5;
        let size = (max_v - min_v).length().max(5.0);
        self.distance = size * 2.2;
    }

    pub fn set_preset_view(&mut self, view: &'static str) {
        match view {
            "iso" => {
                self.yaw = 45.0f32.to_radians();
                self.pitch = 30.0f32.to_radians();
            }
            "top" => {
                self.yaw = 0.0;
                self.pitch = 89.0f32.to_radians();
            }
            "front" => {
                self.yaw = 0.0;
                self.pitch = 0.0;
            }
            "right" => {
                self.yaw = 90.0f32.to_radians();
                self.pitch = 0.0;
            }
            _ => {}
        }
    }
}
