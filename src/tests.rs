pub use super::*;
use std::str::FromStr;

#[test]
fn binary() {
    assert_eq!(
        SourceLine::from_str(
            "deb http://us.archive.ubuntu.com/ubuntu/ cosmic main \
             restricted universe multiverse"
        )
        .unwrap(),
        SourceLine::Entry(SourceEntry {
            source: false,
            url: "http://us.archive.ubuntu.com/ubuntu/".into(),
            suite: "cosmic".into(),
            options: None,
            components: vec![
                "main".into(),
                "restricted".into(),
                "universe".into(),
                "multiverse".into(),
            ]
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
            source: true,
            url: "http://us.archive.ubuntu.com/ubuntu/".into(),
            suite: "cosmic".into(),
            options: None,
            components: vec![
                "main".into(),
                "restricted".into(),
                "universe".into(),
                "multiverse".into(),
            ]
        })
    );
}

#[test]
fn fluff() {
    let comment = "# deb-src http://us.archive.ubuntu.com/ubuntu/ cosmic main \
                   restricted universe multiverse";
    assert_eq!(SourceLine::from_str(comment).unwrap(), SourceLine::Comment(comment.into()));

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
                source: false,
                url: "http://apt.pop-os.org/proprietary".into(),
                suite: "cosmic".into(),
                options: Some("arch=amd64".into()),
                components: vec!["main".into()]
            })
        )
    }
}
