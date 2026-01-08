use anyhow::{Context, Result};
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::path::Path;

use crate::pattern::{TableFilePattern, TableKind, TableMetadata};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PatternConfig {
    pub name: String,
    pub regex: String,
    pub glob: String,
    pub mapping: FieldMapping,
}

/// Defines which capture group corresponds to which metadata field
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FieldMapping {
    /// Capture group index for table kind (B or D)
    pub kind_group: usize,

    /// Optional capture group index for version
    pub version_group: Option<usize>,

    /// Optional capture group index for subcenter
    pub subcenter_group: Option<usize>,

    /// Optional capture group index for center
    pub center_group: Option<usize>,

    /// Optional capture group index for language
    pub language_group: Option<usize>,

    /// Whether this pattern matches local tables
    pub is_local: bool,
}

/// Runtime pattern compiled from configuration
pub struct ConfigurablePattern {
    name: String,
    regex: Regex,
    glob: String,
    mapping: FieldMapping,
}

impl ConfigurablePattern {
    pub fn from_config(config: &PatternConfig) -> Result<Self> {
        let regex = Regex::new(&config.regex)
            .with_context(|| format!("Invalid regex pattern: {}", config.regex))?;

        Ok(Self {
            name: config.name.clone(),
            regex,
            glob: config.glob.clone(),
            mapping: config.mapping.clone(),
        })
    }
}

impl TableFilePattern for ConfigurablePattern {
    fn matches(&self, filename: &str) -> Option<TableMetadata> {
        let caps = self.regex.captures(filename)?;

        // Extract table kind
        let kind_str = caps.get(self.mapping.kind_group)?.as_str();
        let kind = match kind_str.to_lowercase().as_str() {
            "b" => TableKind::B,
            "d" => TableKind::D,
            _ => return None,
        };

        // Extract version
        let version = if let Some(idx) = self.mapping.version_group {
            caps.get(idx).and_then(|m| m.as_str().parse().ok())
        } else {
            None
        };

        // Extract subcenter
        let subcenter = if let Some(idx) = self.mapping.subcenter_group {
            caps.get(idx).and_then(|m| m.as_str().parse().ok())
        } else {
            None
        };

        // Extract center
        let center = if let Some(idx) = self.mapping.center_group {
            caps.get(idx).and_then(|m| m.as_str().parse().ok())
        } else {
            None
        };

        // Extract language
        let language = if let Some(idx) = self.mapping.language_group {
            caps.get(idx).map(|m| m.as_str().to_string())
        } else {
            None
        };

        Some(TableMetadata {
            kind,
            version,
            subcenter,
            center,
            language,
            is_local: self.mapping.is_local,
            filename: filename.to_string(),
        })
    }

    fn glob_pattern(&self) -> &str {
        &self.glob
    }

    fn description(&self) -> &str {
        &self.name
    }
}

/// Full configuration file structure
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ScanConfig {
    /// List of custom patterns
    #[serde(default)]
    pub patterns: Vec<PatternConfig>,
}

impl ScanConfig {
    /// Load configuration from a TOML file
    pub fn load_from_file<P: AsRef<Path>>(path: P) -> Result<Self> {
        let content = std::fs::read_to_string(path.as_ref())
            .with_context(|| format!("Failed to read config file: {}", path.as_ref().display()))?;

        let config: ScanConfig = toml::from_str(&content)
            .with_context(|| format!("Failed to parse config file: {}", path.as_ref().display()))?;

        Ok(config)
    }

    /// Create default configuration with example patterns
    pub fn default_example() -> Self {
        Self {
            patterns: vec![
                PatternConfig {
                    name: "ECMWF local tables".to_string(),
                    regex: r"^ecmwf_table([bd])_v(\d+)\.csv$".to_string(),
                    glob: "ecmwf_table*.csv".to_string(),
                    mapping: FieldMapping {
                        kind_group: 1,
                        version_group: Some(2),
                        subcenter_group: None,
                        center_group: None,
                        language_group: None,
                        is_local: true,
                    },
                },
                PatternConfig {
                    name: "NCEP local tables".to_string(),
                    regex: r"^ncep_bufrtab\.(\d+)\.([bd])$".to_string(),
                    glob: "ncep_bufrtab.*".to_string(),
                    mapping: FieldMapping {
                        kind_group: 2,
                        version_group: Some(1),
                        subcenter_group: None,
                        center_group: None,
                        language_group: None,
                        is_local: true,
                    },
                },
            ],
        }
    }

    /// Save configuration to a TOML file
    pub fn save_to_file<P: AsRef<Path>>(&self, path: P) -> Result<()> {
        let content = toml::to_string_pretty(self).context("Failed to serialize config")?;

        std::fs::write(path.as_ref(), content)
            .with_context(|| format!("Failed to write config file: {}", path.as_ref().display()))?;

        Ok(())
    }

    /// Compile all patterns from this configuration
    pub fn compile_patterns(&self) -> Result<Vec<Box<dyn TableFilePattern>>> {
        let mut patterns: Vec<Box<dyn TableFilePattern>> = Vec::new();

        for config in &self.patterns {
            let pattern = ConfigurablePattern::from_config(config)
                .with_context(|| format!("Failed to compile pattern: {}", config.name))?;
            patterns.push(Box::new(pattern));
        }

        Ok(patterns)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_configurable_pattern() {
        let config = PatternConfig {
            name: "Test pattern".to_string(),
            regex: r"^test_table([bd])_v(\d+)\.csv$".to_string(),
            glob: "test_table*.csv".to_string(),
            mapping: FieldMapping {
                kind_group: 1,
                version_group: Some(2),
                subcenter_group: None,
                center_group: None,
                language_group: None,
                is_local: true,
            },
        };

        let pattern = ConfigurablePattern::from_config(&config).unwrap();

        let meta = pattern.matches("test_tableb_v20.csv").unwrap();
        assert_eq!(meta.kind, TableKind::B);
        assert_eq!(meta.version, Some(20));
        assert!(meta.is_local);

        let meta = pattern.matches("test_tabled_v15.csv").unwrap();
        assert_eq!(meta.kind, TableKind::D);
        assert_eq!(meta.version, Some(15));
    }

    #[test]
    fn test_config_serialization() {
        let config = ScanConfig::default_example();
        let toml_str = toml::to_string_pretty(&config).unwrap();

        println!("Example config:\n{}", toml_str);

        let parsed: ScanConfig = toml::from_str(&toml_str).unwrap();
        assert_eq!(parsed.patterns.len(), config.patterns.len());
    }
}
