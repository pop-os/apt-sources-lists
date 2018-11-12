extern crate apt_sources_lists;

use apt_sources_lists::*;

pub fn main() {
    let list = SourcesList::scan().unwrap();
    for event in list.iter() {
        match event {
            SourceEvent::NewList(path) => {
                println!("{}:", path.display());
            }
            SourceEvent::Entry(entry) => {
                // Pretty
                println!("  {}", entry);
                if let SourceLine::Entry(ref entry) = *entry {
                    println!("    Dist paths:");
                    for dist in entry.dist_components() {
                        println!("      {}", dist);
                    }
                    println!("    Pool path: {}", entry.pool_path());
                }
            }
        }
    }
}
