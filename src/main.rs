use std::io::Cursor;

use base64::prelude::*;
use ndarray::Array2;
use zstd::decode_all;

// TODO: Use binrw to simplify the process of reading binary text
// TODO: use Appendix/Blueprint Specification to get the clipboard format.
/* FORMAT SPECIFICATION
Overview
    - Start with "VCB+" prefix
    - Encoded in base 64
    - Bytes in big endian
Header
    3-byte blueprint version
    6-byte checksum (truncated SHA-1) of the remaining characters in the string
    4-byte width
    4-byte height
Layer Blocks (One per layer)
    4-byte block size (The size Layer Blokcs in bytes)
    4-byte layer id (0 logic, 1 deco on, 2 deco off)
    4-byte uncompressed buffer size
    N-byte zstd compressed RGBA8 buffer
Text Blocks (Optional, one per text block type)
    4-byte block size (The size Layer Blokcs in bytes)
    4-byte layer id (0 logic, 1 deco on, 2 deco off)
    4-byte uncompressed buffer size
    N-byte zstd compressed UTF-8 buffer

*/

// BASE64_STANDARD.decode(logic_data)?

#[derive(Debug)]
struct Blueprint {
    width: usize,
    height: usize,
    values: Array2<u8>,
}

const HEADER_SIZE_BYTES: usize = 32;

const ZSTD_MAGIC_NUMBER: u32 = 0xFD2FB528;

fn main() {
    let clipboard ="VCB+AAAAfoAZnr9sAAAACQAAAAwAAABHAAAAAAAAAbAotS/9YLAAjQEAoAAAZniO/6GYVk04Pv8uR13/kv9jCwDJiaEACPgJAnB2FAsAEFgYugIbOLCbzaMACwAAAB8AAAABAAABsCi1L/1gsABNAAAQAAABAKsqwAIAAAAfAAAAAgAAAbAotS/9YLAATQAAEAAAAQCrKsAC";
    let clipboard ="AAAAfoAZnr9sAAAACQAAAAwAAABHAAAAAAAAAbAotS/9YLAAjQEAoAAAZniO/6GYVk04Pv8uR13/kv9jCwDJiaEACPgJAnB2FAsAEFgYugIbOLCbzaMACwAAAB8AAAABAAABsCi1L/1gsABNAAAQAAABAKsqwAIAAAAfAAAAAgAAAbAotS/9YLAATQAAEAAAAQCrKsAC";

    let blueprint = clipboard_to_blueprint(clipboard.to_string());
    println!("{:?}", blueprint);
}

fn process_data(data: &Vec<u8>) -> Result<Blueprint, anyhow::Error> {
    let data_size = data.len();
    let header_start = (data_size as isize - HEADER_SIZE_BYTES as isize) as usize;
    let header = &data[header_start..data_size];

    let image_size = header[5] as usize;
    let width = header[3] as usize;
    let height = header[1] as usize;

    if image_size != width * height * 4 {
        return Err(anyhow::anyhow!(format!(
            "Header width x height does not match header length. Expected {}, received {}",
            image_size,
            width * height * 4
        )));
    }

    let compressed_data = &data[..header_start];

    println!("Decompress data");

    let mut decompressed = decode_all(Cursor::new(compressed_data))?;

    println!("Data decompressed");
    let decompressed_data = decompressed.split_off(2);

    let decompressed_size = u16::from_le_bytes([decompressed[0], decompressed[1]]) as usize;

    if decompressed_size != image_size {
        return Err(anyhow::anyhow!(format!(
            "Decompressed size does not match header size. Expected {}, received {}",
            image_size, decompressed_size
        )));
    }

    Ok(Blueprint {
        width,
        height,
        values: Array2::from_shape_vec((width, height), decompressed_data)?,
    })
}

fn clipboard_to_blueprint(clipboard: String) -> Result<Blueprint, anyhow::Error> {
    let mut b64_data = BASE64_STANDARD.decode(clipboard)?;

    if b64_data.len() <= 36 {
        return Err(anyhow::anyhow!(format!(
            "Data size too small. Expected {}, received {}",
            36,
            b64_data.len()
        )));
    }

    println!("{:?}", b64_data);
    let magic_number: Vec<_> = b64_data.drain(0..4).collect();
    let magic_number = u32::from_be_bytes([
        magic_number[0],
        magic_number[1],
        magic_number[2],
        magic_number[3],
    ]);

    println!("magic_number: {:x}", magic_number);

    // if magic_number != ZSTD_MAGIC_NUMBER {
    //     return Err(anyhow::anyhow!(format!(
    //         "Invalid ZSTD magic number. Expected {}, received {}",
    //         ZSTD_MAGIC_NUMBER, magic_number
    //     )));
    // }
    process_data(&b64_data)
}
