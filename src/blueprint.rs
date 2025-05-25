use std::io::Cursor;

use base64::{prelude::BASE64_STANDARD, Engine};
use binrw::{binrw, helpers::until_eof, BinRead};
use ndarray::Array2;
use zstd::{decode_all, encode_all};

// TODO: Create a library (create a lib.rs)
// TODO: Replace anyhow with thiserror

// TODO: Review code:
//  - Use `try_map` and remove all `unwrap()` calls.
//  - Create custom error type with `thiserror`
//  - Create constants for:
//      - PREFIX VCB+
//      - HEADER_SIZE
// - Consider replacing `Vec<String>`  with `Box<str>`
// - Find substitute for code in `TryFrom<BluePrintPrimitive>`
// - Add module level documentation
// - Document errors
// - add #[must_use] attributes where appropriate
// - Consider implementing `Display`
// - Revise public fields
// - Add #[non_exhaustive]
// - Revise the use of `Clone`
// - Implement `Default`
// - Consider logging

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
    4-byte block size (The size Layer Blocks in bytes)
    4-byte layer id (0 logic, 1 deco on, 2 deco off)
    4-byte uncompressed buffer size
    N-byte zstd compressed RGBA8 buffer
Text Blocks (Optional, one per text block type)
    4-byte block size (The size Layer Blocks in bytes)
    4-byte data id (1024 Name, 1025 Description, 1026 Tags)
    4-byte uncompressed buffer size
    N-byte zstd compressed UTF-8 buffer

*/

#[binrw]
#[derive(Debug)]
#[brw(big)]
/// Blueprint header containing version, checksum and dimensions.
/// This structure is always present at the start of the blueprint data.
struct Header {
    /// Blueprint version as a 3-byte array.
    version: [u8; 3],

    // TODO: Validate checksum
    /// Truncated SHA-1 checksum (6 bytes) of the remaining data.
    /// Used to verify data integrity.
    checksum: [u8; 6],

    /// Width of the blueprint grid in cells.
    /// Determines the horizontal size of all layers.
    width: u32,

    /// Height of the blueprint grid in cells.
    /// Determines the vertical size of all layers.
    height: u32,
}

#[binrw]
#[derive(Debug, Clone)]
#[brw(big)]
/// A block of data in the blueprint.
/// Each block contains a size header and its payload content.
struct Block {
    /// Total size of the block in bytes, including this size field.
    /// Used to determine how much data to read for the block.
    size: u32,

    /// The actual content of the block, containing either layer data or text.
    #[br(args(size))]
    content: BlockPayload,
}

#[binrw]
#[derive(Debug, Clone)]
#[brw(big)]
#[br(import(size:u32))]
/// Payload content of a block, identified by a magic number as a `u32`.
/// Can contain either layer data (logic/decoration) or text data (name/description/tags).
enum BlockPayload {
    /// Logic layer containing the circuit's functional components.
    /// Is always present on a blueprint.
    /// Identified by magic number 0.
    #[br(magic(0u32))]
    Logic(#[br(args(size))] Layer),

    /// Decoration layer for ON state visualization.
    /// Can be omitted.
    /// Identified by magic number 1.
    #[br(magic(1u32))]
    DecoOn(#[br(args(size))] Layer),

    /// Decoration layer for OFF state visualization.
    /// Can be omitted.
    /// Identified by magic number 2.
    #[br(magic(2u32))]
    DecoOff(#[br(args(size))] Layer),

    /// Blueprint name.
    /// Can be omitted.
    /// Identified by magic number 1024.
    #[br(magic(1024u32))]
    Name(#[br(args(size))] Text),

    /// Blueprint description text.
    /// Can be omitted.
    /// Identified by magic number 1025.
    #[br(magic(1025u32))]
    Description(#[br(args(size))] Text),

    /// Comma-separated list of blueprint tags.
    /// Can be omitted.
    /// Identified by magic number 1026.
    #[br(magic(1026u32))]
    Tags(#[br(args(size))] Text),
}

#[binrw]
#[derive(Debug, Clone)]
#[brw(big)]
#[br(import(size:u32))]
/// The layer information as a linearized array of rgba8888 values.
struct Layer {
    /// Size of the uncompressed RGBA8888 buffer in bytes.
    /// Each pixel uses 4 bytes (R,G,B,A).
    #[bw(try_calc(u32::try_from(rgba8_buffer.len() * 4)))]
    uncompressed_buffer_size: u32,

    /// Grid of RGBA8888 color values.
    /// Stored as a linear array of width * height elements.
    /// Each element is a 32-bit value containing RGBA components.
    /// Decompressed from the binary data using Zstd during reading.
    #[br(count = { // 12 = size of block_size + layer_id + uncompressed_buffer_size
        size.checked_sub(12).expect("Size must be at least 12 bytes")
    })]
    /* FIXME: This panics on errors */
    #[br(map = |compressed: Vec<u8>| {
        let decompressed = decode_all(Cursor::new(compressed)).expect("Decompression failed.");
        decompressed.chunks_exact(4)
            .map(|chunk| u32::from_be_bytes(chunk.try_into().expect("Not enough bytes in chunk.")))
            .collect::<Vec<u32>>()
    })]
    #[bw(map = |decompressed: &Vec<u32>| {
        let decompressed = decompressed.iter().map(|chunk|chunk.to_be_bytes()).collect::<Vec<[u8;4]>>().into_flattened();
        encode_all(Cursor::new(decompressed), 0).unwrap()
    })]
    rgba8_buffer: Vec<u32>,
}

#[binrw]
#[derive(Debug, Clone)]
#[brw(big)]
#[br(import(size:u32))]
struct Text {
    /// Size of the uncompressed UTF-8 text in bytes.
    #[bw(try_calc(u32::try_from(utf8_buffer.len())))]
    uncompressed_buffer_size: u32,

    /// The actual text content stored as a UTF-8 string.
    /// Decompressed from the binary data using Zstd during reading.
    #[br(count = size - 12)]
    /* FIXME: This panics on errors */
    #[br(map = |compressed: Vec<u8>| {
        let decompressed = decode_all(Cursor::new(compressed)).unwrap();
        String::from_utf8(decompressed).unwrap()
    })]
    #[bw(map = |decompressed: &String| {
        let decompressed = decompressed.clone().into_bytes();
        encode_all(Cursor::new(decompressed), 0).unwrap()
    })]
    utf8_buffer: String,
}

#[binrw]
#[derive(Debug)]
#[brw(big)]
/// Raw blueprint data as read from the binary format.
/// Contains the header and a list of data blocks.
struct BlueprintPrimitive {
    /// Blueprint header containing version and dimensions.
    header: Header,

    /// List of data blocks containing layers and text data.
    #[br(parse_with = until_eof)]
    block_info: Vec<Block>,
}

#[derive(Debug)]
/// Processed blueprint data.
/// Contains 2D layer grids and text data as strings.
pub struct Blueprint {
    /// The main logic layer as a 2D grid of RGBA values.
    pub logic_grid: Array2<u32>,

    /// Optional decoration layer for ON state (when present).
    pub deco_on_grid: Option<Array2<u32>>,

    /// Optional decoration layer for OFF state (when present).
    pub deco_off_grid: Option<Array2<u32>>,

    /// Optional blueprint name.
    pub name: Option<String>,

    /// Optional blueprint description.
    pub description: Option<String>,

    /// Optional list of blueprint tags.
    /// Each tag is stored as a separate string.
    pub tags: Option<Vec<String>>, /*FIXME: This might be better as str */
}

impl TryFrom<BlueprintPrimitive> for Blueprint {
    type Error = anyhow::Error;

    fn try_from(primitive: BlueprintPrimitive) -> Result<Self, Self::Error> {
        let shape = [
            primitive.header.height as usize,
            primitive.header.width as usize,
        ];

        let blocks: Vec<_> = primitive
            .block_info
            .iter()
            .map(|block| &block.content)
            .collect();

        // TODO: Improve this so it is more readable
        let logic_layer = blocks
            .iter()
            .filter_map(|block_payload| match block_payload {
                BlockPayload::Logic(x) => Some(x.rgba8_buffer.clone()),
                _ => None,
            })
            .next()
            .ok_or_else(|| anyhow::anyhow!("Logic layer missing."))?;

        let deco_on_layer = blocks
            .iter()
            .filter_map(|block_payload| match block_payload {
                BlockPayload::DecoOn(x) => Some(x.rgba8_buffer.clone()),
                _ => None,
            })
            .next();

        let deco_off_layer = blocks
            .iter()
            .filter_map(|block_payload| match block_payload {
                BlockPayload::DecoOff(x) => Some(x.rgba8_buffer.clone()),
                _ => None,
            })
            .next();

        let name = blocks
            .iter()
            .filter_map(|block_payload| match block_payload {
                BlockPayload::Name(x) => Some(x.utf8_buffer.clone()),
                _ => None,
            })
            .next();

        let description = blocks
            .iter()
            .filter_map(|block_payload| match block_payload {
                BlockPayload::Description(x) => Some(x.utf8_buffer.clone()),
                _ => None,
            })
            .next();

        let tags = blocks
            .iter()
            .filter_map(|block_payload| match block_payload {
                BlockPayload::Tags(x) => Some(x.utf8_buffer.clone()),
                _ => None,
            })
            .next();

        Ok(Blueprint {
            logic_grid: Array2::from_shape_vec(shape, logic_layer)?,
            deco_on_grid: deco_on_layer
                .map(|rgba8_buffer| Array2::from_shape_vec(shape, rgba8_buffer))
                .transpose()?,
            deco_off_grid: deco_off_layer
                .map(|rgba8_buffer| Array2::from_shape_vec(shape, rgba8_buffer))
                .transpose()?,

            name,
            description,
            tags: tags.map(|tags| {
                tags.split(", ")
                    .map(|tag| tag.to_string())
                    .collect::<Vec<String>>()
            }),
        })
    }
}

impl Blueprint {
    /// Attempts to generate a `Blueprint` from a string in the VCB clipboard format.
    pub fn try_from_str(clipboard: &str) -> Result<Self, anyhow::Error> {
        let encoded_clipboard = clipboard.strip_prefix("VCB+").ok_or_else(|| {
            anyhow::anyhow!("Prefix missing: string does not start with \"VCB+\".")
        })?;

        let decoded_clipboard = BASE64_STANDARD.decode(encoded_clipboard)?;

        let blueprint_primitive = BlueprintPrimitive::read(&mut Cursor::new(decoded_clipboard))?;

        blueprint_primitive.try_into()
    }
}
