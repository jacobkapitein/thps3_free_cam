use std::mem;
use std::ptr;
use winapi::shared::minwindef::{DWORD, FALSE, HMODULE};
use winapi::um::errhandlingapi::GetLastError;
use winapi::um::handleapi::CloseHandle;
use winapi::um::memoryapi::{ReadProcessMemory, WriteProcessMemory, VirtualProtectEx};
use winapi::um::processthreadsapi::OpenProcess;
use winapi::um::psapi::EnumProcessModules;
use winapi::um::tlhelp32::{
    CreateToolhelp32Snapshot, Process32First, Process32Next, PROCESSENTRY32, TH32CS_SNAPPROCESS,
};
use winapi::um::winnt::{HANDLE, PROCESS_VM_READ, PROCESS_VM_WRITE, PROCESS_VM_OPERATION, PROCESS_QUERY_INFORMATION, PAGE_EXECUTE_READWRITE};

use crate::camera::{CameraMatrix, CameraPosition};

#[derive(Debug, Clone)]
pub struct CodePatch {
    pub address: usize,
    pub original_bytes: Vec<u8>,
    pub is_applied: bool,
}

pub struct ProcessHandle {
    handle: HANDLE,
    #[allow(dead_code)]
    pid: DWORD,
}

impl ProcessHandle {
    pub fn new(process_name: &str) -> Result<Self, String> {
        let pid = find_process_by_name(process_name)?;
        println!("Found {} with PID: {}", process_name, pid);
        
        let handle = unsafe { 
            OpenProcess(
                PROCESS_VM_READ | PROCESS_VM_WRITE | PROCESS_VM_OPERATION | PROCESS_QUERY_INFORMATION, 
                FALSE, 
                pid
            ) 
        };
        if handle.is_null() {
            let error_code = unsafe { GetLastError() };
            return Err(format!("Failed to open process with PID: {} (Error code: {})", pid, error_code));
        }
        
        println!("Successfully opened process handle!");
        Ok(ProcessHandle { handle, pid })
    }
    
    pub fn read_memory<T>(&self, address: usize) -> Result<T, String> {
        let mut buffer: T = unsafe { mem::zeroed() };
        let mut bytes_read = 0;
        
        let result = unsafe {
            ReadProcessMemory(
                self.handle,
                address as *const _,
                &mut buffer as *mut _ as *mut _,
                mem::size_of::<T>(),
                &mut bytes_read,
            )
        };
        
        if result == 0 {
            let error_code = unsafe { GetLastError() };
            return Err(format!("Failed to read process memory at 0x{:X} (Error: {})", address, error_code));
        }
        
        Ok(buffer)
    }
    
    pub fn write_memory<T>(&self, address: usize, value: &T) -> Result<(), String> {
        let mut bytes_written = 0;
        
        let result = unsafe {
            WriteProcessMemory(
                self.handle,
                address as *mut _,
                value as *const _ as *const _,
                mem::size_of::<T>(),
                &mut bytes_written,
            )
        };
        
        if result == 0 {
            return Err("Failed to write process memory".to_string());
        }
        
        Ok(())
    }
    
    pub fn get_base_address(&self) -> Result<usize, String> {
        let mut modules: [HMODULE; 1024] = [ptr::null_mut(); 1024];
        let mut bytes_needed = 0;
        
        let result = unsafe {
            EnumProcessModules(
                self.handle,
                modules.as_mut_ptr(),
                mem::size_of_val(&modules) as u32,
                &mut bytes_needed,
            )
        };
        
        if result == 0 {
            return Err("Failed to enumerate process modules".to_string());
        }
        
        if bytes_needed > 0 {
            let base_address = modules[0] as usize;
            Ok(base_address)
        } else {
            Err("No modules found".to_string())
        }
    }
    
    pub fn resolve_pointer_chain(&self, base_address: usize, offsets: &[usize]) -> Result<usize, String> {
        let mut current_address = base_address;
        
        // Follow the pointer chain
        for (i, &offset) in offsets.iter().enumerate() {
            if i == offsets.len() - 1 {
                // Last offset - just add it to get final address
                current_address += offset;
            } else {
                // First read the pointer value (32-bit), then add the offset
                match self.read_memory::<u32>(current_address) {
                    Ok(ptr_value) => {
                        current_address = ptr_value as usize;
                        
                        // Check if the pointer is valid (not null and within reasonable range)
                        if current_address == 0 {
                            return Err(format!("Null pointer encountered at step {}", i));
                        }
                        if current_address < 0x10000 || current_address > 0x7FFFFFFF {
                            return Err(format!("Invalid pointer value 0x{:X} at step {}", current_address, i));
                        }
                        
                        // Now add the offset
                        current_address += offset;
                    }
                    Err(e) => {
                        return Err(format!("Failed to read pointer at step {}: {}", i, e));
                    }
                }
            }
        }
        
        Ok(current_address)
    }
    
    pub fn patch_with_nops(&self, address: usize, length: usize) -> Result<CodePatch, String> {
        // First, read the original bytes
        let mut original_bytes = vec![0u8; length];
        let mut bytes_read = 0;
        
        let read_result = unsafe {
            ReadProcessMemory(
                self.handle,
                address as *const _,
                original_bytes.as_mut_ptr() as *mut _,
                length,
                &mut bytes_read,
            )
        };
        
        if read_result == 0 {
            let error_code = unsafe { GetLastError() };
            return Err(format!("Failed to read original bytes at 0x{:X} (Error: {})", address, error_code));
        }
        
        // Change memory protection to allow execution/writing
        let mut old_protect = 0;
        let protect_result = unsafe {
            VirtualProtectEx(
                self.handle,
                address as *mut _,
                length,
                PAGE_EXECUTE_READWRITE,
                &mut old_protect,
            )
        };
        
        if protect_result == 0 {
            let error_code = unsafe { GetLastError() };
            return Err(format!("Failed to change memory protection at 0x{:X} (Error: {})", address, error_code));
        }
        
        // Create NOP bytes (0x90)
        let nop_bytes = vec![0x90u8; length];
        
        // Write NOP bytes
        let mut bytes_written = 0;
        let write_result = unsafe {
            WriteProcessMemory(
                self.handle,
                address as *mut _,
                nop_bytes.as_ptr() as *const _,
                length,
                &mut bytes_written,
            )
        };
        
        if write_result == 0 {
            // Restore original protection
            unsafe {
                VirtualProtectEx(
                    self.handle,
                    address as *mut _,
                    length,
                    old_protect,
                    &mut old_protect,
                );
            }
            return Err("Failed to write NOP bytes".to_string());
        }
        
        // Restore original protection
        unsafe {
            VirtualProtectEx(
                self.handle,
                address as *mut _,
                length,
                old_protect,
                &mut old_protect,
            );
        }
        
        Ok(CodePatch {
            address,
            original_bytes,
            is_applied: true,
        })
    }
    
    pub fn restore_patch(&self, patch: &mut CodePatch) -> Result<(), String> {
        if !patch.is_applied {
            return Err("Patch is not currently applied".to_string());
        }
        
        let length = patch.original_bytes.len();
        
        // Change memory protection
        let mut old_protect = 0;
        let protect_result = unsafe {
            VirtualProtectEx(
                self.handle,
                patch.address as *mut _,
                length,
                PAGE_EXECUTE_READWRITE,
                &mut old_protect,
            )
        };
        
        if protect_result == 0 {
            let error_code = unsafe { GetLastError() };
            return Err(format!("Failed to change memory protection at 0x{:X} (Error: {})", patch.address, error_code));
        }
        
        // Write original bytes back
        let mut bytes_written = 0;
        let write_result = unsafe {
            WriteProcessMemory(
                self.handle,
                patch.address as *mut _,
                patch.original_bytes.as_ptr() as *const _,
                length,
                &mut bytes_written,
            )
        };
        
        if write_result == 0 {
            // Restore original protection
            unsafe {
                VirtualProtectEx(
                    self.handle,
                    patch.address as *mut _,
                    length,
                    old_protect,
                    &mut old_protect,
                );
            }
            return Err("Failed to restore original bytes".to_string());
        }
        
        // Restore original protection
        unsafe {
            VirtualProtectEx(
                self.handle,
                patch.address as *mut _,
                length,
                old_protect,
                &mut old_protect,
            );
        }
        
        patch.is_applied = false;
        Ok(())
    }
    
    pub fn get_camera_write_patch_address(&self, base_address: usize) -> Result<usize, String> {
        // Address of the "repe movsd" instruction that copies camera data
        // Found via Cheat Engine disassembler: Skate3.exe.text+16B2E4
        // This instruction overwrites our camera changes, so we NOP it out
        
        // The offset 0x16B2E4 is from the .text section, which typically starts at base + 0x1000
        // But let's try different approaches to find the right address
        let text_section_offset = 0x1000; // Typical .text section offset
        let instruction_offset = 0x16B2E4;
        
        // Try multiple address calculations
        let addresses_to_try = vec![
            base_address + instruction_offset,                    // Direct offset from base
            base_address + text_section_offset + instruction_offset, // Base + text section + offset
            base_address + instruction_offset - text_section_offset, // Adjust for text section
        ];
        
        for &addr in addresses_to_try.iter() {
            // Try to read 2 bytes from this address to see if it contains the expected instruction
            let mut test_bytes = vec![0u8; 2];
            let mut bytes_read = 0;
            
            let read_result = unsafe {
                ReadProcessMemory(
                    self.handle,
                    addr as *const _,
                    test_bytes.as_mut_ptr() as *mut _,
                    2,
                    &mut bytes_read,
                )
            };
            
            if read_result != 0 && bytes_read == 2 {
                // Check if this matches the expected "repe movsd" instruction (F3 A5)
                if test_bytes[0] == 0xF3 && test_bytes[1] == 0xA5 {
                    return Ok(addr);
                }
            }
        }
        
        // If none of the standard calculations work, return the first one
        Ok(addresses_to_try[0])
    }
    
    pub fn get_camera_position(&self, base_address: usize) -> Result<CameraPosition, String> {
        // Camera pointer chain: "Skate3.exe"+004E1E78+34C+8+4+8C+0+324/328/32C
        let base_offset = 0x004E1E78;
        let offsets = vec![0x34C, 0x8, 0x4, 0x8C, 0x0];
        
        // Get X position (final offset: 0x324)
        let mut x_offsets = offsets.clone();
        x_offsets.push(0x324);
        let x_addr = self.resolve_pointer_chain(base_address + base_offset, &x_offsets)?;
        let x: f32 = self.read_memory(x_addr)?;
        
        // Get Y position (final offset: 0x328)
        let mut y_offsets = offsets.clone();
        y_offsets.push(0x328);
        let y_addr = self.resolve_pointer_chain(base_address + base_offset, &y_offsets)?;
        let y: f32 = self.read_memory(y_addr)?;
        
        // Get Z position (final offset: 0x32C)
        let mut z_offsets = offsets.clone();
        z_offsets.push(0x32C);
        let z_addr = self.resolve_pointer_chain(base_address + base_offset, &z_offsets)?;
        let z: f32 = self.read_memory(z_addr)?;
        
        Ok(CameraPosition { x, y, z })
    }
    
    pub fn get_camera_matrix(&self, base_address: usize) -> Result<CameraMatrix, String> {
        // Camera pointer chain: "Skate3.exe"+004E1E78+34C+8+4+8C+0+2F4 (start of 4x4 matrix)
        // Matrix starts at 0x2F4, positions are at 0x324/0x328/0x32C (which is matrix[12]/[13]/[14])
        // 0x324 - 0x2F4 = 0x30 = 48 bytes = 12 floats (indices 12/13/14)
        let base_offset = 0x004E1E78;
        let offsets = vec![0x34C, 0x8, 0x4, 0x8C, 0x0, 0x2F4];
        
        let matrix_addr = self.resolve_pointer_chain(base_address + base_offset, &offsets)?;
        
        // Read the full 4x4 matrix (16 floats)
        let mut data = [0.0f32; 16];
        for i in 0..16 {
            data[i] = self.read_memory::<f32>(matrix_addr + i * 4)?;
        }
        
        Ok(CameraMatrix { data })
    }
    
    pub fn set_camera_position(&self, base_address: usize, position: &CameraPosition) -> Result<(), String> {
        // Camera pointer chain: "Skate3.exe"+004E1E78+34C+8+4+8C+0+324/328/32C
        let base_offset = 0x004E1E78;
        let offsets = vec![0x34C, 0x8, 0x4, 0x8C, 0x0];
        
        // Set X position (final offset: 0x324)
        let mut x_offsets = offsets.clone();
        x_offsets.push(0x324);
        let x_addr = self.resolve_pointer_chain(base_address + base_offset, &x_offsets)?;
        self.write_memory(x_addr, &position.x)?;
        
        // Set Y position (final offset: 0x328)
        let mut y_offsets = offsets.clone();
        y_offsets.push(0x328);
        let y_addr = self.resolve_pointer_chain(base_address + base_offset, &y_offsets)?;
        self.write_memory(y_addr, &position.y)?;
        
        // Set Z position (final offset: 0x32C)
        let mut z_offsets = offsets.clone();
        z_offsets.push(0x32C);
        let z_addr = self.resolve_pointer_chain(base_address + base_offset, &z_offsets)?;
        self.write_memory(z_addr, &position.z)?;
        
        Ok(())
    }
    
    pub fn set_camera_matrix(&self, base_address: usize, matrix: &CameraMatrix) -> Result<(), String> {
        // Camera pointer chain: "Skate3.exe"+004E1E78+34C+8+4+8C+0+2F4 (start of 4x4 matrix)
        let base_offset = 0x004E1E78;
        let offsets = vec![0x34C, 0x8, 0x4, 0x8C, 0x0, 0x2F4];
        
        let matrix_addr = self.resolve_pointer_chain(base_address + base_offset, &offsets)?;
        
        // Write the full 4x4 matrix (16 floats)
        for i in 0..16 {
            self.write_memory(matrix_addr + i * 4, &matrix.data[i])?;
        }
        
        Ok(())
    }
    
    pub fn get_camera_addresses(&self, base_address: usize) -> Result<(usize, usize, usize), String> {
        // Camera pointer chain: "Skate3.exe"+004E1E78+34C+8+4+8C+0+324/328/32C
        let base_offset = 0x004E1E78;
        let offsets = vec![0x34C, 0x8, 0x4, 0x8C, 0x0];
        
        // Get addresses for X, Y, Z
        let mut x_offsets = offsets.clone();
        x_offsets.push(0x324);
        let x_addr = self.resolve_pointer_chain(base_address + base_offset, &x_offsets)?;
        
        let mut y_offsets = offsets.clone();
        y_offsets.push(0x328);
        let y_addr = self.resolve_pointer_chain(base_address + base_offset, &y_offsets)?;
        
        let mut z_offsets = offsets.clone();
        z_offsets.push(0x32C);
        let z_addr = self.resolve_pointer_chain(base_address + base_offset, &z_offsets)?;
        
        Ok((x_addr, y_addr, z_addr))
    }

    // ...existing code...
}

impl Drop for ProcessHandle {
    fn drop(&mut self) {
        unsafe {
            CloseHandle(self.handle);
        }
    }
}

pub fn find_process_by_name(process_name: &str) -> Result<DWORD, String> {
    let snapshot = unsafe { CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0) };
    if snapshot.is_null() {
        return Err("Failed to create process snapshot".to_string());
    }
    
    let mut process_entry: PROCESSENTRY32 = unsafe { mem::zeroed() };
    process_entry.dwSize = mem::size_of::<PROCESSENTRY32>() as u32;
    
    let mut result = unsafe { Process32First(snapshot, &mut process_entry) };
    
    while result != 0 {
        let current_name = unsafe {
            let raw_name = process_entry.szExeFile.as_ptr() as *const u8;
            let mut name_bytes = Vec::new();
            let mut i = 0;
            while i < process_entry.szExeFile.len() {
                let byte = *raw_name.add(i);
                if byte == 0 {
                    break;
                }
                name_bytes.push(byte);
                i += 1;
            }
            String::from_utf8_lossy(&name_bytes).into_owned()
        };
        
        if current_name.to_lowercase().contains(&process_name.to_lowercase()) {
            unsafe { CloseHandle(snapshot) };
            return Ok(process_entry.th32ProcessID);
        }
        
        result = unsafe { Process32Next(snapshot, &mut process_entry) };
    }
    
    unsafe { CloseHandle(snapshot) };
    Err(format!("Process '{}' not found", process_name))
}

pub fn list_all_processes() -> Result<(), String> {
    let snapshot = unsafe { CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0) };
    if snapshot.is_null() {
        return Err("Failed to create process snapshot".to_string());
    }
    
    let mut process_entry: PROCESSENTRY32 = unsafe { mem::zeroed() };
    process_entry.dwSize = mem::size_of::<PROCESSENTRY32>() as u32;
    
    let mut result = unsafe { Process32First(snapshot, &mut process_entry) };
    
    println!("All running processes:");
    println!("=====================");
    
    while result != 0 {
        let current_name = unsafe {
            let raw_name = process_entry.szExeFile.as_ptr() as *const u8;
            let mut name_bytes = Vec::new();
            let mut i = 0;
            while i < process_entry.szExeFile.len() {
                let byte = *raw_name.add(i);
                if byte == 0 {
                    break;
                }
                name_bytes.push(byte);
                i += 1;
            }
            String::from_utf8_lossy(&name_bytes).into_owned()
        };
        
        if current_name.to_lowercase().contains("skate") {
            println!("🎮 {}: PID {}", current_name, process_entry.th32ProcessID);
        }
        
        result = unsafe { Process32Next(snapshot, &mut process_entry) };
    }
    
    unsafe { CloseHandle(snapshot) };
    Ok(())
}
