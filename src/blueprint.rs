use std::io::Cursor;

use binrw::binrw;
use ndarray::Array2;
use zstd::{decode_all, encode_all};

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
    4-byte block size (The size Layer Blokcs in bytes)
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
    width: u32,
    height: u32,
}

#[binrw]
#[derive(Debug)]
#[brw(big, repr = u32)]
pub enum LayerId {
    Logic = 0,
    DecoOn = 1,
    DecoOff = 2,
}

#[binrw]
#[derive(Debug)]
#[brw(big)]
pub struct Layer {
    block_size: u32,
    layer_id: u32,

    #[bw(try_calc(u32::try_from(rgba8_buffer.len() * 4)))]
    uncompressed_buffer_size: u32,

    #[br(count = block_size - 12)]
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
    rgba8_buffer: Vec<u32>,
}

#[binrw]
#[derive(Debug)]
#[brw(big, repr = u32)]
pub enum DataId {
    Name = 1024,
    Description = 1025,
    Tags = 1026,
}

#[binrw]
#[derive(Debug)]
#[brw(big)]
pub struct Text {
    block_size: u32,
    data_id: u32,

    #[bw(try_calc(u32::try_from(utf8_buffer.len())))]
    uncompressed_buffer_size: u32,

    #[br(count = block_size - 12)]
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
    header: Header,

    logic_layer: Layer,
    deco_on_layer: Option<Layer>,
    deco_off_layer: Option<Layer>,

    name_text: Option<Text>,
    description_text: Option<Text>,
    tags_text: Option<Text>,
}

#[derive(Debug)]
pub struct Blueprint {
    logic_grid: Array2<u32>,
    deco_on_grid: Option<Array2<u32>>,
    deco_off_grid: Option<Array2<u32>>,
    name: Option<String>,
    description: Option<String>,
    tags: Option<Vec<String>>, /*FIXME: This might be better as str */
}

impl TryFrom<BlueprintPrimitive> for Blueprint {
    type Error = anyhow::Error;

    fn try_from(primitive: BlueprintPrimitive) -> Result<Self, Self::Error> {
        let shape = [
            primitive.header.width as usize,
            primitive.header.height as usize,
        ];

        Ok(Blueprint {
            logic_grid: Array2::from_shape_vec(shape, primitive.logic_layer.rgba8_buffer)?,
            deco_on_grid: primitive
                .deco_on_layer
                .map(|layer| Array2::from_shape_vec(shape, layer.rgba8_buffer))
                .transpose()?,
            deco_off_grid: primitive
                .deco_off_layer
                .map(|layer| Array2::from_shape_vec(shape, layer.rgba8_buffer))
                .transpose()?,

            name: primitive.name_text.map(|text| text.utf8_buffer),
            description: primitive.description_text.map(|text| text.utf8_buffer),
            tags: primitive.tags_text.map(|text| {
                text.utf8_buffer
                    .split(", ")
                    .map(|tag| tag.to_string())
                    .collect::<Vec<String>>()
            }),
        })
    }
}
