extern crate apt_sources_lists;

use apt_sources_lists::*;

pub fn main() {
    let mut list = SourcesList::scan().unwrap();
    match list.dist_upgrade("cosmic", "disco") {
        Ok(()) => println!("successfully upgraded"),
        Err(why) => eprintln!("failed to upgrade: {}", why),
    }
}
