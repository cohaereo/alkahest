use std::fmt::Display;

use egui::ahash::HashMap;
use itertools::Itertools;

#[derive(Debug)]
pub struct VersionSection {
    pub header: String,

    /// Key is the change type (Added, Changed, Deprecated, Removed, Fixed, Security)
    pub sections: HashMap<String, Vec<String>>,
}

impl VersionSection {
    pub fn diff(&self, other: &Self) -> VersionSection {
        let mut sections = HashMap::default();

        for (key, changes) in self.sections.iter() {
            if let Some(other_changes) = other.sections.get(key) {
                let diff = changes
                    .iter()
                    .filter(|change| !other_changes.contains(change))
                    .cloned()
                    .collect_vec();

                if !diff.is_empty() {
                    sections.insert(key.clone(), diff);
                }
            }
        }

        VersionSection {
            header: self.header.clone(),
            sections,
        }
    }
}

impl Display for VersionSection {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "## {}\n", self.header)?;

        for (key, changes) in self.sections.iter() {
            writeln!(f, "### {}\n", key)?;

            for change in changes {
                writeln!(f, "{}", change)?;
            }

            writeln!(f)?;
        }

        Ok(())
    }
}

pub fn parse_changelog(changelog: &str) -> Vec<VersionSection> {
    let mut sections = vec![];
    let mut current_version = None;
    let mut current_section = String::new();

    for line in changelog.lines() {
        if line.starts_with("## ") {
            if let Some(section) = current_version.take() {
                sections.push(section);
            }

            if let Some(header) = line.strip_prefix("## ") {
                current_version = Some(VersionSection {
                    header: header.to_string(),
                    sections: HashMap::default(),
                });
            } else {
                error!("Invalid header in changelog: {}", line);
            }
        } else if let Some(section) = current_version.as_mut() {
            if line.starts_with("### ") {
                if let Some(key) = line.strip_prefix("### ") {
                    section.sections.insert(key.to_string(), vec![]);
                    current_section = key.to_string();
                } else {
                    error!("Invalid header in changelog: {}", line);
                }
            } else if let Some(changes) = section.sections.get_mut(&current_section) {
                if !line.is_empty() {
                    changes.push(line.to_string());
                }
            }
        }
    }

    if let Some(section) = current_version.take() {
        sections.push(section);
    }

    sections
}
