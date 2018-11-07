# apt-sources-lists

Rust crate for fetching detailed information from all available apt sources.

### Example

```rust
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
```

### Example Output

```
/etc/apt/sources.list:
        # deb cdrom:[Pop_OS 18.04 _Bionic Beaver_ - Release amd64 (20180916)]/ bionic main restricted
        Comment("# deb cdrom:[Pop_OS 18.04 _Bionic Beaver_ - Release amd64 (20180916)]/ bionic main restricted")
        deb http://us.archive.ubuntu.com/ubuntu/ cosmic main restricted universe multiverse
        Entry(SourceEntry { source: false, options: None, url: "http://us.archive.ubuntu.com/ubuntu/", suite: "cosmic", components: ["main", "restricted", "univ
erse", "multiverse"] })
        deb-src http://us.archive.ubuntu.com/ubuntu/ cosmic main restricted universe multiverse
        Entry(SourceEntry { source: true, options: None, url: "http://us.archive.ubuntu.com/ubuntu/", suite: "cosmic", components: ["main", "restricted", "unive
rse", "multiverse"] })
        deb http://us.archive.ubuntu.com/ubuntu/ cosmic-updates main restricted universe multiverse

        Entry(SourceEntry { source: false, options: None, url: "http://us.archive.ubuntu.com/ubuntu/", suite: "cosmic-updates", components: ["main", "restricted
", "universe", "multiverse"] })
        deb-src http://us.archive.ubuntu.com/ubuntu/ cosmic-updates main restricted universe multiverse
        Entry(SourceEntry { source: true, options: None, url: "http://us.archive.ubuntu.com/ubuntu/", suite: "cosmic-updates", components: ["main", "restricted"
, "universe", "multiverse"] })
        deb http://us.archive.ubuntu.com/ubuntu/ cosmic-security main restricted universe multiverse
        Entry(SourceEntry { source: false, options: None, url: "http://us.archive.ubuntu.com/ubuntu/", suite: "cosmic-security", components: ["main", "restricte
d", "universe", "multiverse"] })
        deb-src http://us.archive.ubuntu.com/ubuntu/ cosmic-security main restricted universe multiverse
        Entry(SourceEntry { source: true, options: None, url: "http://us.archive.ubuntu.com/ubuntu/", suite: "cosmic-security", components: ["main", "restricted
", "universe", "multiverse"] })
        deb http://us.archive.ubuntu.com/ubuntu/ cosmic-backports main restricted universe multiverse
        Entry(SourceEntry { source: false, options: None, url: "http://us.archive.ubuntu.com/ubuntu/", suite: "cosmic-backports", components: ["main", "restrict
ed", "universe", "multiverse"] })
        deb-src [] http://us.archive.ubuntu.com/ubuntu/ cosmic-backports main restricted universe multiverse
        Entry(SourceEntry { source: true, options: Some(""), url: "http://us.archive.ubuntu.com/ubuntu/", suite: "cosmic-backports", components: ["main", "restr
icted", "universe", "multiverse"] })

        Empty
        # deb file:///home/mmstick/Sources/repo-proprietary/build/repo/ bionic main
        Comment("# deb file:///home/mmstick/Sources/repo-proprietary/build/repo/ bionic main")
        # deb file:///home/mmstick/Sources/repo-curated-free/repo/ bionic main
        Comment("# deb file:///home/mmstick/Sources/repo-curated-free/repo/ bionic main")
        # deb file:///home/mmstick/Sources/repo-rust-updates/repo cosmic main # disabled on upgrade to cosmic
        Comment("# deb file:///home/mmstick/Sources/repo-rust-updates/repo cosmic main # disabled on upgrade to cosmic")
        deb file:///home/mmstick/Sources/system76-cuda/repo cosmic main
        Entry(SourceEntry { source: false, options: None, url: "file:///home/mmstick/Sources/system76-cuda/repo", suite: "cosmic", components: ["main"] })
        deb http://apt.pop-os.org/proprietary cosmic main
        Entry(SourceEntry { source: false, options: None, url: "http://apt.pop-os.org/proprietary", suite: "cosmic", components: ["main"] })
        # deb-src http://apt.pop-os.org/proprietary cosmic main
        Comment("# deb-src http://apt.pop-os.org/proprietary cosmic main")
/etc/apt/sources.list.d/mmstick76-ubuntu-ion-shell-cosmic.list:
        deb http://ppa.launchpad.net/mmstick76/ion-shell/ubuntu cosmic main
        Entry(SourceEntry { source: false, options: None, url: "http://ppa.launchpad.net/mmstick76/ion-shell/ubuntu", suite: "cosmic", components: ["main"] })
        # deb-src http://ppa.launchpad.net/mmstick76/ion-shell/ubuntu cosmic main
        Comment("# deb-src http://ppa.launchpad.net/mmstick76/ion-shell/ubuntu cosmic main")
        # deb-src http://ppa.launchpad.net/mmstick76/ion-shell/ubuntu cosmic main
        Comment("# deb-src http://ppa.launchpad.net/mmstick76/ion-shell/ubuntu cosmic main")
/etc/apt/sources.list.d/system76-ubuntu-pop-bionic.list:
        deb http://ppa.launchpad.net/system76/pop/ubuntu cosmic main
        Entry(SourceEntry { source: false, options: None, url: "http://ppa.launchpad.net/system76/pop/ubuntu", suite: "cosmic", components: ["main"] })
        deb-src http://ppa.launchpad.net/system76/pop/ubuntu cosmic main
        Entry(SourceEntry { source: true, options: None, url: "http://ppa.launchpad.net/system76/pop/ubuntu", suite: "cosmic", components: ["main"] })
```
