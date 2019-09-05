extern crate apt_sources_lists;

use apt_sources_lists::*;
use std::collections::HashSet;

pub fn main() {
    let mut list = SourcesLists::scan().unwrap();
    match list.dist_upgrade(&HashSet::new(), "disco", "cosmic") {
        Ok(()) => println!("successfully upgraded"),
        Err(why) => eprintln!("failed to upgrade: {}", why),
    }
}
