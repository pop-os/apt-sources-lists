extern crate apt_sources_lists;

use apt_sources_lists::*;

pub fn main() {
    let list = SourcesLists::scan().unwrap();
    for file in list.iter() {
        println!("{}:", file.path.display());
        for entry in &file.lines {
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
