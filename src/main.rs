#![allow(unused)]

mod diffing;
mod hello_world;

use crate::hello_world::get_hw_string;

fn main() {
    let hw = get_hw_string();
    println!("{}", hw);
}
