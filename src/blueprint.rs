use std::io::Cursor;

use anyhow::Error;
use binrw::{binrw, helpers::until_eof};
use ndarray::Array2;
use zstd::{decode_all, encode_all};

// TODO: Create a library (create a lib.rs)
// TODO: Replace anyhow with thiserror

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
pub struct Header {
    version: [u8; 3],
    // TODO: Validate checksum
    checksum: [u8; 6],
    pub width: u32,
    pub height: u32,
}

#[binrw]
#[derive(Debug, Clone)]
#[brw(big)]
pub struct Block {
    #[br(dbg)]
    size: u32,

    //id: u32,
    #[br(args(size))]
    #[br(dbg)]
    pub content: BlockPayload,
}

// TODO: Find a way to utilize this enum for the data itself
#[binrw]
#[derive(Debug, Clone)]
#[brw(big)]
#[br(import(size:u32))]
pub enum BlockPayload {
    #[br(magic(0u32))]
    Logic(#[br(args(size))] Layer),
    #[br(magic(1u32))]
    DecoOn(#[br(args(size))] Layer),
    #[br(magic(2u32))]
    DecoOff(#[br(args(size))] Layer),
    #[br(magic(1024u32))]
    Name(#[br(args(size))] Text),
    #[br(magic(1025u32))]
    Description(#[br(args(size))] Text),
    #[br(magic(1026u32))]
    Tags(#[br(args(size))] Text),
}

#[binrw]
#[derive(Debug, Clone)]
#[brw(big)]
#[br(import(size:u32))]
pub struct Layer {
    #[bw(try_calc(u32::try_from(rgba8_buffer.len() * 4)))]
    uncompressed_buffer_size: u32,

    #[br(count = {
        println!("Debug: Reading Layer with size = {}", size);
        size.checked_sub(12).expect("Size must be at least 12 bytes")
    })]
    // 12 = size of block_size + layer_id + uncompressed_buffer_size
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
    pub rgba8_buffer: Vec<u32>,
}

#[binrw]
#[derive(Debug, Clone)]
#[brw(big)]
#[br(import(size:u32))]
pub struct Text {
    #[bw(try_calc(u32::try_from(utf8_buffer.len())))]
    uncompressed_buffer_size: u32,

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
pub struct BlueprintPrimitive {
    pub header: Header,

    #[br(parse_with = until_eof)]
    pub block_info: Vec<Block>,
}

#[derive(Debug)]
pub struct Blueprint {
    pub logic_grid: Array2<u32>,
    pub deco_on_grid: Option<Array2<u32>>,
    pub deco_off_grid: Option<Array2<u32>>,
    pub name: Option<String>,
    pub description: Option<String>,
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
