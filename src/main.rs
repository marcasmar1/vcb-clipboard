use ndarray::Array2;

use vcb_clipboard::*;

use colored::*;

fn main() {
    let clipboard = "VCB+AAAAtordPn2VAAAAGAAAABkAAAE9AAAAAAAACWAotS/9YGAIPQkAdAIAAKFsVv+hmFb/n6iuoVVe/1Ve/2ZWoVVe/yo1QVVeVnuhVmJ7YmKAjaiwJRk2A7CCEaGrOhKgggzFYwyEM4pARpQlIyeRZCcsSHMOIf09hmMXT1TUI3Q7oMYRHmgbbsfhWfmeQE+qALlEdgYJryyGiV0BRwUvkKPREiPxMPFAtEa8/VrTedP4yI10NO/MwXn+//43SaqH7IPg+Z/P8eYvW8e0052esb39Hes7LeE/WeaY/YfoWXi0Td8QN7ZyeivrR0beMHZ0frqHL5G/c6TkzvlGWqzCZhnut0b2sz6pBTvHdLaTfC6eW8uqrwWjpSmAc4xrcQMYpv1K84HP2dvORZmHzMOJKcR/iDuNvp/3yGT+3YJXpDbOWHdQ7vjkxOfqadB6LQAAAB8AAAABAAAJYCi1L/1gYAhNAAAQAAABAFvxARYAAAAfAAAAAgAACWAotS/9YGAITQAAEAAAAQBb8QEWAAAAGAAABAAAAAADKLUv/SADGQAA4LaeAAAAPwAABAEAAAAqKLUv/SAqUQEAVGhlcmUgaXMgYW4gaW1wb3N0b3IgYW1vbmcgdGhlIGJsdWVwcmludHMuAAAAGAAABAIAAAADKLUv/SADGQAAU3Vz";

    let blueprint = Blueprint::try_from_str(clipboard).unwrap();

    show_grid(blueprint.logic_grid);

    println!("{:#?}", blueprint.name);
    println!("{:#?}", blueprint.description);
    println!("{:#?}", blueprint.tags);
    //println!("{:#?}", blueprint);
}

fn show_grid(grid: Array2<u32>) -> () {
    for row in grid.rows() {
        for rgba in row {
            let r = (rgba >> 24) as u8;
            let g = ((rgba >> 16) & 0xFF) as u8;
            let b = ((rgba >> 8) & 0xFF) as u8;
            let _a = (rgba & 0xFF) as u8;

            print!("{}", "██".truecolor(r, g, b));
        }
        println!();
    }
}
