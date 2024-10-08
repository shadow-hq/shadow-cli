use std::fmt::Display;

use eyre::Result;
use serde_json::Value;

static APP_USER_AGENT: &str = concat!(env!("CARGO_PKG_NAME"), "/", env!("CARGO_PKG_VERSION"),);

/// Versioning
#[derive(Debug)]
pub struct Version {
    major: u32,
    minor: u32,
    patch: u32,
    channel: Option<String>,
}

/// get the current version from cargo
pub fn current_version() -> Version {
    // get the current version from the cargo package
    let version_string = env!("CARGO_PKG_VERSION");

    // remove +<channel>... from the version string
    let version_channel =
        version_string.split('+').collect::<Vec<&str>>().get(1).map(|s| s.to_string());
    let version_string = version_string.split('+').collect::<Vec<&str>>()[0];
    let version_parts = version_string.split('.').collect::<Vec<&str>>();

    Version {
        major: version_parts[0].parse::<u32>().unwrap_or(0),
        minor: version_parts[1].parse::<u32>().unwrap_or(0),
        patch: version_parts[2].parse::<u32>().unwrap_or(0),
        channel: version_channel,
    }
}

/// get the latest version from github
pub async fn remote_version() -> Result<Version> {
    // retrieve the latest release tag from github
    let client = reqwest::Client::builder()
        .user_agent(APP_USER_AGENT)
        .danger_accept_invalid_certs(true)
        .build()?;
    let res = client
        .get("https://api.github.com/repos/shadow-hq/shadow-cli/releases/latest")
        .send()
        .await?;
    let release: Value = res.json().await?;

    if let Some(tag_name) = release["tag_name"].as_str() {
        let version_string = tag_name.replace('v', "");
        let version_parts: Vec<&str> = version_string.split('.').collect();

        if version_parts.len() == 3 {
            let major = version_parts[0].parse::<u32>().unwrap_or(0);
            let minor = version_parts[1].parse::<u32>().unwrap_or(0);
            let patch = version_parts[2].parse::<u32>().unwrap_or(0);

            return Ok(Version { major, minor, patch, channel: None });
        }
    }

    // if we can't get the latest release, return a default version
    Ok(Version { major: 0, minor: 0, patch: 0, channel: None })
}

/// get the latest nightly version from github
pub async fn remote_nightly_version() -> Result<Version> {
    // get the latest release
    let mut remote_ver = remote_version().await?;

    let client = reqwest::Client::builder()
        .user_agent(APP_USER_AGENT)
        .danger_accept_invalid_certs(true)
        .build()?;
    let res =
        client.get("https://api.github.com/repos/shadow-hq/shadow-cli/commits/main").send().await?;
    let commit: Value = res.json().await?;

    // get the latest commit hash
    if let Some(sha) = commit["sha"].as_str() {
        // channel is nightly.1234567
        remote_ver.channel = format!("nightly.{}", &sha[..7]).into();
    }

    Ok(remote_ver)
}

impl Display for Version {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let version_string = format!("{}.{}.{}{}", self.major, self.minor, self.patch, {
            if let Some(channel) = &self.channel {
                format!("+{}", channel)
            } else {
                "".to_string()
            }
        });
        write!(f, "{}", version_string)
    }
}

impl PartialEq for Version {
    fn eq(&self, other: &Version) -> bool {
        self.major == other.major &&
            self.minor == other.minor &&
            self.patch == other.patch &&
            self.channel == other.channel
    }
}

impl Version {
    /// greater than
    pub fn gt(&self, other: &Version) -> bool {
        self.major > other.major ||
            (self.major == other.major && self.minor > other.minor) ||
            (self.major == other.major && self.minor == other.minor && self.patch > other.patch)
    }

    /// greater than or equal to
    pub fn gte(&self, other: &Version) -> bool {
        self.major > other.major ||
            (self.major == other.major && self.minor > other.minor) ||
            (self.major == other.major && self.minor == other.minor && self.patch >= other.patch)
    }

    /// less than
    pub fn lt(&self, other: &Version) -> bool {
        self.major < other.major ||
            (self.major == other.major && self.minor < other.minor) ||
            (self.major == other.major && self.minor == other.minor && self.patch < other.patch)
    }

    /// less than or equal to
    pub fn lte(&self, other: &Version) -> bool {
        self.major < other.major ||
            (self.major == other.major && self.minor < other.minor) ||
            (self.major == other.major && self.minor == other.minor && self.patch <= other.patch)
    }

    /// not equal to
    pub fn ne(&self, other: &Version) -> bool {
        self.major != other.major ||
            self.minor != other.minor ||
            self.patch != other.patch ||
            self.channel != other.channel
    }

    /// if the version is a nightly version
    pub fn is_nightly(&self) -> bool {
        self.channel.is_some() && self.channel.as_ref().unwrap().starts_with("nightly.")
    }
}

#[cfg(test)]
mod tests {
    use crate::utils::version::*;

    #[test]
    fn test_greater_than() {
        let v1 = Version { major: 2, minor: 3, patch: 4, channel: None };
        let v2 = Version { major: 2, minor: 3, patch: 3, channel: None };
        let v3 = Version { major: 2, minor: 2, patch: 5, channel: None };
        let v4 = Version { major: 1, minor: 4, patch: 4, channel: None };

        assert!(v1.gt(&v2));
        assert!(v1.gt(&v3));
        assert!(v1.gt(&v4));
        assert!(!v2.gt(&v1));
        assert!(!v1.gt(&v1));
    }

    #[test]
    fn test_greater_than_or_equal_to() {
        let v1 = Version { major: 2, minor: 3, patch: 4, channel: None };
        let v2 = Version { major: 2, minor: 3, patch: 4, channel: None };

        assert!(v1.gte(&v2));
        assert!(v2.gte(&v1));
        assert!(v1.gte(&Version { major: 1, minor: 0, patch: 0, channel: None }));
    }

    #[test]
    fn test_less_than() {
        let v1 = Version { major: 2, minor: 3, patch: 4, channel: None };
        let v2 = Version { major: 2, minor: 3, patch: 5, channel: None };
        let v3 = Version { major: 2, minor: 4, patch: 4, channel: None };
        let v4 = Version { major: 3, minor: 3, patch: 4, channel: None };

        assert!(v1.lt(&v2));
        assert!(v1.lt(&v3));
        assert!(v1.lt(&v4));
        assert!(!v2.lt(&v1));
        assert!(!v1.lt(&v1));
    }

    #[test]
    fn test_less_than_or_equal_to() {
        let v1 = Version { major: 2, minor: 3, patch: 4, channel: None };
        let v2 = Version { major: 2, minor: 3, patch: 4, channel: None };

        assert!(v1.lte(&v2));
        assert!(v2.lte(&v1));
        assert!(v1.lte(&Version { major: 3, minor: 0, patch: 0, channel: None }));
    }

    #[test]
    fn test_equal_to() {
        let v1 = Version { major: 2, minor: 3, patch: 4, channel: None };
        let v2 = Version { major: 2, minor: 3, patch: 4, channel: None };
        let v3 = Version { major: 2, minor: 3, patch: 5, channel: None };

        assert!(v1.eq(&v2));
        assert!(!v1.eq(&v3));
    }

    #[test]
    fn test_not_equal_to() {
        let v1 = Version { major: 2, minor: 3, patch: 4, channel: None };
        let v2 = Version { major: 2, minor: 3, patch: 5, channel: None };
        let v3 = Version { major: 3, minor: 3, patch: 4, channel: None };

        assert!(v1.ne(&v2));
        assert!(v1.ne(&v3));
        assert!(!v1.ne(&Version { major: 2, minor: 3, patch: 4, channel: None }));
    }

    #[test]
    fn test_version_display() {
        let version = Version { major: 2, minor: 3, patch: 4, channel: None };

        assert_eq!(version.to_string(), "2.3.4");
    }

    #[test]
    fn test_version_current() {
        let version = current_version();

        assert_eq!(version.to_string(), env!("CARGO_PKG_VERSION"));
    }

    #[tokio::test]
    async fn test_version_remote() {
        let version = remote_version().await;

        assert!(version.is_ok());
    }

    #[tokio::test]
    async fn test_version_remote_nightly() {
        let version = remote_nightly_version().await;

        assert!(version.is_ok());
    }
}
