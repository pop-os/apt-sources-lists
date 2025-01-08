use oma_apt_sources_lists::*;

pub fn main() {
    let list = SourcesLists::scan().unwrap();
    for file in list.iter() {
        println!("{}:", file.path.display());
        match &file.entries {
            SourceListType::Deb822(entries) => {
                for entry in &entries.entries {
                    println!("  {}", entry);
                    println!("    Dist paths:");
                    for dist in entry.dist_components() {
                        println!("      {}", dist);
                    }
                    println!("    Pool path: {}", entry.pool_path());
                }
            }
            SourceListType::SourceLine(lines) => {
                for entry in &lines.0 {
                    println!("  {}", entry);
                    if let SourceLine::Entry(ref entry) = entry {
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
}
