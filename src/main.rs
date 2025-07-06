mod camera;
mod controller;
mod input;
mod process;

use controller::{CameraController, BasicCameraController};
use input::{is_key_pressed, VK_M, VK_P};
use process::{ProcessHandle, CodePatch, list_all_processes};
use winapi::um::winuser::GetAsyncKeyState;

fn main() {
    println!("THPS3 Free Cam Tool");
    println!("===================");
    
    // First, let's see what processes are running
    println!("🔍 Scanning for Tony Hawk Pro Skater 3 process...");
    if let Err(e) = list_all_processes() {
        println!("❌ Failed to list processes: {}", e);
    }
    
    // Try to find and attach to Skate3 process
    let process_names = vec!["skate3.exe", "Skate3.exe", "SKATE3.EXE"];
    let mut process_handle = None;
    
    for name in process_names {
        match ProcessHandle::new(name) {
            Ok(handle) => {
                process_handle = Some(handle);
                break;
            }
            Err(e) => {
                println!("Could not find process '{}': {}", name, e);
            }
        }
    }
    
    let process = match process_handle {
        Some(p) => p,
        None => {
            println!("❌ Could not attach to THPS3 process!");
            println!("This is likely due to insufficient privileges.");
            println!("💡 Try running this program as Administrator:");
            println!("   1. Right-click on PowerShell/Command Prompt");
            println!("   2. Select 'Run as administrator'");
            println!("   3. Navigate to the project folder and run: cargo run");
            println!("   4. Make sure THPS3 is running before starting this tool");
            println!("\nPress Enter to exit...");
            let mut input = String::new();
            std::io::stdin().read_line(&mut input).unwrap();
            return;
        }
    };
    
    println!("✅ Successfully attached to THPS3!");
    
    // Get the base address of the process
    match process.get_base_address() {
        Ok(base_addr) => {
            println!("📍 Base address: 0x{:X}", base_addr);
            
            // Test camera position reading
            println!("\n🎮 Testing camera position access...");
            match process.get_camera_position(base_addr) {
                Ok(cam_pos) => {
                    println!("✅ Successfully read camera position!");
                    println!("   📍 Camera X: {:.6}", cam_pos.x);
                    println!("   📍 Camera Y: {:.6}", cam_pos.y);
                    println!("   📍 Camera Z: {:.6}", cam_pos.z);
                    
                    // Test camera matrix reading
                    println!("\n🎮 Testing camera matrix access...");
                    match process.get_camera_matrix(base_addr) {
                        Ok(cam_matrix) => {
                            println!("✅ Successfully read camera matrix!");
                            let matrix_pos = cam_matrix.get_position();
                            println!("   📍 Matrix Position X: {:.6}", matrix_pos.x);
                            println!("   📍 Matrix Position Y: {:.6}", matrix_pos.y);
                            println!("   📍 Matrix Position Z: {:.6}", matrix_pos.z);
                            
                            // Show the actual memory addresses
                            match process.get_camera_addresses(base_addr) {
                                Ok((x_addr, y_addr, z_addr)) => {
                                    println!("\n🔍 Memory addresses:");
                                    println!("   X: 0x{:X}", x_addr);
                                    println!("   Y: 0x{:X}", y_addr);
                                    println!("   Z: 0x{:X}", z_addr);
                                }
                                Err(e) => println!("❌ Failed to get camera addresses: {}", e),
                            }
                            
                            // Start real-time camera control
                            println!("\n🎮 Starting Free Camera Mode!");
                            println!("===============================");
                            println!("Controls:");
                            println!("   I/K - Move Forward/Backward");
                            println!("   J/L - Move Left/Right");
                            println!("   U/O - Move Up/Down");
                            println!("   M   - Toggle Mouse Look");
                            println!("   P   - Toggle Camera Write Patch");
                            println!("   Page Up/Down - Increase/Decrease Speed");
                            println!("");
                            println!("💡 Switch to Skate3 window and use the controls!");
                            println!("   Camera will respond to key presses in real-time.");
                            println!("   Close this terminal window to stop the program.");
                            println!("");
                            
                            let mut controller = CameraController::new(5.0, 0.5); // Move speed: 5 units per press, mouse sensitivity: 0.1 (perfect responsiveness)
                            let mut last_pos_display = cam_pos.clone();
                            let mut mouse_toggle_pressed = false;
                            let mut patch_toggle_pressed = false;
                            let mut camera_patch: Option<CodePatch> = None;
                            
                            loop {
                                // Check for mouse toggle
                                if is_key_pressed(VK_M) {
                                    if !mouse_toggle_pressed {
                                        if controller.is_mouse_enabled() {
                                            controller.disable_mouse();
                                            println!("\n🖱️ Mouse look disabled");
                                        } else {
                                            controller.enable_mouse();
                                            println!("\n🖱️ Mouse look enabled - move mouse to look around");
                                        }
                                        mouse_toggle_pressed = true;
                                    }
                                } else {
                                    mouse_toggle_pressed = false;
                                }
                                
                                // Check for patch toggle
                                let p_key_state = unsafe { GetAsyncKeyState(VK_P) };
                                let p_pressed = (p_key_state & 0x8000u16 as i16) != 0;
                                let p_just_pressed = (p_key_state & 0x0001u16 as i16) != 0;
                                
                                if p_pressed || p_just_pressed {
                                    if !patch_toggle_pressed {
                                        match &mut camera_patch {
                                            Some(patch) => {
                                                if patch.is_applied {
                                                    match process.restore_patch(patch) {
                                                        Ok(_) => println!("\n🔧 Camera patch disabled - game will overwrite camera"),
                                                        Err(e) => println!("\n❌ Failed to disable patch: {}", e),
                                                    }
                                                } else {
                                                    // Re-apply the patch
                                                    match process.get_camera_write_patch_address(base_addr) {
                                                        Ok(patch_addr) => {
                                                            match process.patch_with_nops(patch_addr, 2) {
                                                                Ok(new_patch) => {
                                                                    *patch = new_patch;
                                                                    println!("\n🔧 Camera patch re-enabled - free camera active!");
                                                                }
                                                                Err(e) => println!("\n❌ Failed to re-apply patch: {}", e),
                                                            }
                                                        }
                                                        Err(e) => println!("\n❌ Failed to get patch address: {}", e),
                                                    }
                                                }
                                            }
                                            None => {
                                                // First time applying patch
                                                match process.get_camera_write_patch_address(base_addr) {
                                                    Ok(patch_addr) => {
                                                        match process.patch_with_nops(patch_addr, 2) {
                                                            Ok(patch) => {
                                                                camera_patch = Some(patch);
                                                                println!("\n🔧 Camera patch enabled - free camera active!");
                                                            }
                                                            Err(e) => println!("\n❌ Failed to apply patch: {}", e),
                                                        }
                                                    }
                                                    Err(e) => println!("\n❌ Failed to get patch address: {}", e),
                                                }
                                            }
                                        }
                                        patch_toggle_pressed = true;
                                    }
                                } else {
                                    patch_toggle_pressed = false;
                                }
                                
                                // Update camera based on input
                                match controller.update_camera(&process, base_addr) {
                                    Ok(moved) => {
                                        if moved {
                                            // Get and display current position
                                            if let Ok(current_pos) = process.get_camera_position(base_addr) {
                                                // Only print if position changed significantly
                                                let dx = (current_pos.x - last_pos_display.x).abs();
                                                let dy = (current_pos.y - last_pos_display.y).abs();
                                                let dz = (current_pos.z - last_pos_display.z).abs();
                                                
                                                if dx > 0.1 || dy > 0.1 || dz > 0.1 {
                                                    let mouse_status = if controller.is_mouse_enabled() { "🖱️ ON" } else { "🖱️ OFF" };
                                                    print!("\r📍 Camera: X:{:.1}, Y:{:.1}, Z:{:.1} | Mouse: {} | Speed: {:.1}   ", 
                                                           current_pos.x, current_pos.y, current_pos.z, mouse_status, controller.get_speed());
                                                    std::io::Write::flush(&mut std::io::stdout()).unwrap();
                                                    last_pos_display = current_pos;
                                                }
                                            }
                                        }
                                    }
                                    Err(e) => {
                                        println!("\n❌ Camera control error: {}", e);
                                        println!("This might happen if you're not in-game or the game state changed.");
                                        break;
                                    }
                                }
                                
                                // Small delay to prevent excessive CPU usage
                                std::thread::sleep(std::time::Duration::from_millis(16)); // ~60 FPS
                            }
                        }
                        Err(e) => {
                            println!("❌ Failed to read camera matrix: {}", e);
                            // This might mean the matrix pointer chain is incorrect.
                            
                            // Fall back to position-only mode
                            println!("\n📍 Falling back to position-only mode...");
                            
                            // Show the actual memory addresses
                            match process.get_camera_addresses(base_addr) {
                                Ok((x_addr, y_addr, z_addr)) => {
                                    println!("\n🔍 Memory addresses:");
                                    println!("   X: 0x{:X}", x_addr);
                                    println!("   Y: 0x{:X}", y_addr);
                                    println!("   Z: 0x{:X}", z_addr);
                                }
                                Err(e) => println!("❌ Failed to get camera addresses: {}", e),
                            }
                            
                            // Start basic camera control (position-only)
                            println!("\n🎮 Starting Basic Camera Mode!");
                            println!("===============================");
                            println!("Controls:");
                            println!("   I/K - Move Forward/Backward");
                            println!("   J/L - Move Left/Right");
                            println!("   U/O - Move Up/Down");
                            println!("   P   - Toggle Camera Write Patch");
                            println!("   Page Up/Down - Increase/Decrease Speed");
                            println!("");
                            println!("💡 Switch to Skate3 window and use the controls!");
                            println!("   Camera will respond to key presses in real-time.");
                            println!("   Close this terminal window to stop the program.");
                            println!("");
                            
                            let mut basic_controller = BasicCameraController::new(10.0); // Move speed: 10 units per press
                            let mut last_pos_display = cam_pos.clone();
                            let mut patch_toggle_pressed = false;
                            let mut camera_patch: Option<CodePatch> = None;
                            
                            loop {
                                // Check for patch toggle
                                let p_key_state = unsafe { GetAsyncKeyState(VK_P) };
                                let p_pressed = (p_key_state & 0x8000u16 as i16) != 0;
                                let p_just_pressed = (p_key_state & 0x0001u16 as i16) != 0;
                                
                                if p_pressed || p_just_pressed {
                                    if !patch_toggle_pressed {
                                        match &mut camera_patch {
                                            Some(patch) => {
                                                if patch.is_applied {
                                                    match process.restore_patch(patch) {
                                                        Ok(_) => println!("\n🔧 Camera patch disabled - game will overwrite camera"),
                                                        Err(e) => println!("\n❌ Failed to disable patch: {}", e),
                                                    }
                                                } else {
                                                    // Re-apply the patch
                                                    match process.get_camera_write_patch_address(base_addr) {
                                                        Ok(patch_addr) => {
                                                            match process.patch_with_nops(patch_addr, 2) {
                                                                Ok(new_patch) => {
                                                                    *patch = new_patch;
                                                                    println!("\n🔧 Camera patch re-enabled - free camera active!");
                                                                }
                                                                Err(e) => println!("\n❌ Failed to re-apply patch: {}", e),
                                                            }
                                                        }
                                                        Err(e) => println!("\n❌ Failed to get patch address: {}", e),
                                                    }
                                                }
                                            }
                                            None => {
                                                // First time applying patch
                                                match process.get_camera_write_patch_address(base_addr) {
                                                    Ok(patch_addr) => {
                                                        match process.patch_with_nops(patch_addr, 2) {
                                                            Ok(patch) => {
                                                                camera_patch = Some(patch);
                                                                println!("\n🔧 Camera patch enabled - free camera active!");
                                                                println!("   You can now move the camera without pausing the game!");
                                                            }
                                                            Err(e) => println!("\n❌ Failed to apply patch: {}", e),
                                                        }
                                                    }
                                                    Err(e) => println!("\n❌ Failed to get patch address: {}", e),
                                                }
                                            }
                                        }
                                        patch_toggle_pressed = true;
                                    }
                                } else {
                                    patch_toggle_pressed = false;
                                }
                                
                                // Update camera based on input
                                match basic_controller.update_camera(&process, base_addr) {
                                    Ok(moved) => {
                                        if moved {
                                            // Get and display current position
                                            if let Ok(current_pos) = process.get_camera_position(base_addr) {
                                                // Only print if position changed significantly
                                                let dx = (current_pos.x - last_pos_display.x).abs();
                                                let dy = (current_pos.y - last_pos_display.y).abs();
                                                let dz = (current_pos.z - last_pos_display.z).abs();
                                                
                                                if dx > 0.1 || dy > 0.1 || dz > 0.1 {
                                                    print!("\r📍 Camera: X:{:.1}, Y:{:.1}, Z:{:.1} | Speed: {:.1}   ", 
                                                           current_pos.x, current_pos.y, current_pos.z, basic_controller.get_speed());
                                                    std::io::Write::flush(&mut std::io::stdout()).unwrap();
                                                    last_pos_display = current_pos;
                                                }
                                            }
                                        }
                                    }
                                    Err(e) => {
                                        println!("\n❌ Camera control error: {}", e);
                                        println!("This might happen if you're not in-game or the game state changed.");
                                        break;
                                    }
                                }
                                
                                // Small delay to prevent excessive CPU usage
                                std::thread::sleep(std::time::Duration::from_millis(16)); // ~60 FPS
                            }
                        }
                    }
                }
                Err(e) => {
                    println!("❌ Failed to read camera position: {}", e);
                    println!("   This might mean the pointer chain is incorrect or the game state has changed.");
                }
            }
            
            println!("\n🎮 Camera system ready!");
        }
        Err(e) => {
            println!("❌ Failed to get base address: {}", e);
        }
    }
}
