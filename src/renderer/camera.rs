use glam::{Mat4, Vec2, Vec3, Vec4};

#[derive(Debug, Clone)]
pub struct IsometricCamera {
    pub target: Vec3,
    pub distance: f32,
    pub pitch_rad: f32,
    pub yaw_rad: f32,
    pub fov_y: f32,
    pub z_near: f32,
    pub z_far: f32,
}

impl Default for IsometricCamera {
    fn default() -> Self {
        Self {
            target: Vec3::new(50.0, 0.0, 50.0),
            distance: 40.0,
            pitch_rad: -55.0_f32.to_radians(),
            yaw_rad: 45.0_f32.to_radians(),
            fov_y: 45.0_f32.to_radians(),
            z_near: 0.5,
            z_far: 600.0,
        }
    }
}

impl IsometricCamera {
    pub fn new(target: Vec3) -> Self {
        Self {
            target: Vec3::new(target.x, 0.0, target.z),
            ..Default::default()
        }
    }

    pub fn eye_position(&self) -> Vec3 {
        let pitch = self.pitch_rad.abs();
        let height = self.distance * pitch.sin();
        let ground_dist = self.distance * pitch.cos();
        let offset = Vec3::new(
            ground_dist * self.yaw_rad.sin(),
            height,
            ground_dist * self.yaw_rad.cos(),
        );
        self.target + offset
    }

    pub fn view_matrix(&self) -> Mat4 {
        Mat4::look_at_rh(self.eye_position(), self.target, Vec3::Y)
    }

    pub fn proj_matrix(&self, aspect: f32) -> Mat4 {
        let mut proj = Mat4::perspective_rh(self.fov_y, aspect.max(0.001), self.z_near, self.z_far);
        proj.y_axis.y *= -1.0;
        proj
    }

    pub fn view_proj_matrix(&self, aspect: f32) -> Mat4 {
        self.proj_matrix(aspect) * self.view_matrix()
    }

    pub fn screen_ray(&self, screen_pos: Vec2, viewport_size: Vec2) -> (Vec3, Vec3) {
        let w = viewport_size.x.max(1.0);
        let h = viewport_size.y.max(1.0);
        let aspect = w / h;

        let ndc_x = (2.0 * screen_pos.x / w) - 1.0;
        let ndc_y = (2.0 * screen_pos.y / h) - 1.0;

        let inv_vp = self.view_proj_matrix(aspect).inverse();

        let near_clip = inv_vp * Vec4::new(ndc_x, ndc_y, 0.0, 1.0);
        let far_clip = inv_vp * Vec4::new(ndc_x, ndc_y, 1.0, 1.0);

        let near_world = near_clip.truncate() / near_clip.w;
        let far_world = far_clip.truncate() / far_clip.w;

        let dir = (far_world - near_world).normalize();
        (near_world, dir)
    }

    pub fn screen_to_ground(&self, screen_pos: Vec2, viewport_size: Vec2) -> Option<Vec2> {
        let (origin, dir) = self.screen_ray(screen_pos, viewport_size);
        if dir.y.abs() < 1e-6 {
            return None;
        }

        let t = -origin.y / dir.y;
        if t < 0.0 {
            return None;
        }

        let hit = origin + dir * t;
        Some(Vec2::new(hit.x, hit.z))
    }

    pub fn world_to_screen(&self, world_pos: Vec3, viewport_size: Vec2) -> Option<Vec2> {
        let w = viewport_size.x.max(1.0);
        let h = viewport_size.y.max(1.0);
        let aspect = w / h;

        let vp = self.view_proj_matrix(aspect);
        let clip = vp * Vec4::new(world_pos.x, world_pos.y, world_pos.z, 1.0);

        if clip.w <= 0.0 {
            return None;
        }

        let ndc = clip.truncate() / clip.w;
        if ndc.z < 0.0 || ndc.z > 1.0 {
            return None;
        }

        let screen_x = (ndc.x + 1.0) * 0.5 * w;
        let screen_y = (ndc.y + 1.0) * 0.5 * h;

        Some(Vec2::new(screen_x, screen_y))
    }

    pub fn zoom(&mut self, scroll: f32) {
        self.distance = (self.distance - scroll * 3.5).clamp(22.0, 75.0);
    }

    pub fn pan(&mut self, delta_x: f32, delta_z: f32, map_width: f32, map_height: f32) {
        let yaw = self.yaw_rad;
        let cos_y = yaw.cos();
        let sin_y = yaw.sin();

        // Forward on ground relative to isometric view
        let forward = Vec3::new(-sin_y, 0.0, -cos_y);
        let right = Vec3::new(cos_y, 0.0, -sin_y);

        let move_vec = right * delta_x + forward * delta_z;
        self.target += move_vec;
        self.target.x = self.target.x.clamp(0.0, map_width);
        self.target.z = self.target.z.clamp(0.0, map_height);
    }

    pub fn pan_screen_delta(
        &mut self,
        screen_delta: Vec2,
        viewport_size: Vec2,
        map_width: f32,
        map_height: f32,
    ) {
        let center = viewport_size * 0.5;
        let p0 = self.screen_to_ground(center, viewport_size);
        let p1 = self.screen_to_ground(center + screen_delta, viewport_size);

        if let (Some(g0), Some(g1)) = (p0, p1) {
            let d = g0 - g1;
            self.target.x = (self.target.x + d.x).clamp(0.0, map_width);
            self.target.z = (self.target.z + d.y).clamp(0.0, map_height);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_camera_center_projection() {
        let target = Vec3::new(60.0, 0.0, 40.0);
        let cam = IsometricCamera::new(target);
        let viewport = Vec2::new(1280.0, 720.0);

        let center = viewport * 0.5;
        let ground = cam.screen_to_ground(center, viewport).expect("hit ground");
        assert!((ground.x - target.x).abs() < 1e-2);
        assert!((ground.y - target.z).abs() < 1e-2);

        let screen_back = cam.world_to_screen(target, viewport).expect("world to screen");
        assert!((screen_back.x - center.x).abs() < 1e-2);
        assert!((screen_back.y - center.y).abs() < 1e-2);
    }

    #[test]
    fn test_zoom_clamp() {
        let mut cam = IsometricCamera::default();
        cam.zoom(100.0);
        assert!(cam.distance >= 22.0);
        cam.zoom(-200.0);
        assert!(cam.distance <= 75.0);
    }
}
