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
