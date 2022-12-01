#![no_std]

use core::convert::TryInto;

const IMAGE_FILE_MACHINE_I386: u16 = 0x014c;
const IMAGE_FILE_MACHINE_X86_64: u16 = 0x8664;

const IMAGE_SCN_MEM_EXECUTE: u32 = 0x2000_0000;
const IMAGE_SCN_MEM_READ: u32 = 0x4000_0000;
const IMAGE_SCN_MEM_WRITE: u32 = 0x8000_0000;

pub struct PeParser<'a> {
    bytes: &'a [u8],
    image_base: u64,
    num_sections: usize,
    section_off: usize,
    /// Virtual Address of the entry point
    pub entry_point: u64,
}

impl<'a> PeParser<'a> {
    pub fn parse(bytes: &'a [u8]) -> Option<Self> {
        // Get a reference to bytes
        let bytes: &[u8] = bytes.as_ref();

        // Check for an MZ header
        if bytes.get(0..2) != Some(b"MZ") { return None; }

        // Get the PE offset
        let pe_offset: usize =
            u32::from_le_bytes(bytes.get(0x3c..0x40)?.try_into().ok()?).try_into().ok()?;

        // Check for the PE signature
        if bytes.get(pe_offset..pe_offset.checked_add(4)?) != Some(b"PE\0\0") {
            return None;
        }

        // Make sure the COFF header is within bounds of our input
        if pe_offset.checked_add(0x18)? > bytes.len() {
            return None;
        }

        // Get machine field and check it belongs to x86 or x86_64
        let machine: u16 = u16::from_le_bytes(bytes[pe_offset + 4..pe_offset + 6]
            .try_into().ok()?);

        if machine != IMAGE_FILE_MACHINE_I386 && machine != IMAGE_FILE_MACHINE_X86_64 {
            return None;
        }

        // Get number of sections
        let num_sections: usize = u16::from_le_bytes(bytes[pe_offset + 6..pe_offset + 8]
            .try_into().ok()?).try_into().ok()?;

        // Get optional header size
        let opt_header_size: usize = u16::from_le_bytes(bytes[pe_offset + 0x14..pe_offset + 0x16]
            .try_into().ok()?).into();


        // Get the base for the program
        let image_base = if machine == IMAGE_FILE_MACHINE_I386 {
            u32::from_le_bytes(
                bytes.get(pe_offset + 0x34..pe_offset + 0x38)?.try_into().ok()?
            ) as u64
        } else if machine == IMAGE_FILE_MACHINE_X86_64 {
            u64::from_le_bytes(
                bytes.get(pe_offset + 0x30..pe_offset + 0x38)?.try_into().ok()?
            )
        } else {
            unreachable!();
        };

        let entry_point: u64 = u32::from_le_bytes(
                bytes.get(pe_offset + 0x28..pe_offset + 0x2c)?.try_into().ok()?
            ) as u64;
        let entry_point = image_base.checked_add(entry_point)?;

        // Compute the size of all headers, including sections
        let header_size = pe_offset.checked_add(0x18)?
            .checked_add(opt_header_size as usize)?
            .checked_add(0x28usize.checked_mul(num_sections)?)?;

        if header_size > bytes.len() {
            return None;
        }

        Some(PeParser {
            bytes,
            image_base,
            num_sections,
            entry_point,
            section_off: pe_offset + 0x18 + opt_header_size,
        })
    }

    /// Invoke a closue with the format
    /// (virtual_address, virtual_size, raw_bytes, read, write, execture) for each section in the
    /// PE file
    pub fn sections<F: FnMut(u64, u32, &[u8], bool, bool, bool) -> Option<()>>(
        &self,
        mut func: F
    ) -> Option<()> {
        let bytes = self.bytes;

        for section in 0..self.num_sections {
            let off = self.section_off + section * 0x28;

            // Get the virtual raw sizes and offsets
            let virt_size = u32::from_le_bytes(bytes[off + 0x8..off + 0xc].try_into().ok()?);
            let virt_addr = u32::from_le_bytes(bytes[off + 0xc..off + 0x10].try_into().ok()?);
            let raw_size = u32::from_le_bytes(bytes[off + 0x10..off + 0x14].try_into().ok()?).try_into().ok()?;
            let raw_off: usize = u32::from_le_bytes(bytes[off + 0x14..off + 0x18].try_into().ok()?).try_into().ok()?;

            // Get the section characteristics
            let characteristics: u32 = u32::from_le_bytes(bytes[off + 0x24..off + 0x28].try_into().ok()?);

            let raw_size: usize = core::cmp::min(raw_size, virt_size as usize).try_into().ok()?;

            func(
                self.image_base.checked_add(virt_addr as u64)?,
                virt_size,
                bytes.get(raw_off..raw_off.checked_add(raw_size)?)?,
                (characteristics & IMAGE_SCN_MEM_READ) != 0,
                (characteristics & IMAGE_SCN_MEM_WRITE) != 0,
                (characteristics & IMAGE_SCN_MEM_EXECUTE) != 0,
            )?;
        }

        Some(())
    }
}
