#[derive(Debug, Clone)]
pub struct CameraPosition {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

impl CameraPosition {
    pub fn new(x: f32, y: f32, z: f32) -> Self {
        Self { x, y, z }
    }
}

#[derive(Debug, Clone)]
pub struct CameraMatrix {
    pub data: [f32; 16], // 4x4 matrix stored as a flat array
}

impl CameraMatrix {
    #[allow(dead_code)]
    pub fn new() -> Self {
        // Identity matrix
        let mut data = [0.0f32; 16];
        data[0] = 1.0;  // m00
        data[5] = 1.0;  // m11
        data[10] = 1.0; // m22
        data[15] = 1.0; // m33
        Self { data }
    }
    
    pub fn get_position(&self) -> CameraPosition {
        CameraPosition::new(self.data[12], self.data[13], self.data[14])
    }
    
    pub fn set_position(&mut self, pos: &CameraPosition) {
        self.data[12] = pos.x;
        self.data[13] = pos.y;
        self.data[14] = pos.z;
    }
    
    pub fn apply_pitch(&mut self, angle: f32) {
        // Apply pitch rotation around X-axis
        let cos_a = angle.cos();
        let sin_a = angle.sin();
        
        // Save current matrix
        let old_matrix = self.data.clone();
        
        // Apply pitch rotation to the rotation part of the matrix
        // Only modify the rotation components, keep position intact
        let new_right_y = old_matrix[1] * cos_a - old_matrix[2] * sin_a;
        let new_right_z = old_matrix[1] * sin_a + old_matrix[2] * cos_a;
        
        let new_up_y = old_matrix[5] * cos_a - old_matrix[6] * sin_a;
        let new_up_z = old_matrix[5] * sin_a + old_matrix[6] * cos_a;
        
        let new_forward_y = old_matrix[9] * cos_a - old_matrix[10] * sin_a;
        let new_forward_z = old_matrix[9] * sin_a + old_matrix[10] * cos_a;
        
        // Update the matrix
        self.data[1] = new_right_y;
        self.data[2] = new_right_z;
        self.data[5] = new_up_y;
        self.data[6] = new_up_z;
        self.data[9] = new_forward_y;
        self.data[10] = new_forward_z;
    }
    
    pub fn apply_yaw(&mut self, angle: f32) {
        // Apply yaw rotation around Y-axis
        let cos_a = angle.cos();
        let sin_a = angle.sin();
        
        // Save current matrix
        let old_matrix = self.data.clone();
        
        // Apply yaw rotation to the rotation part of the matrix
        // Only modify the rotation components, keep position intact
        let new_right_x = old_matrix[0] * cos_a + old_matrix[2] * sin_a;
        let new_right_z = -old_matrix[0] * sin_a + old_matrix[2] * cos_a;
        
        let new_up_x = old_matrix[4] * cos_a + old_matrix[6] * sin_a;
        let new_up_z = -old_matrix[4] * sin_a + old_matrix[6] * cos_a;
        
        let new_forward_x = old_matrix[8] * cos_a + old_matrix[10] * sin_a;
        let new_forward_z = -old_matrix[8] * sin_a + old_matrix[10] * cos_a;
        
        // Update the matrix
        self.data[0] = new_right_x;
        self.data[2] = new_right_z;
        self.data[4] = new_up_x;
        self.data[6] = new_up_z;
        self.data[8] = new_forward_x;
        self.data[10] = new_forward_z;
    }
    
    pub fn apply_translation(&mut self, dx: f32, dy: f32, dz: f32) {
        // Apply translation based on current rotation
        let forward_x = self.data[8];
        let forward_y = self.data[9];
        let forward_z = self.data[10];
        
        let right_x = self.data[0];
        let right_y = self.data[1];
        let right_z = self.data[2];
        
        let up_x = self.data[4];
        let up_y = self.data[5];
        let up_z = self.data[6];
        
        self.data[12] += dx * right_x + dy * up_x + dz * forward_x;
        self.data[13] += dx * right_y + dy * up_y + dz * forward_y;
        self.data[14] += dx * right_z + dy * up_z + dz * forward_z;
    }
    
    pub fn multiply_matrix(&mut self, a: &[f32; 16], b: &[f32; 16]) {
        let mut result = [0.0f32; 16];
        
        for i in 0..4 {
            for j in 0..4 {
                for k in 0..4 {
                    result[i * 4 + j] += a[i * 4 + k] * b[k * 4 + j];
                }
            }
        }
        
        self.data = result;
    }
    
    pub fn get_forward(&self) -> CameraPosition {
        // Forward vector is the negative Z axis (third column, negated)
        CameraPosition {
            x: -self.data[8],
            y: -self.data[9],
            z: -self.data[10],
        }
    }
}
