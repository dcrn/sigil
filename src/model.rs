use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
#[serde(rename_all = "snake_case")]
pub enum Priority {
    #[default]
    Must,
    Should,
    Prefer,
}

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
#[serde(rename_all = "snake_case")]
pub enum Status {
    #[default]
    Active,
    Draft,
    Deprecated,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(untagged)]
pub enum AppliesTo {
    Single(String),
    Multiple(Vec<String>),
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Trigger {
    #[serde(rename = "type", skip_serializing_if = "Option::is_none")]
    pub kind: Option<String>,
    #[serde(flatten)]
    pub extra: HashMap<String, serde_json::Value>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Rule {
    pub id: String,
    pub description: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub files: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub constraints: Option<Vec<String>>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ChangelogEntry {
    pub version: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub date: Option<String>,
    pub description: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Contract {
    pub id: String,
    pub version: String,
    pub name: String,
    pub description: String,
    #[serde(default)]
    pub priority: Priority,
    #[serde(default)]
    pub status: Status,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub domain: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tags: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub applies_to: Option<AppliesTo>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub trigger: Option<Trigger>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub files: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rules: Option<Vec<Rule>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub notes: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub changelog: Option<Vec<ChangelogEntry>>,
    #[serde(flatten)]
    pub extra: serde_json::Map<String, serde_json::Value>,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn minimal() -> Contract {
        Contract {
            id: "test".to_string(),
            version: "1.0.0".to_string(),
            name: "Test".to_string(),
            description: "desc".to_string(),
            priority: Priority::Must,
            status: Status::Active,
            domain: None,
            tags: None,
            applies_to: None,
            trigger: None,
            files: None,
            rules: None,
            notes: None,
            changelog: None,
            extra: serde_json::Map::new(),
        }
    }

    #[test]
    fn all_files_empty_when_none_set() {
        assert!(minimal().all_files().is_empty());
    }

    #[test]
    fn all_files_collects_top_level_files() {
        let mut c = minimal();
        c.files = Some(vec!["src/foo.rs".to_string(), "src/bar.rs".to_string()]);
        assert_eq!(c.all_files(), vec!["src/foo.rs", "src/bar.rs"]);
    }

    #[test]
    fn all_files_collects_rule_files() {
        let mut c = minimal();
        c.rules = Some(vec![Rule {
            id: "r1".to_string(),
            description: "rule".to_string(),
            files: Some(vec!["schema/x.json".to_string()]),
            constraints: None,
        }]);
        assert_eq!(c.all_files(), vec!["schema/x.json"]);
    }

    #[test]
    fn all_files_collects_both_top_level_and_rule_files() {
        let mut c = minimal();
        c.files = Some(vec!["src/main.rs".to_string()]);
        c.rules = Some(vec![Rule {
            id: "r1".to_string(),
            description: "rule".to_string(),
            files: Some(vec!["schema/x.json".to_string()]),
            constraints: None,
        }]);
        assert_eq!(c.all_files(), vec!["src/main.rs", "schema/x.json"]);
    }

    #[test]
    fn applies_to_patterns_none() {
        assert!(minimal().applies_to_patterns().is_empty());
    }

    #[test]
    fn applies_to_patterns_single() {
        let mut c = minimal();
        c.applies_to = Some(AppliesTo::Single("src/**/*.rs".to_string()));
        assert_eq!(c.applies_to_patterns(), vec!["src/**/*.rs"]);
    }

    #[test]
    fn applies_to_patterns_multiple() {
        let mut c = minimal();
        c.applies_to = Some(AppliesTo::Multiple(vec![
            "src/**/*.rs".to_string(),
            "tests/**/*.rs".to_string(),
        ]));
        assert_eq!(c.applies_to_patterns(), vec!["src/**/*.rs", "tests/**/*.rs"]);
    }
}

impl Contract {
    /// All file paths referenced in this contract.
    pub fn all_files(&self) -> Vec<&str> {
        let mut paths = Vec::new();
        if let Some(files) = &self.files {
            for f in files {
                paths.push(f.as_str());
            }
        }
        if let Some(rules) = &self.rules {
            for rule in rules {
                if let Some(files) = &rule.files {
                    for f in files {
                        paths.push(f.as_str());
                    }
                }
            }
        }
        paths
    }

    pub fn applies_to_patterns(&self) -> Vec<&str> {
        match &self.applies_to {
            None => vec![],
            Some(AppliesTo::Single(s)) => vec![s.as_str()],
            Some(AppliesTo::Multiple(v)) => v.iter().map(|s| s.as_str()).collect(),
        }
    }
}
