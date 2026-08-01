use bytemuck::{Pod, Zeroable};
use glam::{Mat4, Vec3};

#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
pub struct CameraUniform {
    pub view_proj: [[f32; 4]; 4],
}

pub struct Camera2D {
    pub position: Vec3,
    pub zoom: f32,
    pub rotation: f32,

    pub viewport_x: f32,
    pub viewport_y: f32,
    pub viewport_width: f32,
    pub viewport_height: f32,
}

impl Camera2D {
    pub fn new(viewport_x: f32, viewport_y: f32, width: f32, height: f32) -> Self {
        Self {
            position: Vec3::new(0.0, 0.0, 0.0),
            zoom: 1.0,
            rotation: 0.0,
            viewport_x,
            viewport_y,
            viewport_width: width,
            viewport_height: height,
        }
    }

    pub fn resize_fullscreen(&mut self, width: u32, height: u32) {
        self.viewport_x = 0.0;
        self.viewport_y = 0.0;
        self.viewport_width = width as f32;
        self.viewport_height = height as f32;
    }

    pub fn build_view_projection_matrix(&self) -> Mat4 {
        let aspect = if self.viewport_height > 0.0 {
            self.viewport_width / self.viewport_height
        } else {
            1.0
        };

        // Новая функция glam 0.33 для WebGPU/DirectX (Z от 0 до 1)
        let proj = glam::camera::lh::proj::directx::orthographic(
            -aspect * self.zoom, // left
            aspect * self.zoom,  // right
            -1.0 * self.zoom,    // bottom
            1.0 * self.zoom,     // top
            -1.0,                // near
            1.0,                 // far
        );

        let view = Mat4::from_translation(-self.position)
            * Mat4::from_rotation_z(self.rotation);

        proj * view
    }
}