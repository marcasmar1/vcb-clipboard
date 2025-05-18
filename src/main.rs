use std::io::Cursor;

use base64::prelude::*;
use ndarray::Array2;
use zstd::decode_all;

mod blueprint;

use crate::blueprint::{Blueprint, BlueprintPrimitive};
use binrw::BinRead;

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
    4-byte data id (1024 Name, 1025 Description, 1026 Tags)
    4-byte uncompressed buffer size
    N-byte zstd compressed UTF-8 buffer

*/

// BASE64_STANDARD.decode(logic_data)?

const HEADER_SIZE_BYTES: usize = 32;

const ZSTD_MAGIC_NUMBER: u32 = 0xFD2FB528;

fn main() {
    let clipboard ="VCB+AAAAfoAZnr9sAAAACQAAAAwAAABHAAAAAAAAAbAotS/9YLAAjQEAoAAAZniO/6GYVk04Pv8uR13/kv9jCwDJiaEACPgJAnB2FAsAEFgYugIbOLCbzaMACwAAAB8AAAABAAABsCi1L/1gsABNAAAQAAABAKsqwAIAAAAfAAAAAgAAAbAotS/9YLAATQAAEAAAAQCrKsAC";
    // TODO: Move this to clipboard_to_blueprint function
    let clipboard = clipboard.strip_prefix("VCB+").unwrap();

    let blueprint = BlueprintPrimitive::read(&mut Cursor::new(clipboard)).unwrap();

    let blueprint: Blueprint = blueprint.try_into().unwrap();

    println!("{:?}", blueprint);
}
