use super::*;
use std::fmt;
use std::str::FromStr;

/// An apt source entry that is active on the system.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct SourceEntry {
    /// Whether the entry is enabled or not.
    pub enabled: bool,
    /// Whether this is a binary or source repo.
    pub source: bool,
    /// Some repos may have special options defined.
    pub options: Option<String>,
    /// The URL of the repo.
    pub url: String,
    /// The suite of the repo would be as `bionic` or `cosmic`.
    pub suite: String,
    /// Components that have been enabled for this repo.
    pub components: Vec<String>,
}

impl fmt::Display for SourceEntry {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        if !self.enabled {
            fmt.write_str("# ")?;
        }

        fmt.write_str(if self.source { "deb-src " } else { "deb " })?;
        if let Some(ref options) = self.options.as_ref() {
            write!(fmt, "[{}] ", options)?;
        }

        write!(fmt, "{} {} {}", self.url, self.suite, self.components.join(" "))
    }
}

impl FromStr for SourceEntry {
    type Err = SourceError;
    fn from_str(line: &str) -> Result<Self, Self::Err> {
        let mut components = Vec::new();
        let mut options = None;
        let url;

        let mut fields = line.split_whitespace();

        let source = match fields.next().ok_or(SourceError::MissingField { field: "source" })? {
            "deb" => false,
            "deb-src" => true,
            other => {
                return Err(SourceError::InvalidValue { field: "source", value: other.to_owned() })
            }
        };

        let field = fields.next().ok_or(SourceError::MissingField { field: "url" })?;
        if field.starts_with('[') {
            let mut leftover: Option<String> = None;
            let mut field: String = field[1..].into();

            if let Some(pos) = field.find(']') {
                if pos == field.len() - 1 {
                    options = Some(field[..pos].into());
                } else {
                    options = Some(field[..pos].into());
                    leftover = Some(field[pos + 1..].into());
                }
            } else {
                loop {
                    let next =
                        fields.next().ok_or(SourceError::MissingField { field: "option" })?;
                    if let Some(pos) = next.find(']') {
                        field.push_str(&next[..pos]);
                        if pos != next.len() - 1 {
                            leftover = Some(next[pos + 1..].into());
                        }
                        break;
                    } else {
                        field.push_str(next);
                    }
                }

                options = Some(field);
            }

            url = match leftover {
                Some(field) => field,
                None => fields.next().ok_or(SourceError::MissingField { field: "url" })?.into(),
            };
        } else {
            url = field.into();
        }

        if options.as_ref().map_or(false, String::is_empty) {
            options = None;
        }

        let suite = fields.next().ok_or(SourceError::MissingField { field: "suite" })?.into();

        for field in fields {
            components.push(field.into());
        }

        Ok(SourceEntry { enabled: true, source, url, suite, components, options })
    }
}

impl SourceEntry {
    pub fn url(&self) -> &str {
        let mut url: &str = &self.url;
        while url.ends_with('/') {
            url = &url[..url.len() - 1];
        }

        url
    }

    /// The base filename to be used when storing files for this entries.
    pub fn filename(&self) -> String {
        let mut url = self.url();
        if let Some(pos) = url.find("//") {
            url = &url[pos..];
        }

        url.replace("/", "_")
    }

    /// Returns the root URL for this entry's dist path.
    ///
    /// For an entry such as:
    ///
    /// ```toml
    /// deb http://us.archive.ubuntu.com/ubuntu/ cosmic main
    /// ```
    ///
    /// The path that will be returned will be:
    ///
    /// ```toml
    /// http://us.archive.ubuntu.com/ubuntu/dists/cosmic
    /// ```
    pub fn dist_path(&self) -> String {
        [self.url(), "/dists/", &self.suite].concat()
    }

    pub fn dist_path_get(&self, path: &str) -> String {
        let url = self.url();
        [url, "/dists/", &self.suite, "/", path].concat()
    }

    /// Iterator that returns each of the dist components that are to be fetched.
    pub fn dist_components<'a>(&'a self) -> impl Iterator<Item = String> + 'a {
        let url = self.url();
        self.components
            .iter()
            .map(move |component| [url, "/dists/", &self.suite, "/", &component].concat())
    }

    /// Returns the root URL for this entry's pool path.
    ///
    /// For an entry such as:
    ///
    /// ```toml
    /// deb http://us.archive.ubuntu.com/ubuntu/ cosmic main
    /// ```
    ///
    /// The path that will be returned will be:
    ///
    /// ```toml
    /// http://us.archive.ubuntu.com/ubuntu/pool/cosmic
    /// ```
    pub fn pool_path(&self) -> String {
        [self.url(), "/pool/"].concat()
    }
}
