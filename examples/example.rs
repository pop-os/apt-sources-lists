extern crate apt_sources_lists;

use apt_sources_lists::*;

pub fn main() {
    let list = SourcesList::scan().unwrap();
    for event in list.into_iter() {
        match event {
            SourceEvent::NewList(path) => {
                println!("{}:", path.display());
            }
            SourceEvent::Entry(entry) => {
                // Pretty
                println!("\t{}", entry);
                // Details
                println!("\t{:?}", entry);
            }
        }
    }
}
