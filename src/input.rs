use winapi::um::winuser::{GetAsyncKeyState, GetCursorPos, SetCursorPos, GetSystemMetrics, SM_CXSCREEN, SM_CYSCREEN};
use winapi::shared::windef::POINT;

// Virtual key codes for movement keys
pub const VK_I: i32 = 0x49; // I key
pub const VK_J: i32 = 0x4A; // J key  
pub const VK_K: i32 = 0x4B; // K key
pub const VK_L: i32 = 0x4C; // L key
pub const VK_U: i32 = 0x55; // U key (up)
pub const VK_O: i32 = 0x4F; // O key (down)
pub const VK_M: i32 = 0x4D; // M key (toggle mouse)
pub const VK_P: i32 = 0x50; // P key (toggle patch)

pub fn is_key_pressed(vk_code: i32) -> bool {
    unsafe {
        (GetAsyncKeyState(vk_code) & 0x8000u16 as i16) != 0
    }
}

// Speed control using Page Up/Down
pub fn get_speed_delta() -> i32 {
    const VK_PRIOR: i32 = 0x21; // Page Up
    const VK_NEXT: i32 = 0x22;  // Page Down
    
    if is_key_pressed(VK_PRIOR) {
        return 1; // Increase speed
    } else if is_key_pressed(VK_NEXT) {
        return -1; // Decrease speed
    }
    
    0
}

pub struct MouseHandler {
    screen_center_x: i32,
    screen_center_y: i32,
    sensitivity: f32,
    enabled: bool,
}

impl MouseHandler {
    pub fn new(sensitivity: f32) -> Self {
        let screen_center_x = unsafe { GetSystemMetrics(SM_CXSCREEN) / 2 };
        let screen_center_y = unsafe { GetSystemMetrics(SM_CYSCREEN) / 2 };
        
        Self {
            screen_center_x,
            screen_center_y,
            sensitivity,
            enabled: false,
        }
    }
    
    pub fn enable(&mut self) {
        self.enabled = true;
        // Center the cursor initially
        unsafe {
            SetCursorPos(self.screen_center_x, self.screen_center_y);
        }
    }
    
    pub fn disable(&mut self) {
        self.enabled = false;
    }
    
    pub fn is_enabled(&self) -> bool {
        self.enabled
    }
    
    pub fn get_delta(&self) -> (f32, f32) {
        if !self.enabled {
            return (0.0, 0.0);
        }
        
        let mut cursor_pos = POINT { x: 0, y: 0 };
        unsafe {
            if GetCursorPos(&mut cursor_pos) == 0 {
                return (0.0, 0.0);
            }
        }
        
        let delta_x = (cursor_pos.x - self.screen_center_x) as f32;
        let delta_y = (cursor_pos.y - self.screen_center_y) as f32;
        
        // Only re-center if there's significant movement
        if delta_x.abs() > 1.0 || delta_y.abs() > 1.0 {
            unsafe {
                SetCursorPos(self.screen_center_x, self.screen_center_y);
            }
        }
        
        (delta_x * self.sensitivity, delta_y * self.sensitivity)
    }
}

#[derive(Debug)]
pub struct MovementInput {
    pub forward: bool,
    pub backward: bool,
    pub left: bool,
    pub right: bool,
    pub up: bool,
    pub down: bool,
}

impl MovementInput {
    pub fn new() -> Self {
        Self {
            forward: false,
            backward: false,
            left: false,
            right: false,
            up: false,
            down: false,
        }
    }
    
    pub fn read_input(&mut self) {
        self.forward = is_key_pressed(VK_I);
        self.backward = is_key_pressed(VK_K);
        // Fixed J/L mapping: J should move left, L should move right
        self.left = is_key_pressed(VK_J);
        self.right = is_key_pressed(VK_L);
        self.up = is_key_pressed(VK_U);
        self.down = is_key_pressed(VK_O);
    }
    
    pub fn has_movement(&self) -> bool {
        self.forward || self.backward || self.left || self.right || self.up || self.down
    }
    
    pub fn get_movement_vector(&self, speed: f32) -> (f32, f32, f32) {
        let mut dx = 0.0;
        let mut dy = 0.0;
        let mut dz = 0.0;
        
        if self.forward {
            dz += speed;
        }
        if self.backward {
            dz -= speed;
        }
        if self.left {
            dx += speed; // J key moves left (positive X in this game)
        }
        if self.right {
            dx -= speed; // L key moves right (negative X in this game)
        }
        if self.up {
            dy += speed;
        }
        if self.down {
            dy -= speed;
        }
        
        (dx, dy, dz)
    }
}
