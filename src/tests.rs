use deb822::signature::Signature;

pub use super::*;
use std::{path::PathBuf, str::FromStr};

const SOURCE_LIST: &str = r#"
# deb cdrom:[Pop_OS 18.04 _Bionic Beaver_ - Release amd64 (20180916)]/ bionic main restricted
deb http://us.archive.ubuntu.com/ubuntu/ disco restricted multiverse universe main
deb-src http://us.archive.ubuntu.com/ubuntu/ disco restricted multiverse universe main
deb http://us.archive.ubuntu.com/ubuntu/ disco-updates restricted multiverse universe main
deb-src http://us.archive.ubuntu.com/ubuntu/ disco-updates restricted multiverse universe main
deb http://us.archive.ubuntu.com/ubuntu/ disco-security restricted multiverse universe main
deb-src http://us.archive.ubuntu.com/ubuntu/ disco-security restricted multiverse universe main
deb http://us.archive.ubuntu.com/ubuntu/ disco-backports restricted multiverse universe main
deb-src http://us.archive.ubuntu.com/ubuntu/ disco-backports restricted multiverse universe main
deb http://us.archive.ubuntu.com/ubuntu/ disco-proposed restricted multiverse universe main
deb-src http://us.archive.ubuntu.com/ubuntu/ disco-proposed restricted multiverse universe main

deb http://apt.pop-os.org/proprietary disco main
# deb-src http://apt.pop-os.org/proprietary disco main
"#;

const POP_PPA: &str = r#"
deb http://ppa.launchpad.net/system76/pop/ubuntu disco main
deb-src http://ppa.launchpad.net/system76/pop/ubuntu disco main
"#;

const POP_PPA_DISABLED: &str = r#"
# deb http://ppa.launchpad.net/system76/pop/ubuntu disco main
# deb-src http://ppa.launchpad.net/system76/pop/ubuntu disco main
"#;

fn sources_lists() -> SourcesLists {
    SourcesLists {
        modified: Vec::new(),
        files: vec![
            SourcesList {
                path: PathBuf::new(),
                entries: sources_list::SourceListType::SourceLine(
                    SOURCE_LIST
                        .parse::<SourceListLineStyle>()
                        .expect("source list gen"),
                ),
            },
            SourcesList {
                path: PathBuf::new(),
                entries: sources_list::SourceListType::SourceLine(
                    POP_PPA.parse::<SourceListLineStyle>().expect("pop ppa gen"),
                ),
            },
        ],
    }
}

fn sources_lists_pop_disabled() -> SourcesLists {
    SourcesLists {
        modified: Vec::new(),
        files: vec![
            SourcesList {
                path: PathBuf::new(),
                entries: sources_list::SourceListType::SourceLine(
                    SOURCE_LIST
                        .parse::<SourceListLineStyle>()
                        .expect("source list gen"),
                ),
            },
            SourcesList {
                path: PathBuf::new(),
                entries: sources_list::SourceListType::SourceLine(
                    POP_PPA_DISABLED.parse::<SourceListLineStyle>().expect("pop ppa gen"),
                ),
            },
        ],
    }
}

#[test]
fn disable_sources() {
    let mut lists = sources_lists();

    lists.repo_modify("http://apt.pop-os.org/proprietary", false);
    let proprietary = lists
        .entries()
        .find(|e| e.url == "http://apt.pop-os.org/proprietary")
        .expect("failed to find proprietary PPA");

    assert!(!proprietary.enabled);
    assert_eq!(
        "# deb http://apt.pop-os.org/proprietary disco main",
        &format!("{}", proprietary)
    );
}

#[test]
fn enable_sources() {
    let mut lists = sources_lists_pop_disabled();

    lists.repo_modify("http://apt.pop-os.org/proprietary", true);
    let proprietary = lists
        .entries()
        .find(|e| e.url == "http://apt.pop-os.org/proprietary")
        .expect("failed to find proprietary PPA");

    assert!(proprietary.enabled);
    assert_eq!(
        "deb http://apt.pop-os.org/proprietary disco main",
        &format!("{}", proprietary)
    );
}

#[test]
fn binary() {
    assert_eq!(
        SourceLine::from_str(
            "deb http://us.archive.ubuntu.com/ubuntu/ cosmic main \
             restricted universe multiverse"
        )
        .unwrap(),
        SourceLine::Entry(SourceEntry {
            enabled: true,
            source: false,
            url: "http://us.archive.ubuntu.com/ubuntu/".into(),
            suite: "cosmic".into(),
            options: vec![],
            components: vec![
                "main".into(),
                "restricted".into(),
                "universe".into(),
                "multiverse".into(),
            ],
            is_deb822: false,
            archs: None,
            signed_by: None,
            trusted: false,
        })
    );
}

#[test]
fn source() {
    assert_eq!(
        SourceLine::from_str(
            "deb-src http://us.archive.ubuntu.com/ubuntu/ cosmic main \
             restricted universe multiverse"
        )
        .unwrap(),
        SourceLine::Entry(SourceEntry {
            enabled: true,
            source: true,
            url: "http://us.archive.ubuntu.com/ubuntu/".into(),
            suite: "cosmic".into(),
            options: vec![],
            components: vec![
                "main".into(),
                "restricted".into(),
                "universe".into(),
                "multiverse".into(),
            ],
            is_deb822: false,
            archs: None,
            signed_by: None,
            trusted: false,
        })
    );
}

#[test]
fn fluff() {
    let comment = "# deb-src http://us.archive.ubuntu.com/ubuntu/ cosmic main \
                   restricted universe multiverse";
    assert_eq!(
        SourceLine::from_str(comment).unwrap(),
        SourceLine::Comment(comment.into())
    );

    assert_eq!(SourceLine::from_str("").unwrap(), SourceLine::Empty);
}

#[test]
fn options() {
    let options = [
        "deb [ arch=amd64 ] http://apt.pop-os.org/proprietary cosmic main",
        "deb [arch=amd64 ] http://apt.pop-os.org/proprietary cosmic main",
        "deb [ arch=amd64] http://apt.pop-os.org/proprietary cosmic main",
        "deb [arch=amd64]http://apt.pop-os.org/proprietary cosmic main",
        "deb [ arch=amd64 ]http://apt.pop-os.org/proprietary cosmic main",
    ];

    for source in &options {
        assert_eq!(
            SourceLine::from_str(source).unwrap(),
            SourceLine::Entry(SourceEntry {
                enabled: true,
                source: false,
                url: "http://apt.pop-os.org/proprietary".into(),
                suite: "cosmic".into(),
                options: vec![],
                components: vec!["main".into()],
                is_deb822: false,
                archs: Some(vec!["amd64".to_string()]),
                signed_by: None,
                trusted: false,
            })
        )
    }

    let options = [
        "deb [arch=amd64 signed-by=/usr/share/keyrings/termius-2023.gpg,/usr/share/keyrings/termius-2026.gpg a=b] https://deb.termius.com squeeze main",
        "deb [ arch=amd64 signed-by=/usr/share/keyrings/termius-2023.gpg,/usr/share/keyrings/termius-2026.gpg a=b] https://deb.termius.com squeeze main",
        "deb [ arch=amd64 signed-by=/usr/share/keyrings/termius-2023.gpg,/usr/share/keyrings/termius-2026.gpg a=b ] https://deb.termius.com squeeze main",
        "deb [arch=amd64 signed-by=/usr/share/keyrings/termius-2023.gpg,/usr/share/keyrings/termius-2026.gpg a=b ] https://deb.termius.com squeeze main"
    ];

    for source in &options {
        assert_eq!(
            SourceLine::from_str(source).unwrap(),
            SourceLine::Entry(SourceEntry {
                enabled: true,
                source: false,
                url: "https://deb.termius.com".into(),
                suite: "squeeze".into(),
                options: vec![("a".to_string(), vec!["b".to_string()])],
                components: vec!["main".into()],
                is_deb822: false,
                archs: Some(vec!["amd64".to_string()]),
                signed_by: Some(Signature::KeyPath(vec![
                    "/usr/share/keyrings/termius-2023.gpg".into(),
                    "/usr/share/keyrings/termius-2026.gpg".into()
                ]),),
                trusted: false,
            })
        )
    }
}
