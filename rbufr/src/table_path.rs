use std::path::{Path, PathBuf};
use std::sync::OnceLock;

static TABLES_BASE_PATH: OnceLock<PathBuf> = OnceLock::new();

pub fn set_tables_base_path<P: AsRef<Path>>(path: P) {
    let _ = TABLES_BASE_PATH.set(path.as_ref().to_path_buf());
}

pub fn get_tables_base_path() -> PathBuf {
    if let Some(path) = TABLES_BASE_PATH.get() {
        return path.clone();
    }

    if let Ok(env_path) = std::env::var("RBUFR_TABLES_PATH") {
        return PathBuf::from(env_path);
    }

    #[cfg(feature = "python_bindings")]
    if let Some(python_path) = try_find_python_package_path() {
        return python_path;
    }

    PathBuf::from("tables")
}

#[allow(dead_code)]
fn try_find_python_package_path() -> Option<PathBuf> {
    if let Ok(exe_path) = std::env::current_exe() {
        let mut candidate = exe_path.parent()?.to_path_buf();
        candidate.push("rbufrp");
        candidate.push("tables");
        if candidate.exists() {
            return Some(candidate);
        }
    }
    None
}

pub fn get_table_path<P: AsRef<Path>>(relative_path: P) -> PathBuf {
    let base = get_tables_base_path();
    base.join(relative_path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_set_and_get_path() {
        set_tables_base_path("/custom/tables/path");
        let path = get_tables_base_path();
        assert_eq!(path, PathBuf::from("/custom/tables/path"));
    }

    #[test]
    fn test_get_table_path() {
        set_tables_base_path("/base");
        let table_path = get_table_path("master/BUFR_TableB_0.bufrtbl");
        assert_eq!(
            table_path,
            PathBuf::from("/base/master/BUFR_TableB_0.bufrtbl")
        );
    }
}
