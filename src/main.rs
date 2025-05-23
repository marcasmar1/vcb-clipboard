use std::io::Cursor;

use base64::prelude::*;
use ndarray::Array2;
use zstd::decode_all;

mod blueprint;

use crate::blueprint::*; //{Blueprint, BlueprintPrimitive};
use binrw::BinRead;

use colored::*;

fn main() {
    let clipboard ="VCB+AAAAfoAZnr9sAAAACQAAAAwAAABHAAAAAAAAAbAotS/9YLAAjQEAoAAAZniO/6GYVk04Pv8uR13/kv9jCwDJiaEACPgJAnB2FAsAEFgYugIbOLCbzaMACwAAAB8AAAABAAABsCi1L/1gsABNAAAQAAABAKsqwAIAAAAfAAAAAgAAAbAotS/9YLAATQAAEAAAAQCrKsAC";
    // TODO: Move this to clipboard_to_blueprint function
    let clipboard = clipboard.strip_prefix("VCB+").unwrap();

    let clipboard = BASE64_STANDARD.decode(clipboard).unwrap();

    let aligned_clipboard = clipboard.clone().split_off(9);
    for (position, chunk) in aligned_clipboard.chunks_exact(4).enumerate() {
        print!("{:#04x}:\t", position * 4 + 9);
        for val in chunk {
            print!("{:02x}\t", val);
        }
        println!("");
    }

    let blueprint = BlueprintPrimitive::read(&mut Cursor::new(clipboard)).unwrap();

    let grid = match &blueprint.block_info[0].content {
        BlockPayload::Logic(x) => Some(x.rgba8_buffer.clone()),
        _ => None,
    }
    .unwrap();

    show_grid(
        grid,
        blueprint.header.height as usize,
        blueprint.header.width as usize,
    );

    let blueprint: Blueprint = blueprint.try_into().unwrap();

    for row in blueprint.logic_grid.rows() {
        for rgba in row {
            let r = (rgba >> 24) as u8;
            let g = ((rgba >> 16) & 0xFF) as u8;
            let b = ((rgba >> 8) & 0xFF) as u8;
            let _a = (rgba & 0xFF) as u8;

            print!("{}", "██".truecolor(r, g, b));
        }
        println!();
    }
    //println!("{:#?}", blueprint);
}

fn show_grid(grid: Vec<u32>, height: usize, width: usize) -> () {
    let grid = Array2::from_shape_vec([height, width], grid).unwrap();

    println!("{:?}", grid);

    for row in 0..height {
        for col in 0..width {
            let rgba = grid[[row, col]];
            let r = (rgba >> 24) as u8;
            let g = ((rgba >> 16) & 0xFF) as u8;
            let b = ((rgba >> 8) & 0xFF) as u8;
            let _a = (rgba & 0xFF) as u8;

            // Create a colored block using the RGB values
            let block = "██".truecolor(r, g, b);
            print!("{}", block);
        }
        println!();
    }
}
