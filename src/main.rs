extern crate clap;
extern crate supernova;

use clap::{App, Arg, crate_name, crate_version};
use std::process;
use supernova::Config;

fn main() {
    let config: Config = App::new(crate_name!())
        .version(crate_version!())
        .arg(
            Arg::with_name("USERNAME")
                .help("The user whose stars to collect")
                .required(true),
        ).arg(
            Arg::with_name("TOKEN")
                .short("t")
                .long("token")
                .help("Sets the authentication token for requests to GitHub")
                .takes_value(true),
        ).get_matches()
        .into();

    if let Err(e) = supernova::collect_stars(config) {
        println!("Application error: {}", e);
        process::exit(1);
    }
}
