use crate::camera::{CameraMatrix, CameraPosition};
use crate::input::{MovementInput, MouseHandler, get_speed_delta};
use crate::process::ProcessHandle;

pub struct CameraController {
    move_speed: f32,
    mouse_handler: MouseHandler,
    last_position: Option<CameraPosition>,
    min_speed: f32,
    max_speed: f32,
    speed_step: f32,
    yaw: f32,   // Rotation around Y-axis (left/right)
    pitch: f32, // Rotation around X-axis (up/down)
    movement_input: MovementInput,
}

impl CameraController {
    pub fn new(move_speed: f32, mouse_sensitivity: f32) -> Self {
        Self {
            move_speed,
            mouse_handler: MouseHandler::new(mouse_sensitivity),
            last_position: None,
            min_speed: 0.1,
            max_speed: 100.0,
            speed_step: 0.5,
            yaw: 0.0,
            pitch: 0.0,
            movement_input: MovementInput::new(),
        }
    }
    
    pub fn increase_speed(&mut self) {
        self.move_speed = (self.move_speed + self.speed_step).min(self.max_speed);
    }
    
    pub fn decrease_speed(&mut self) {
        self.move_speed = (self.move_speed - self.speed_step).max(self.min_speed);
    }
    
    pub fn get_speed(&self) -> f32 {
        self.move_speed
    }
    
    pub fn enable_mouse(&mut self) {
        self.mouse_handler.enable();
    }
    
    pub fn disable_mouse(&mut self) {
        self.mouse_handler.disable();
    }
    
    pub fn is_mouse_enabled(&self) -> bool {
        self.mouse_handler.is_enabled()
    }
    
    fn reconstruct_camera_matrix(&self, camera_matrix: &mut CameraMatrix) {
        // Create rotation matrix from yaw and pitch
        let cos_yaw = self.yaw.cos();
        let sin_yaw = self.yaw.sin();
        let cos_pitch = self.pitch.cos();
        let sin_pitch = self.pitch.sin();
        
        // Calculate forward vector
        let forward_x = cos_pitch * cos_yaw;
        let forward_y = sin_pitch;
        let forward_z = cos_pitch * sin_yaw;
        
        // Calculate right vector (cross product of world up and forward)
        let right_x = -sin_yaw;
        let right_y = 0.0;
        let right_z = cos_yaw;
        
        // Calculate up vector (cross product of forward and right)
        let up_x = -sin_pitch * cos_yaw;
        let up_y = cos_pitch;
        let up_z = -sin_pitch * sin_yaw;
        
        // Set the rotation part of the matrix (preserve position)
        camera_matrix.data[0] = right_x;
        camera_matrix.data[1] = right_y;
        camera_matrix.data[2] = right_z;
        
        camera_matrix.data[4] = up_x;
        camera_matrix.data[5] = up_y;
        camera_matrix.data[6] = up_z;
        
        camera_matrix.data[8] = -forward_x;
        camera_matrix.data[9] = -forward_y;
        camera_matrix.data[10] = -forward_z;
        
        // Keep existing position (data[12], data[13], data[14])
        // Keep existing bottom row (data[3], data[7], data[11], data[15])
    }
    
    pub fn update_camera(&mut self, process: &ProcessHandle, base_addr: usize) -> Result<bool, String> {
        // Check for speed adjustment using Page Up/Down
        let speed_delta = get_speed_delta();
        if speed_delta > 0 {
            self.increase_speed();
        } else if speed_delta < 0 {
            self.decrease_speed();
        }
        
        // Get current camera matrix
        let mut camera_matrix = match process.get_camera_matrix(base_addr) {
            Ok(matrix) => matrix,
            Err(e) => return Err(format!("Failed to read camera matrix: {}", e)),
        };
        
        // Store the first position we read and initialize yaw/pitch from camera
        let current_pos = camera_matrix.get_position();
        if self.last_position.is_none() {
            self.last_position = Some(current_pos.clone());
            // Initialize yaw and pitch from current camera orientation
            let forward = camera_matrix.get_forward();
            self.yaw = forward.z.atan2(forward.x);
            self.pitch = (-forward.y).asin();
        }
        
        let mut moved = false;
        
        // Handle mouse movement for rotation
        if self.mouse_handler.is_enabled() {
            let (mouse_dx, mouse_dy) = self.mouse_handler.get_delta();
            
            if mouse_dx.abs() > 0.01 || mouse_dy.abs() > 0.01 {
                // Update yaw and pitch (inverted controls for natural feel)
                self.yaw += mouse_dx * 0.002; // Convert mouse delta to radians (inverted)
                self.pitch += mouse_dy * 0.002; // (inverted)
                
                // Clamp pitch to prevent camera flipping
                self.pitch = self.pitch.clamp(-std::f32::consts::FRAC_PI_2 * 0.99, 
                                              std::f32::consts::FRAC_PI_2 * 0.99);
                
                // Reconstruct camera matrix from yaw and pitch
                self.reconstruct_camera_matrix(&mut camera_matrix);
                moved = true;
            }
        }
        
        // Read movement input
        self.movement_input.read_input();
        
        // Apply movement if any keys were pressed
        if self.movement_input.has_movement() {
            let (dx, dy, dz) = self.movement_input.get_movement_vector(self.move_speed);
            camera_matrix.apply_translation(dx, dy, dz);
            moved = true;
        }
        
        // Update camera matrix if anything changed
        if moved {
            match process.set_camera_matrix(base_addr, &camera_matrix) {
                Ok(_) => {
                    let new_pos = camera_matrix.get_position();
                    self.last_position = Some(new_pos);
                    return Ok(true);
                }
                Err(e) => return Err(format!("Failed to set camera matrix: {}", e)),
            }
        }
        
        Ok(false)
    }
}

// Basic camera controller (fallback for position-only mode)
pub struct BasicCameraController {
    move_speed: f32,
    last_position: Option<CameraPosition>,
    min_speed: f32,
    max_speed: f32,
    speed_step: f32,
    movement_input: MovementInput,
}

impl BasicCameraController {
    pub fn new(move_speed: f32) -> Self {
        Self {
            move_speed,
            last_position: None,
            min_speed: 0.1,
            max_speed: 100.0,
            speed_step: 1.0,
            movement_input: MovementInput::new(),
        }
    }
    
    pub fn increase_speed(&mut self) {
        self.move_speed = (self.move_speed + self.speed_step).min(self.max_speed);
    }
    
    pub fn decrease_speed(&mut self) {
        self.move_speed = (self.move_speed - self.speed_step).max(self.min_speed);
    }
    
    pub fn get_speed(&self) -> f32 {
        self.move_speed
    }
    
    pub fn update_camera(&mut self, process: &ProcessHandle, base_addr: usize) -> Result<bool, String> {
        // Check for speed adjustment using Page Up/Down
        let speed_delta = get_speed_delta();
        if speed_delta > 0 {
            self.increase_speed();
        } else if speed_delta < 0 {
            self.decrease_speed();
        }
        
        // Get current camera position
        let current_pos = match process.get_camera_position(base_addr) {
            Ok(pos) => pos,
            Err(e) => return Err(format!("Failed to read camera position: {}", e)),
        };
        
        // Store the first position we read
        if self.last_position.is_none() {
            self.last_position = Some(current_pos.clone());
        }
        
        let mut new_pos = current_pos.clone();
        
        // Read movement input
        self.movement_input.read_input();
        
        // Apply movement if any keys were pressed
        if self.movement_input.has_movement() {
            // For basic controller, apply movement directly to world coordinates
            if self.movement_input.forward {
                new_pos.z += self.move_speed;
            }
            if self.movement_input.backward {
                new_pos.z -= self.move_speed;
            }
            if self.movement_input.left {
                new_pos.x -= self.move_speed; // J key moves left (negative X)
            }
            if self.movement_input.right {
                new_pos.x += self.move_speed; // L key moves right (positive X)
            }
            if self.movement_input.up {
                new_pos.y += self.move_speed;
            }
            if self.movement_input.down {
                new_pos.y -= self.move_speed;
            }
            
            match process.set_camera_position(base_addr, &new_pos) {
                Ok(_) => {
                    self.last_position = Some(new_pos.clone());
                    return Ok(true);
                }
                Err(e) => return Err(format!("Failed to set camera position: {}", e)),
            }
        }
        
        Ok(false)
    }
}
