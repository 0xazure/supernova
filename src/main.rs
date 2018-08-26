extern crate supernova;

use std::{env, process};
use supernova::Config;

fn main() {
    let config = Config::new(env::args()).unwrap_or_else(|err| {
        println!("Problem parsing arguments: {}", err);
        process::exit(1);
    });

    if let Err(e) = supernova::collect_stars(config) {
        println!("Application error: {}", e);
        process::exit(1);
    }
}
