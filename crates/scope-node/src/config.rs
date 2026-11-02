//! Strict configuration and explicitly masked diagnostics.
use crate::{detect::DetectSettings, Settings};
use serde::{Deserialize, Serialize};
use std::{fmt, net::SocketAddr, path::Path};

#[derive(Clone, Copy, Debug, Default, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Adapter {
    #[default]
    Rpicam,
    Simulated,
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize)]
#[serde(untagged)]
pub enum Control {
    #[default]
    Auto,
    Fixed(f64),
}
impl<'de> Deserialize<'de> for Control {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        #[derive(Deserialize)]
        #[serde(untagged)]
        enum Value {
            Number(f64),
            Text(String),
        }
        match Value::deserialize(deserializer)? {
            Value::Number(value) if value.is_finite() && value > 0.0 => Ok(Self::Fixed(value)),
            Value::Text(value) if value == "auto" => Ok(Self::Auto),
            _ => Err(serde::de::Error::custom(
                "expected a positive finite number or 'auto'",
            )),
        }
    }
}
impl Control {
    pub fn value(self) -> Option<f64> {
        match self {
            Self::Auto => None,
            Self::Fixed(value) => Some(value),
        }
    }
}

/// Optional serial mount; unknown keys are errors just as in the camera config.
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct MountConfig {
    pub device: std::path::PathBuf,
}

/// Token-bearing config deliberately has no Debug/Serialize implementation.
#[derive(Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Config {
    #[serde(default = "default_listen")]
    pub listen: SocketAddr,
    pub token: String,
    #[serde(default)]
    pub adapter: Adapter,
    #[serde(default)]
    pub exposure_ms: Control,
    #[serde(default)]
    pub gain: Control,
    #[serde(default)]
    pub detection: DetectSettings,
    #[serde(default)]
    pub mount: Option<MountConfig>,
}
fn default_listen() -> SocketAddr {
    SocketAddr::from(([0, 0, 0, 0], 50051))
}

#[derive(Debug)]
pub struct ConfigError(String);
impl fmt::Display for ConfigError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}
impl std::error::Error for ConfigError {}

impl Config {
    pub fn parse(source: &str) -> Result<Self, ConfigError> {
        let config: Self = toml::from_str(source).map_err(|error: toml::de::Error| {
            // TOML's Display includes the entire source line, which might hold a
            // token. Report only a key and line number, never the source/value.
            let offset = error.span().map_or(0, |span| span.start.min(source.len()));
            let line_no = source[..offset].bytes().filter(|b| *b == b'\n').count() + 1;
            let line = source.lines().nth(line_no - 1).unwrap_or_default();
            let key = line
                .split_once('=')
                .map(|(key, _)| key.trim())
                .filter(|key| {
                    !key.is_empty()
                        && key
                            .chars()
                            .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '.')
                })
                .unwrap_or("configuration");
            // A missing required field is not located on a key's source line.
            let missing = if error.message().contains("missing field `token`") {
                " (missing token)"
            } else if error.message().contains("missing field `device`") {
                " (missing mount.device)"
            } else {
                ""
            };
            ConfigError(format!("invalid {key} at line {line_no}{missing}"))
        })?;
        config.validate()?;
        Ok(config)
    }
    pub fn load(path: &Path) -> Result<Self, ConfigError> {
        let source = std::fs::read_to_string(path)
            .map_err(|e| ConfigError(format!("cannot read config {}: {e}", path.display())))?;
        Self::parse(&source)
    }
    fn validate(&self) -> Result<(), ConfigError> {
        if self
            .mount
            .as_ref()
            .is_some_and(|mount| mount.device.as_os_str().is_empty())
        {
            return Err(ConfigError("mount.device must not be empty".into()));
        }
        if self.token.trim().is_empty() {
            return Err(ConfigError("token must not be empty".into()));
        }
        if self.listen.port() == 0 {
            return Err(ConfigError("listen must have a nonzero port".into()));
        }
        if !self.detection.sigma.is_finite() || !(1.0..=50.0).contains(&self.detection.sigma) {
            return Err(ConfigError(
                "detection.sigma must be between 1 and 50".into(),
            ));
        }
        if ![1, 2, 4, 8].contains(&self.detection.binning) {
            return Err(ConfigError("detection.binning must be 1, 2, 4 or 8".into()));
        }
        if self.detection.noise_estimate.is_some()
            || self.detection.return_binned
            || !self.detection.use_binned_for_star_candidates
        {
            return Err(ConfigError("detection.noise_estimate, return_binned and use_binned_for_star_candidates must retain cedar defaults".into()));
        }
        Ok(())
    }
    pub fn settings(&self) -> Settings {
        Settings {
            exposure_ms: self.exposure_ms.value(),
            gain: self.gain.value(),
            detection: self.detection.clone(),
        }
    }
    /// TOML which can be inspected without exposing the pairing token.
    pub fn masked(&self) -> String {
        let control = |value: Control| match value {
            Control::Auto => "\"auto\"".into(),
            Control::Fixed(n) => n.to_string(),
        };
        format!("listen = \"{}\"\ntoken = \"****\"\nadapter = \"{}\"\nexposure_ms = {}\ngain = {}\n\n[detection]\n{}", self.listen, match self.adapter { Adapter::Rpicam => "rpicam", Adapter::Simulated => "simulated" }, control(self.exposure_ms), control(self.gain), toml::to_string(&self.detection).expect("validated detection serializes"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn minimal_config_uses_automatic_controls_and_cedar_defaults() {
        let config = Config::parse("token = 'test-token'").unwrap();
        assert_eq!(config.adapter, Adapter::Rpicam);
        assert_eq!(config.listen, default_listen());
        assert_eq!(config.settings(), Settings::default());
        assert!(config.masked().contains("****"));
        assert!(!config.masked().contains("test-token"));
    }
    #[test]
    fn strict_values_and_nested_keys_fail_with_key_names_without_token() {
        for (line, key) in [
            ("gain = -1", "gain"),
            ("exposure_ms = nan", "exposure_ms"),
            ("adapter = 'usb'", "adapter"),
            ("listen = 'bad'", "listen"),
            ("listen = '127.0.0.1:0'", "listen"),
            ("[detection]\nsigma = 0", "sigma"),
            ("[detection]\nbinning = 3", "binning"),
            ("[detection]\nsigam = 8", "sigam"),
        ] {
            let error = Config::parse(&format!("token = 'test-token'\n{line}"))
                .err()
                .unwrap()
                .to_string();
            assert!(error.contains(key), "{error}");
            assert!(!error.contains("test-token"));
        }
        assert!(Config::parse("")
            .err()
            .unwrap()
            .to_string()
            .contains("token"));
        assert!(Config::parse("token = ''").is_err());
    }
}
