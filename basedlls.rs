#![no_std]
#![no_main]
#![allow(nonstandard_style)]

// This uses PEB_LDR_DATA to list the modules and print to the console.
// See https://learn.microsoft.com/en-us/windows/win32/api/winternl/ns-winternl-peb_ldr_data

mod winapi;
use winapi::{
    IMAGE_DIRECTORY_ENTRY_EXPORT, IMAGE_EXPORT_DIRECTORY, IMAGE_NT_HEADERS64, LDR_DATA_TABLE_ENTRY,
    PEB,
};

use core::{arch, mem};

// Not exactly accurate but I'm lazy.
type WriteFileFn = extern "system" fn(isize, *const u8, u32, *mut u32, isize) -> i32;

/// # Safety
///
/// Both `a` and `b` must be null terminated.
unsafe fn eq_cstr(a: *const u8, b: *const u8) -> bool {
    unsafe {
        let mut i = 0;
        loop {
            if *a.add(i) != *b.add(i) {
                return false;
            } else if *a.add(i) == 0 {
                return true;
            }
            i += 1;
        }
    }
}

#[unsafe(no_mangle)]
extern "C" fn main() -> u32 {
    // Get the list
    unsafe {
        let peb: *mut PEB;
        arch::asm!(
            "mov {}, gs:[0x60]",
            out(reg) peb,
        );
        let ldr = (*peb).Ldr;
        let modules = (*ldr).InMemoryOrderModuleList;

        let mut cursor = modules.Flink;
        let mut write_file: Option<WriteFileFn> = None;
        // should never be null but just in case...
        'modules: while !cursor.is_null() {
            // We're iterating in memory order, so adjust to that header.
            let offset = mem::offset_of!(LDR_DATA_TABLE_ENTRY, InMemoryOrderLinks);
            let entry = cursor.byte_sub(offset).cast::<LDR_DATA_TABLE_ENTRY>();
            let dll_base = (*entry).DllBase.cast::<u8>();
            let full_name = &(*entry).FullDllName;
            if dll_base.is_null() {
                break;
            }
            if let Some(WriteFile) = write_file {
                let mut written = 0;
                let r = WriteFile(
                    -11,
                    full_name.Buffer.cast::<u8>(),
                    full_name.Length as u32,
                    &mut written,
                    0,
                );
                if r == 0 {
                    return core::line!();
                }
                let nl = (b'\n' as u16).to_ne_bytes();
                let r = WriteFile(-11, nl.as_ptr(), nl.len() as u32, &mut written, 0);
                if r == 0 {
                    return core::line!();
                }
            } else {
                // Find WriteFile so we can write output.
                // Once found, we iterate modules again from the start.
                let pe_offset = dll_base.byte_add(0x3c).cast::<u16>();
                let pe = dll_base
                    .byte_add(*pe_offset as usize)
                    .cast::<IMAGE_NT_HEADERS64>();
                let data =
                    &(*pe).OptionalHeader.DataDirectory[IMAGE_DIRECTORY_ENTRY_EXPORT as usize];
                if data.VirtualAddress != 0 {
                    let export_table = &*dll_base
                        .add(data.VirtualAddress as usize)
                        .cast::<IMAGE_EXPORT_DIRECTORY>();

                    let names = core::slice::from_raw_parts(
                        dll_base
                            .add(export_table.AddressOfNames as usize)
                            .cast::<u32>(),
                        export_table.NumberOfNames as usize,
                    );
                    let name_ordinals = core::slice::from_raw_parts(
                        dll_base
                            .add(export_table.AddressOfNameOrdinals as usize)
                            .cast::<u16>(),
                        export_table.NumberOfNames as usize,
                    );
                    let functions = core::slice::from_raw_parts(
                        dll_base
                            .add(export_table.AddressOfFunctions as usize)
                            .cast::<u32>(),
                        export_table.NumberOfFunctions as usize,
                    );
                    for i in 0..(*export_table).NumberOfNames as usize {
                        // See https://learn.microsoft.com/en-us/windows/win32/debug/pe-format#export-ordinal-table
                        let name = dll_base.add(names[i] as usize);
                        if eq_cstr(b"WriteFile\0".as_ptr(), name) {
                            let ordinal = name_ordinals[i];
                            let Some(&function) = functions.get(ordinal as usize) else {
                                return core::line!();
                            };

                            if function >= data.VirtualAddress
                                && function < data.VirtualAddress + data.Size
                            {
                                // Note: I didn't bother handling forwarders
                                continue;
                            }
                            let function = dll_base.add(function as usize);
                            let WriteFile: WriteFileFn = core::mem::transmute(function);
                            write_file = Some(WriteFile);
                            // Reset the iterator to the beginning.
                            cursor = modules.Flink;
                            continue 'modules;
                        }
                    }
                }
            }
            cursor = (*cursor).Flink;
        }
    }
    0
}

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    loop {}
}
