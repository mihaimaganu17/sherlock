use std::{
    path::Path,
    process::Command,
    convert::TryInto,
};

use parse_pe::PeParser;

// Base address to the Rust bootloader
const BOOTLOADER_BASE: u32 = 0x7e00;
const MAX_BOOTLOADER_SIZE: u64 = 32 * 1024;

/// Create a flattened PE image
fn flatten_pe<P: AsRef<Path>>(filename: P) -> Option<(u32, u32, Vec<u8>)> {
    let pe = std::fs::read(filename).ok()?;
    let pe = PeParser::parse(&pe).unwrap();

    // Compute the bounds of the _loaded_ image
    let mut image_start = None;
    let mut image_end = None;

    pe.sections(|base, size, _raw, _, _, _| {
        let end = base.checked_add(size.checked_sub(1)?.into())?;

        if image_start.is_none() {
            image_start = Some(base);
            image_end = Some(end);
        }

        image_start = image_start.map(|x| core::cmp::min(x, base));
        image_end = image_end.map(|x| core::cmp::max(x, end));

        Some(())
    })?;

    let image_start = image_start?;
    let image_end = image_end?;
    let image_size = image_end.checked_sub(image_start)?.checked_add(1)?.try_into().ok()?;

    // Allocate a zeroed image
    let mut flattened = std::vec![0u8; image_size];

    pe.sections(|base, size, raw, _, _, _| {
        let flat_off: usize = (base - image_start).try_into().ok()?;
        let size: usize = size.try_into().ok()?;

        // Compute the number of bytes to initialize
        let to_copy = std::cmp::min(size, raw.len());

        flattened[flat_off..flat_off.checked_add(to_copy)?].copy_from_slice(raw);
        Some(())
    })?;

    // Make sure the entry point falls within the image
    if pe.entry_point < image_start || pe.entry_point > image_end {
        return None;
    }

    Some((pe.entry_point.try_into().ok()?, image_start.try_into().ok()?, flattened))
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create a build folder, if it does not exist
    let build_dir = Path::new("build");
    let bootloader_build_dir = build_dir.clone().join("bootloader").canonicalize().expect("Nope");

    std::fs::create_dir_all(&build_dir).expect("Failed to create build directory");
    std::fs::create_dir_all(&bootloader_build_dir).expect("Failed to create boot directory");
    std::fs::create_dir_all("build/kernel").expect("Failed to create kernel directory");


    // Create the boot file name
    let boot_file = build_dir.clone().join("sherlock.boot");

    // Build the assembly routines for the bootloader
    if !Command::new("nasm")
        .args(&[
            "-f",
            "win32",
            &format!("-DPROGRAM_BASE={:#x}", BOOTLOADER_BASE),
            Path::new("bootloader").join("src").join("asm_routines.asm").to_str().unwrap(),
            "-o",
            Path::new("build").join("bootloader").join("asm_routines.obj").to_str().unwrap()
        ])
        .status()?
        .success()
    {
        return Err("Failed to build bootloader assembly routines".into());
    }

    let boot_build_cmd = Command::new("cargo")
        .current_dir("bootloader")
        .args(&[
            "build",
            "--release",
            "--target",
            "i586-pc-windows-msvc",
            "--target-dir",
            bootloader_build_dir.to_str().unwrap()
        ]).status()?;

    if !boot_build_cmd.success() {
        return Err("Failed to build bootloader".into());
    }

    // Flatten the PE image
    let (entry, base, image) = flatten_pe(bootloader_build_dir.join("i586-pc-windows-msvc")
        .join("release").join("bootloader.exe"))
        .ok_or("Failed to flatten bootloader PE image")?;

    // Make sure the PE gets loaded to where we expect
    if base != BOOTLOADER_BASE {
        return Err("Base address for bootloader did not match expected".into());
    }

    // Write out the flattened bootloader image
    std::fs::write(Path::new("build").join("sherlock.flat"), image).expect("Failed to write flat");


    // Build the stage0
    let stage0 = Path::new("bootloader").join("src").join("stage0.asm");

    // Compile with `nasm`
    let nasm_stage0_cmd = Command::new("nasm")
        .args(&[
            "-f", "bin", &format!("-Dentry_point={:#x}", entry), "-o",
            boot_file.to_str().unwrap(), stage0.to_str().unwrap()
        ]).status()?;

    // Check command status
    if !nasm_stage0_cmd.success() {
        return Err("Failed to assemble stage0".into());
    }

    // Check bootloader size is within bounds
    let bl_size = boot_file.metadata()?.len();
    print!("Current bootloader size is {} of {} bytes [{:8.4} %]\n",
                bl_size, MAX_BOOTLOADER_SIZE,
                        bl_size as f64 / MAX_BOOTLOADER_SIZE as f64 * 100.);
    if bl_size > MAX_BOOTLOADER_SIZE {
        return Err("Bootloader size exceeds allowed PXE limit".into());
    }

    // Build the kernel
    let kernel_build_dir = Path::new("build").join("kernel").canonicalize()?;
    let kernel_exe = kernel_build_dir.join("x86_64-pc-windows-msvc")
        .join("release").join("kernel.exe");

    if !Command::new("cargo")
        .current_dir("kernel")
        .args(&[
            "build",
            "--release",
            "--target",
            "x86_64-pc-windows-msvc",
            "--target-dir",
            kernel_build_dir.to_str().unwrap()
        ]).status()?.success() {
       return Err("Failed to kernel".into()); 
    }

    //std::fs::copy(boot_file, "/home/m3m0ry/fun/sherlock/build/sherlock.boot")?;
    std::fs::copy(kernel_exe, Path::new("build").join("sherlock.kern"))?;

    Ok(())
}
