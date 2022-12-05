use core::convert::TryInto;
use lockcell::LockCell;
use alloc::vec::Vec;
use crate::realmode::{invoke_realmode, pxecall, RegisterState};

/// A guard to prevent multiple uses of the PXE API at the same time
static PXE_GUARD: LockCell<()> = LockCell::new(());

/// Convert a 16-bit `seg:off` pointer into a linear address
fn segoff_to_linear(seg: u16, off: u16) -> usize {
    ((seg as usize) << 4) + off as usize
}

/// Download a file with the `filename` over TFTP with the PXE 16-bit API
pub fn download<P: AsRef<[u8]>>(filename: P) -> Option<Vec<u8>> {
    // Lock access to PXE
    let _guard = PXE_GUARD.lock();

    // Convert the filename to a slice of bytes
    let filename_bytes: &[u8] = filename.as_ref();

    // Invoke the PXE installation check with int 0x1a
    let mut regs = RegisterState::default();
    regs.eax = 0x5650;
    unsafe { invoke_realmode(0x1a, &mut regs); }

    if regs.eax != 0x564e || (regs.efl & 1) != 0 {
        return None;
    }

    // Get the linear address to the PXENV+ structure
    let pxenv = segoff_to_linear(regs.es, regs.ebx as u16);
    let pxenv = unsafe {
        core::slice::from_raw_parts(pxenv as *const u8, 0x2c)
    };

    // Extract the fields we need to validate the structure
    let signature = &pxenv[..6];
    let length = pxenv[0x8];
    // Compute the checksum of the PXENV structure
    let checksum = pxenv.iter().fold(0u8, |acc, &x| acc.wrapping_add(x));

    // Check the signature and length for sanity
    if &signature != b"PXENV+" || length != 0x2c || checksum != 0 {
        return None;
    }

    // Get the pointer to the !PXE structure
    let off = u16::from_le_bytes(pxenv[0x28..0x2a].try_into().ok()?);
    let seg = u16::from_le_bytes(pxenv[0x2a..0x2c].try_into().ok()?);
    let pxe = segoff_to_linear(seg, off);

    let pxe = unsafe {
        core::slice::from_raw_parts(pxe as *const u8, 0x58)
    };

    // Extract the fields we need to validate the !PXE structure
    let signature = &pxe[..4];
    let length = pxe[4];
    let checksum = pxe.iter().fold(0u8, |acc, &x| acc.wrapping_add(x));

    if signature != b"!PXE" || length != 0x58 || checksum != 0 {
        return None;
    }

    // Get the 16-bit PXE API entry point
    let ep_off = u16::from_le_bytes(pxe[0x10..0x12].try_into().ok()?);
    let ep_seg = u16::from_le_bytes(pxe[0x12..0x14].try_into().ok()?);

    // According to the spec "CS must not be 0000h"
    if ep_seg == 0 {
        return None;
    }

    // Determine the server IP from the cached information used during the PXE boot process. We
    // grab the DHCP ACK packet and extract the server IP field from it.
    let server_ip: [u8; 4] = {
        const PXE_OPCODE_GET_CACHED_INFO: u16 = 0x71;
        const PXENV_PACKET_TYPE_DHCP_ACK: u16 = 2;
        let mut pkt_buf = [0u8; 128];
        #[derive(Debug, Default)]
        #[repr(C)]
        struct GetCachedInfo {
            status: u16,
            packet_type: u16,
            buffer_size: u16,
            buffer_off: u16,
            buffer_seg: u16,
            buffer_limit: u16,
        }

        let mut st = GetCachedInfo::default();
        st.packet_type = PXENV_PACKET_TYPE_DHCP_ACK;
        st.buffer_size = 128;
        st.buffer_seg = 0;
        st.buffer_off = &mut pkt_buf as *mut _ as u16;

        unsafe { pxecall(ep_seg, ep_off, PXE_OPCODE_GET_CACHED_INFO, 0, &mut st as *mut _ as u16); }

        // Make sure the call was successfule
        if st.status != 0 {
            return None;
        }

        pkt_buf[0x14..0x18].try_into().ok()?
    };

    print!("TFTP Server IP: {}.{}.{}.{}\n",
        server_ip[0],
        server_ip[1],
        server_ip[2],
        server_ip[3],
    );    

    // Get the file size for the next stage
    let file_size = {
        const PXENV_TFTP_GET_FILE_SIZE: u16 = 0x25;

        #[repr(C, packed)]
        struct GetFileSize {
            status: u16,
            server_ip: [u8; 4],
            gateway_ip: [u8; 4],
            filename: [u8; 128],
            file_size: u32,
        }

        let mut st = GetFileSize {
            status: 0,
            server_ip: server_ip,
            gateway_ip: [0; 4],
            filename: [0; 128],
            file_size: 0,
        };

        // Check to see if we have enough room for the filename and the NULL terminator
        if filename_bytes.len() + 1 > st.filename.len() {
            return None;
        }

        // Copy the file name
        st.filename[..filename_bytes.len()].copy_from_slice(filename_bytes);

        unsafe {
            pxecall(ep_seg, ep_off, PXENV_TFTP_GET_FILE_SIZE, 0, &mut st as *mut _ as u16);
        }

        // Check that the call was successful
        if st.status != 0 {
            return None;
        }

        st.file_size as usize
    };
    
    print!("Requested file \"{:?}\" is {} bytes\n",
        core::str::from_utf8(filename.as_ref()),
        file_size
    );

    // Open the file
    {
        const PXE_OPCODE_TFTP_OPEN: u16 = 0x20;
        
        #[repr(C)]
        struct TftpOpen {
            status: u16,
            server_ip: [u8; 4],
            gateway_ip: [u8; 4],
            filename: [u8; 128],
            tftp_port: u16,
            packet_size: u16,
        }

        let mut st = TftpOpen {
            status: 0,
            server_ip: server_ip,
            gateway_ip: [0; 4],
            filename: [0; 128],
            tftp_port: 69_u16.to_be(),
            packet_size: 512,
        };

        // Check to see if we have enough room for the filename and the NULL terminator
        if filename_bytes.len() + 1 > st.filename.len() {
            return None;
        }

        // Copy the file name
        st.filename[..filename_bytes.len()].copy_from_slice(filename_bytes);

        unsafe {
            pxecall(ep_seg, ep_off, PXE_OPCODE_TFTP_OPEN, 0, &mut st as *mut _ as u16);
        }

        // Check that the call was successful
        if st.status != 0 || st.packet_size != 512 {
            return None;
        }

        print!("Openend file\n");
    }

    let mut download = alloc::vec::Vec::with_capacity(file_size);

    // Read the file
    {
        const PXE_OPCODE_TFTP_READ: u16 = 0x22;

        #[repr(C)]
        struct TftpRead {
            status: u16,
            packet_number: u16,
            buffer_size: u16,
            buffer_off: u16,
            buffer_seg: u16,
        }

        // Enough room to hold the packet size requested during open, which we use the minimum
        let mut read_buf = [0u8; 512];

        // Create the read request
        let mut st = TftpRead {
            status: 0,
            packet_number: 0,
            buffer_size: 0,
            buffer_off: &mut read_buf as *mut _ as u16, 
            buffer_seg: 0,
        };

        loop {
            // Do the request
            unsafe {
                pxecall(ep_seg, ep_off, PXE_OPCODE_TFTP_READ, 0, &mut st as *mut _ as u16);
            }

            // The number of bytes we read in the request
            let bytes_read = st.buffer_size as usize;

            if st.status != 0 || bytes_read > read_buf.len() {
                return None;
            }

            // Make suere we don't overflow our allocation. This can happen if the file has changed
            // since we got the size. We'll just fail here rather than causing re-allocs which are
            // not handled well with our high-fragmentation bootloader heap.
            if download.len() + bytes_read > download.capacity() {
                return None;
            }

            download.extend_from_slice(&read_buf[..bytes_read]);

            if bytes_read < read_buf.len() {
                break;
            }
        }
    }

    print!("Downloaded {} bytes\n", download.len());

    // Close file
    {
        const PXE_OPCODE_TFTP_CLOSE: u16 = 0x21;
        let mut status: u16 = 0;

        // Do the request
        unsafe {
            pxecall(ep_seg, ep_off, PXE_OPCODE_TFTP_CLOSE, 0, &mut status as *mut _ as u16);
        }

        if status != 0 {
            return None;
        }

        print!("Closed file\n");
    }

    Some(download)
}
