use crate::model::Contract;
use walkdir::WalkDir;
use toml;

pub fn load_contracts(dir: &str) -> (Vec<Contract>, Vec<String>) {
    let mut contracts = Vec::new();
    let mut warnings = Vec::new();

    for entry in WalkDir::new(dir)
        .follow_links(true)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| {
            e.file_type().is_file()
                && e.file_name()
                    .to_string_lossy()
                    .ends_with(".contract.toml")
        })
    {
        let path = entry.path().display().to_string();
        match std::fs::read_to_string(entry.path()) {
            Ok(content) => match toml::from_str::<Contract>(&content) {
                Ok(contract) => contracts.push(contract),
                Err(e) => warnings.push(format!("Failed to parse {path}: {e}")),
            },
            Err(e) => warnings.push(format!("Failed to read {path}: {e}")),
        }
    }

    contracts.sort_by(|a, b| a.id.cmp(&b.id));
    (contracts, warnings)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn temp_dir(tag: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!("sigil_loader_test_{tag}"));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn write(dir: &std::path::Path, name: &str, content: &str) {
        fs::write(dir.join(name), content).unwrap();
    }

    const VALID: &str = r#"
id = "my-contract"
version = "1.0.0"
name = "My Contract"
description = "A test contract"
"#;

    #[test]
    fn empty_dir_returns_nothing() {
        let dir = temp_dir("empty");
        let (contracts, warnings) = load_contracts(dir.to_str().unwrap());
        assert!(contracts.is_empty());
        assert!(warnings.is_empty());
    }

    #[test]
    fn loads_valid_contract() {
        let dir = temp_dir("valid");
        write(&dir, "my-contract.contract.toml", VALID);
        let (contracts, warnings) = load_contracts(dir.to_str().unwrap());
        assert_eq!(contracts.len(), 1);
        assert_eq!(contracts[0].id, "my-contract");
        assert!(warnings.is_empty());
    }

    #[test]
    fn warns_on_malformed_toml_without_crashing() {
        let dir = temp_dir("malformed");
        write(&dir, "bad.contract.toml", "not valid toml ===");
        let (contracts, warnings) = load_contracts(dir.to_str().unwrap());
        assert!(contracts.is_empty(), "Malformed contract must not be loaded");
        assert!(!warnings.is_empty(), "Must warn on parse failure");
    }

    #[test]
    fn warns_on_missing_required_fields() {
        let dir = temp_dir("missing_fields");
        write(&dir, "bad.contract.toml", r#"id = "bad""#);
        let (contracts, warnings) = load_contracts(dir.to_str().unwrap());
        assert!(contracts.is_empty());
        assert!(!warnings.is_empty());
    }

    #[test]
    fn ignores_non_contract_toml_files() {
        let dir = temp_dir("nonmatching");
        write(&dir, "README.md", "not a contract");
        write(&dir, "config.toml", "[config]\n");
        let (contracts, warnings) = load_contracts(dir.to_str().unwrap());
        assert!(contracts.is_empty());
        assert!(warnings.is_empty());
    }

    #[test]
    fn sorts_contracts_by_id() {
        let dir = temp_dir("sorted");
        write(&dir, "z-last.contract.toml", r#"
id = "z-last"
version = "1.0.0"
name = "Z"
description = "Z contract"
"#);
        write(&dir, "a-first.contract.toml", r#"
id = "a-first"
version = "1.0.0"
name = "A"
description = "A contract"
"#);
        let (contracts, _) = load_contracts(dir.to_str().unwrap());
        assert_eq!(contracts.len(), 2);
        assert_eq!(contracts[0].id, "a-first");
        assert_eq!(contracts[1].id, "z-last");
    }
}
