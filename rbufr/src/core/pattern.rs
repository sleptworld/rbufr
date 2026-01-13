use anyhow::{Context, Result};
use regex::Regex;
use std::path::{Path, PathBuf};

/// Represents the type of BUFR table
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TableKind {
    B,
    D,
}

impl TableKind {
    pub fn as_str(&self) -> &str {
        match self {
            TableKind::B => "b",
            TableKind::D => "d",
        }
    }
}

/// Metadata extracted from a table filename
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TableMetadata {
    /// Table type (B or D)
    pub kind: TableKind,
    /// Version number (e.g., 35 for BUFRCREX_TableB_en_35.csv)
    pub version: Option<u32>,
    /// Subcenter ID (for local tables)
    pub subcenter: Option<u32>,
    /// Originating center ID (for local tables)
    pub center: Option<u32>,
    /// Language code (e.g., "en")
    pub language: Option<String>,
    /// Whether this is a local table
    pub is_local: bool,
    /// Original filename
    pub filename: String,
}

impl TableMetadata {
    /// Generate an output filename based on metadata
    ///
    /// Naming rules:
    /// - WMO standard tables: BUFR_TableB_{version} or BUFR_TableD_{version}
    ///   Example: BUFR_TableB_14
    ///
    /// - Local tables with subcenter: BUFR_TableB_{subcenter}_{version}
    ///   Example: BUFR_TableB_1_14 (subcenter 1, version 14)
    pub fn output_name(&self) -> String {
        let kind = match self.kind {
            TableKind::B => "TableB",
            TableKind::D => "TableD",
        };

        if self.is_local && self.subcenter.is_some() {
            // Format: BUFR_Table{B|D}_{subcenter}_{version}
            let subcenter = self.subcenter.unwrap();
            let version = self.version.unwrap_or(0);
            format!("BUFR_{}_{}_{}", kind, subcenter, version)
        } else {
            // Format: BUFR_Table{B|D}_{version}
            let version = self.version.unwrap_or(0);
            format!("BUFR_{}_{}", kind, version)
        }
    }
}

/// A pattern for matching table filenames
pub trait TableFilePattern: Send + Sync {
    /// Try to match a filename and extract metadata
    fn matches(&self, filename: &str) -> Option<TableMetadata>;

    /// Get a glob pattern for scanning directories
    fn glob_pattern(&self) -> &str;

    /// Get a description of this pattern
    fn description(&self) -> &str;
}

/// WMO standard table pattern
/// Examples:
/// - BUFRCREX_TableB_en_35.csv
/// - BUFR_TableD_en_40.csv
#[derive(Debug)]
pub struct WMOPattern {
    regex: Regex,
}

impl Default for WMOPattern {
    fn default() -> Self {
        Self::new()
    }
}

impl WMOPattern {
    pub fn new() -> Self {
        // Pattern: (BUFR|BUFRCREX)_Table(B|D)_([a-z]{2})_(\d+)\.csv
        let regex = Regex::new(r"^(?:BUFR(?:CREX)?)_Table([BD])_([a-z]{2})_(\d+)\.csv$")
            .expect("Invalid regex");

        Self { regex }
    }
}

impl TableFilePattern for WMOPattern {
    fn matches(&self, filename: &str) -> Option<TableMetadata> {
        let caps = self.regex.captures(filename)?;

        let kind = match &caps[1] {
            "B" => TableKind::B,
            "D" => TableKind::D,
            _ => return None,
        };

        let language = caps[2].to_string();
        let version = caps[3].parse().ok()?;

        Some(TableMetadata {
            kind,
            version: Some(version),
            subcenter: None,
            center: None,
            language: Some(language),
            is_local: false,
            filename: filename.to_string(),
        })
    }

    fn glob_pattern(&self) -> &str {
        "*Table[BD]_*.csv"
    }

    fn description(&self) -> &str {
        "WMO standard tables (BUFR_Table[BD]_en_*.csv)"
    }
}

/// Local table pattern
/// Examples:
/// - localtabb_85_20.csv (subcenter 85, version 20)
/// - localtabd_100_5.csv (subcenter 100, version 5)
#[derive(Debug)]
pub struct LocalPattern {
    regex: Regex,
}

impl Default for LocalPattern {
    fn default() -> Self {
        Self::new()
    }
}

impl LocalPattern {
    pub fn new() -> Self {
        // Pattern: localtab(b|d)_(\d+)_(\d+)\.csv
        let regex = Regex::new(r"^localtab([bd])_(\d+)_(\d+)\.csv$").expect("Invalid regex");

        Self { regex }
    }
}

impl TableFilePattern for LocalPattern {
    fn matches(&self, filename: &str) -> Option<TableMetadata> {
        let caps = self.regex.captures(filename)?;

        let kind = match &caps[1] {
            "b" => TableKind::B,
            "d" => TableKind::D,
            _ => return None,
        };

        let subcenter = caps[2].parse().ok()?;
        let version = caps[3].parse().ok()?;

        Some(TableMetadata {
            kind,
            version: Some(version),
            subcenter: Some(subcenter),
            center: None,
            language: None,
            is_local: true,
            filename: filename.to_string(),
        })
    }

    fn glob_pattern(&self) -> &str {
        "localtab[bd]_*.csv"
    }

    fn description(&self) -> &str {
        "Local tables (localtab[bd]_subcenter_version.csv)"
    }
}

pub struct OldMasterPattern {
    regex: Regex,
}

impl OldMasterPattern {
    pub fn new() -> Self {
        // Pattern: localtab(b|d)_(\d+)_(\d+)\.csv
        let regex = Regex::new(r"^bufrtab([bd])_(\d+)\.csv$").expect("Invalid regex");

        Self { regex }
    }
}

impl TableFilePattern for OldMasterPattern {
    fn matches(&self, filename: &str) -> Option<TableMetadata> {
        let caps = self.regex.captures(filename)?;

        let kind = match &caps[1] {
            "b" => TableKind::B,
            "d" => TableKind::D,
            _ => return None,
        };

        let version = caps[2].parse().ok()?;

        Some(TableMetadata {
            kind,
            version: Some(version),
            subcenter: None,
            center: None,
            is_local: false,
            language: None,
            filename: filename.to_string(),
        })
    }

    fn glob_pattern(&self) -> &str {
        "bufrtab[bd]_*.csv"
    }

    fn description(&self) -> &str {
        "Old master tables (bufrtab[bd]_version.csv)"
    }
}

/// Custom pattern with flexible center/subcenter
/// Examples:
/// - center_7_subcenter_85_tableb_v20.csv
/// - c7_sc85_d_v20.csv
#[derive(Debug)]
pub struct CustomPattern {
    regex: Regex,
}

impl Default for CustomPattern {
    fn default() -> Self {
        Self::new()
    }
}

impl CustomPattern {
    pub fn new() -> Self {
        // Pattern: .*_c(\d+)_sc(\d+)_([bd])_v?(\d+)\.csv
        let regex =
            Regex::new(r"(?i).*_?c(?:enter)?_?(\d+)_sc(?:enter)?_?(\d+)_table([bd])_v?(\d+)\.csv$")
                .expect("Invalid regex");

        Self { regex }
    }
}

impl TableFilePattern for CustomPattern {
    fn matches(&self, filename: &str) -> Option<TableMetadata> {
        let caps = self.regex.captures(filename)?;

        let center = caps[1].parse().ok()?;
        let subcenter = caps[2].parse().ok()?;

        let kind = match caps[3].to_lowercase().as_str() {
            "b" => TableKind::B,
            "d" => TableKind::D,
            _ => return None,
        };

        let version = caps[4].parse().ok()?;

        Some(TableMetadata {
            kind,
            version: Some(version),
            subcenter: Some(subcenter),
            center: Some(center),
            language: None,
            is_local: true,
            filename: filename.to_string(),
        })
    }

    fn glob_pattern(&self) -> &str {
        "*_c*_sc*_table*_*.csv"
    }

    fn description(&self) -> &str {
        "Custom center/subcenter tables (*_c{center}_sc{subcenter}_table[bd]_v{version}.csv)"
    }
}

/// Scanner that tries multiple patterns
pub struct TableScanner {
    patterns: Vec<Box<dyn TableFilePattern>>,
}

impl Default for TableScanner {
    fn default() -> Self {
        Self::new()
    }
}

impl TableScanner {
    pub fn new() -> Self {
        Self {
            patterns: vec![
                Box::new(WMOPattern::new()),
                Box::new(OldMasterPattern::new()),
                Box::new(LocalPattern::new()),
                Box::new(CustomPattern::new()),
            ],
        }
    }

    /// Create scanner with custom patterns
    pub fn with_patterns(patterns: Vec<Box<dyn TableFilePattern>>) -> Self {
        Self { patterns }
    }

    /// Add a pattern to the scanner
    pub fn add_pattern(&mut self, pattern: Box<dyn TableFilePattern>) {
        self.patterns.push(pattern);
    }

    /// Try to match a filename with any registered pattern
    pub fn match_filename(&self, filename: &str) -> Option<TableMetadata> {
        for pattern in &self.patterns {
            if let Some(metadata) = pattern.matches(filename) {
                return Some(metadata);
            }
        }
        None
    }

    /// Scan a directory for matching files
    pub fn scan_directory<P: AsRef<Path>>(
        &self,
        dir: P,
        kind_filter: Option<TableKind>,
    ) -> Result<Vec<(PathBuf, TableMetadata)>> {
        let dir = dir.as_ref();
        let mut results = Vec::new();

        // Try each pattern
        for pattern in &self.patterns {
            let glob_pattern = dir.join(pattern.glob_pattern());

            for entry in
                glob::glob(glob_pattern.to_str().unwrap()).context("Failed to read glob pattern")?
            {
                match entry {
                    Ok(path) => {
                        if let Some(filename) = path.file_name().and_then(|f| f.to_str()) {
                            if let Some(metadata) = pattern.matches(filename) {
                                // Apply kind filter if specified
                                if let Some(filter_kind) = kind_filter {
                                    if metadata.kind != filter_kind {
                                        continue;
                                    }
                                }

                                results.push((path, metadata));
                            }
                        }
                    }
                    Err(e) => {
                        eprintln!("Warning: Error reading file entry: {}", e);
                    }
                }
            }
        }

        // Remove duplicates (same file matched by multiple patterns)
        results.sort_by(|a, b| a.0.cmp(&b.0));
        results.dedup_by(|a, b| a.0 == b.0);

        Ok(results)
    }

    /// Get all registered patterns
    pub fn patterns(&self) -> &[Box<dyn TableFilePattern>] {
        &self.patterns
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_wmo_pattern() {
        let pattern = WMOPattern::new();

        // Valid WMO patterns
        let meta = pattern.matches("BUFRCREX_TableB_en_35.csv").unwrap();
        assert_eq!(meta.kind, TableKind::B);
        assert_eq!(meta.version, Some(35));
        assert_eq!(meta.language, Some("en".to_string()));
        assert!(!meta.is_local);

        let meta = pattern.matches("BUFR_TableD_en_40.csv").unwrap();
        assert_eq!(meta.kind, TableKind::D);
        assert_eq!(meta.version, Some(40));
        assert!(!meta.is_local);

        // Invalid patterns
        assert!(pattern.matches("BUFRCREX_TableB_35.csv").is_none());
        assert!(pattern.matches("TableB_en_35.csv").is_none());
    }

    #[test]
    fn test_local_pattern() {
        let pattern = LocalPattern::new();

        // Valid local patterns
        let meta = pattern.matches("localtabb_85_20.csv").unwrap();
        assert_eq!(meta.kind, TableKind::B);
        assert_eq!(meta.subcenter, Some(85));
        assert_eq!(meta.version, Some(20));
        assert!(meta.is_local);

        let meta = pattern.matches("localtabd_100_5.csv").unwrap();
        assert_eq!(meta.kind, TableKind::D);
        assert_eq!(meta.subcenter, Some(100));
        assert_eq!(meta.version, Some(5));
        assert!(meta.is_local);

        // Invalid patterns
        assert!(pattern.matches("local_table_85_20.csv").is_none());
        assert!(pattern.matches("localtabb_85.csv").is_none());
    }

    #[test]
    fn test_custom_pattern() {
        let pattern = CustomPattern::new();

        // Valid custom patterns
        let meta = pattern.matches("test_c7_sc85_tableb_v20.csv").unwrap();
        assert_eq!(meta.kind, TableKind::B);
        assert_eq!(meta.center, Some(7));
        assert_eq!(meta.subcenter, Some(85));
        assert_eq!(meta.version, Some(20));
        assert!(meta.is_local);

        let meta = pattern
            .matches("data_center_7_scenter_85_tabled_10.csv")
            .unwrap();
        assert_eq!(meta.kind, TableKind::D);
        assert_eq!(meta.center, Some(7));
        assert_eq!(meta.subcenter, Some(85));
        assert_eq!(meta.version, Some(10));
    }

    #[test]
    fn test_output_name_generation() {
        // WMO table (no subcenter) - Format: BUFR_TableB_{version}
        let meta = TableMetadata {
            kind: TableKind::B,
            version: Some(14),
            subcenter: None,
            center: None,
            language: Some("en".to_string()),
            is_local: false,
            filename: "BUFRCREX_TableB_en_14.csv".to_string(),
        };
        assert_eq!(meta.output_name(), "BUFR_TableB_14");

        // WMO Table D
        let meta = TableMetadata {
            kind: TableKind::D,
            version: Some(40),
            subcenter: None,
            center: None,
            language: Some("en".to_string()),
            is_local: false,
            filename: "BUFR_TableD_en_40.csv".to_string(),
        };
        assert_eq!(meta.output_name(), "BUFR_TableD_40");

        // Local table with subcenter - Format: BUFR_TableB_{subcenter}_{version}
        let meta = TableMetadata {
            kind: TableKind::B,
            version: Some(14),
            subcenter: Some(1),
            center: None,
            language: None,
            is_local: true,
            filename: "localtabb_1_14.csv".to_string(),
        };
        assert_eq!(meta.output_name(), "BUFR_TableB_1_14");

        // Local table with larger subcenter number
        let meta = TableMetadata {
            kind: TableKind::B,
            version: Some(20),
            subcenter: Some(85),
            center: None,
            language: None,
            is_local: true,
            filename: "localtabb_85_20.csv".to_string(),
        };
        assert_eq!(meta.output_name(), "BUFR_TableB_85_20");
    }

    #[test]
    fn test_scanner() {
        let scanner = TableScanner::new();

        // Should match WMO pattern
        let meta = scanner.match_filename("BUFRCREX_TableB_en_35.csv").unwrap();
        assert_eq!(meta.kind, TableKind::B);
        assert!(!meta.is_local);

        // Should match local pattern
        let meta = scanner.match_filename("localtabb_85_20.csv").unwrap();
        assert_eq!(meta.kind, TableKind::B);
        assert!(meta.is_local);

        // Should match custom pattern
        let meta = scanner
            .match_filename("test_c7_sc85_tableb_v20.csv")
            .unwrap();
        assert_eq!(meta.kind, TableKind::B);
        assert!(meta.is_local);
    }
}
