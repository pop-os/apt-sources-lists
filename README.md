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

```

### Example Output

```
/etc/apt/sources.list:
  # deb cdrom:[Pop_OS 18.04 _Bionic Beaver_ - Release amd64 (20180916)]/ bionic main restricted
  deb http://us.archive.ubuntu.com/ubuntu/ cosmic main restricted universe multiverse
    Dist paths:
      http://us.archive.ubuntu.com/ubuntu/dists/cosmic/main
      http://us.archive.ubuntu.com/ubuntu/dists/cosmic/restricted
      http://us.archive.ubuntu.com/ubuntu/dists/cosmic/universe
      http://us.archive.ubuntu.com/ubuntu/dists/cosmic/multiverse
    Pool path: http://us.archive.ubuntu.com/ubuntu/pool/
  deb-src http://us.archive.ubuntu.com/ubuntu/ cosmic main restricted universe multiverse
    Dist paths:
      http://us.archive.ubuntu.com/ubuntu/dists/cosmic/main
      http://us.archive.ubuntu.com/ubuntu/dists/cosmic/restricted
      http://us.archive.ubuntu.com/ubuntu/dists/cosmic/universe
      http://us.archive.ubuntu.com/ubuntu/dists/cosmic/multiverse
    Pool path: http://us.archive.ubuntu.com/ubuntu/pool/
  deb http://us.archive.ubuntu.com/ubuntu/ cosmic-updates main restricted universe multiverse
    Dist paths:
      http://us.archive.ubuntu.com/ubuntu/dists/cosmic-updates/main
      http://us.archive.ubuntu.com/ubuntu/dists/cosmic-updates/restricted
      http://us.archive.ubuntu.com/ubuntu/dists/cosmic-updates/universe
      http://us.archive.ubuntu.com/ubuntu/dists/cosmic-updates/multiverse
    Pool path: http://us.archive.ubuntu.com/ubuntu/pool/
  deb-src http://us.archive.ubuntu.com/ubuntu/ cosmic-updates main restricted universe multiverse
    Dist paths:
      http://us.archive.ubuntu.com/ubuntu/dists/cosmic-updates/main
      http://us.archive.ubuntu.com/ubuntu/dists/cosmic-updates/restricted
      http://us.archive.ubuntu.com/ubuntu/dists/cosmic-updates/universe
      http://us.archive.ubuntu.com/ubuntu/dists/cosmic-updates/multiverse
    Pool path: http://us.archive.ubuntu.com/ubuntu/pool/
  deb http://us.archive.ubuntu.com/ubuntu/ cosmic-security main restricted universe multiverse
    Dist paths:
      http://us.archive.ubuntu.com/ubuntu/dists/cosmic-security/main
      http://us.archive.ubuntu.com/ubuntu/dists/cosmic-security/restricted
      http://us.archive.ubuntu.com/ubuntu/dists/cosmic-security/universe
      http://us.archive.ubuntu.com/ubuntu/dists/cosmic-security/multiverse
    Pool path: http://us.archive.ubuntu.com/ubuntu/pool/
  deb-src http://us.archive.ubuntu.com/ubuntu/ cosmic-security main restricted universe multiverse
    Dist paths:
      http://us.archive.ubuntu.com/ubuntu/dists/cosmic-security/main
      http://us.archive.ubuntu.com/ubuntu/dists/cosmic-security/restricted
      http://us.archive.ubuntu.com/ubuntu/dists/cosmic-security/universe
      http://us.archive.ubuntu.com/ubuntu/dists/cosmic-security/multiverse
    Pool path: http://us.archive.ubuntu.com/ubuntu/pool/
  deb http://us.archive.ubuntu.com/ubuntu/ cosmic-backports main restricted universe multiverse
    Dist paths:
      http://us.archive.ubuntu.com/ubuntu/dists/cosmic-backports/main
      http://us.archive.ubuntu.com/ubuntu/dists/cosmic-backports/restricted
      http://us.archive.ubuntu.com/ubuntu/dists/cosmic-backports/universe
      http://us.archive.ubuntu.com/ubuntu/dists/cosmic-backports/multiverse
    Pool path: http://us.archive.ubuntu.com/ubuntu/pool/
  deb-src http://us.archive.ubuntu.com/ubuntu/ cosmic-backports main restricted universe multiverse
    Dist paths:
      http://us.archive.ubuntu.com/ubuntu/dists/cosmic-backports/main
      http://us.archive.ubuntu.com/ubuntu/dists/cosmic-backports/restricted
      http://us.archive.ubuntu.com/ubuntu/dists/cosmic-backports/universe
      http://us.archive.ubuntu.com/ubuntu/dists/cosmic-backports/multiverse
    Pool path: http://us.archive.ubuntu.com/ubuntu/pool/
  deb http://apt.pop-os.org/proprietary cosmic main
    Dist paths:
      http://apt.pop-os.org/proprietary/dists/cosmic/main
    Pool path: http://apt.pop-os.org/proprietary/pool/
  # deb-src http://apt.pop-os.org/proprietary cosmic main
/etc/apt/sources.list.d/mmstick76-ubuntu-ion-shell-cosmic.list:
  deb http://ppa.launchpad.net/mmstick76/ion-shell/ubuntu cosmic main
    Dist paths:
      http://ppa.launchpad.net/mmstick76/ion-shell/ubuntu/dists/cosmic/main
    Pool path: http://ppa.launchpad.net/mmstick76/ion-shell/ubuntu/pool/
  # deb-src http://ppa.launchpad.net/mmstick76/ion-shell/ubuntu cosmic main
  # deb-src http://ppa.launchpad.net/mmstick76/ion-shell/ubuntu cosmic main
/etc/apt/sources.list.d/dmj726-ubuntu-nvidia-367-bionic.list:
  # deb http://ppa.launchpad.net/dmj726/nvidia-367/ubuntu cosmic main # disabled on upgrade to cosmic
  # deb-src http://ppa.launchpad.net/dmj726/nvidia-367/ubuntu bionic main
/etc/apt/sources.list.d/system76-ubuntu-pop-bionic.list:
  deb http://ppa.launchpad.net/system76/pop/ubuntu cosmic main
    Dist paths:
      http://ppa.launchpad.net/system76/pop/ubuntu/dists/cosmic/main
    Pool path: http://ppa.launchpad.net/system76/pop/ubuntu/pool/
  deb-src http://ppa.launchpad.net/system76/pop/ubuntu cosmic main
    Dist paths:
      http://ppa.launchpad.net/system76/pop/ubuntu/dists/cosmic/main
    Pool path: http://ppa.launchpad.net/system76/pop/ubuntu/pool/
```
