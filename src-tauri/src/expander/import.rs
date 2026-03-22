//! Import snippets from espanso configuration files.

use std::path::{Path, PathBuf};
use std::fs;
use serde::Deserialize;
use log::{debug, info, warn};

/// An espanso match entry (trigger + replace).
#[derive(Debug, Deserialize)]
struct EspansoMatch {
    trigger: Option<String>,
    replace: Option<String>,
    #[serde(default)]
    word: bool,
    // form, vars, etc. are not supported yet
}

/// An espanso match file (contains a list of matches).
#[derive(Debug, Deserialize)]
struct EspansoMatchFile {
    #[serde(default)]
    matches: Vec<EspansoMatch>,
}

/// A parsed snippet ready for import.
#[derive(Debug, Clone, serde::Serialize)]
pub struct ImportedSnippet {
    pub abbreviation: String,
    pub name: String,
    pub content: String,
    pub content_type: String,
    pub source_file: String,
}

/// Result of an import operation.
#[derive(Debug, Clone, serde::Serialize)]
pub struct ImportResult {
    pub imported: usize,
    pub skipped: usize,
    pub errors: Vec<String>,
}

/// Get the default espanso match directory.
pub fn default_espanso_path() -> PathBuf {
    dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("/tmp"))
        .join("espanso")
        .join("match")
}

/// Parse all espanso YAML files from a directory.
pub fn parse_espanso_dir(dir: &Path) -> Result<Vec<ImportedSnippet>, String> {
    if !dir.exists() {
        return Err(format!("Directory not found: {}", dir.display()));
    }
    if !dir.is_dir() {
        return Err(format!("Not a directory: {}", dir.display()));
    }

    let mut snippets = Vec::new();

    let entries = fs::read_dir(dir).map_err(|e| format!("Failed to read directory: {e}"))?;

    for entry in entries {
        let entry = entry.map_err(|e| format!("Failed to read entry: {e}"))?;
        let path = entry.path();

        // Only process .yml and .yaml files
        let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
        if ext != "yml" && ext != "yaml" {
            continue;
        }

        match parse_espanso_file(&path) {
            Ok(file_snippets) => {
                info!("Parsed {} snippets from {}", file_snippets.len(), path.display());
                snippets.extend(file_snippets);
            }
            Err(e) => {
                warn!("Failed to parse {}: {e}", path.display());
            }
        }
    }

    Ok(snippets)
}

/// Parse a single espanso YAML file.
fn parse_espanso_file(path: &Path) -> Result<Vec<ImportedSnippet>, String> {
    let content = fs::read_to_string(path).map_err(|e| format!("Read error: {e}"))?;
    let file: EspansoMatchFile =
        serde_yaml::from_str(&content).map_err(|e| format!("YAML parse error: {e}"))?;

    let filename = path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("unknown")
        .to_string();

    let mut snippets = Vec::new();

    for m in &file.matches {
        let Some(ref trigger) = m.trigger else {
            debug!("Skipping match without trigger in {filename}");
            continue;
        };
        let Some(ref replace) = m.replace else {
            debug!("Skipping match without replace in {filename}");
            continue;
        };

        let content = convert_espanso_variables(replace);
        let name = generate_name(trigger);

        snippets.push(ImportedSnippet {
            abbreviation: trigger.clone(),
            name,
            content,
            content_type: "plain".to_string(),
            source_file: filename.clone(),
        });
    }

    Ok(snippets)
}

/// Convert espanso variable syntax to paste macro syntax.
pub fn convert_espanso_variables(content: &str) -> String {
    let mut result = content.to_string();

    // {{clipboard}} -> %clipboard
    result = result.replace("{{clipboard}}", "%clipboard");

    // {{date}} -> %Y-%m-%d (espanso's default date format)
    result = result.replace("{{date}}", "%Y-%m-%d");

    // {{time}} -> %H:%M:%S
    result = result.replace("{{time}}", "%H:%M:%S");

    // {{timestamp}} -> Unix timestamp via shell
    result = result.replace("{{timestamp}}", "%shell(date +%s)");

    // {{newline}} -> literal newline
    result = result.replace("{{newline}}", "\n");

    // {{tab}} -> literal tab
    result = result.replace("{{tab}}", "\t");

    result
}

/// Generate a human-readable name from a trigger string.
fn generate_name(trigger: &str) -> String {
    // Remove common prefix characters like ;, /, ,
    let cleaned = trigger
        .trim_start_matches(|c: char| !c.is_alphanumeric())
        .to_string();

    if cleaned.is_empty() {
        format!("Snippet: {trigger}")
    } else {
        // Capitalize first letter
        let mut chars = cleaned.chars();
        match chars.next() {
            Some(first) => {
                let capitalized: String = first.to_uppercase().chain(chars).collect();
                capitalized
            }
            None => format!("Snippet: {trigger}"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn test_convert_clipboard() {
        assert_eq!(
            convert_espanso_variables("paste: {{clipboard}}"),
            "paste: %clipboard"
        );
    }

    #[test]
    fn test_convert_date() {
        assert_eq!(
            convert_espanso_variables("today is {{date}}"),
            "today is %Y-%m-%d"
        );
    }

    #[test]
    fn test_convert_time() {
        assert_eq!(
            convert_espanso_variables("time: {{time}}"),
            "time: %H:%M:%S"
        );
    }

    #[test]
    fn test_convert_timestamp() {
        assert_eq!(
            convert_espanso_variables("ts: {{timestamp}}"),
            "ts: %shell(date +%s)"
        );
    }

    #[test]
    fn test_convert_newline_tab() {
        assert_eq!(
            convert_espanso_variables("a{{newline}}b{{tab}}c"),
            "a\nb\tc"
        );
    }

    #[test]
    fn test_convert_no_variables() {
        assert_eq!(
            convert_espanso_variables("plain text"),
            "plain text"
        );
    }

    #[test]
    fn test_convert_multiple_variables() {
        assert_eq!(
            convert_espanso_variables("{{date}} at {{time}}: {{clipboard}}"),
            "%Y-%m-%d at %H:%M:%S: %clipboard"
        );
    }

    #[test]
    fn test_generate_name() {
        assert_eq!(generate_name(";sig"), "Sig");
        assert_eq!(generate_name("//date"), "Date");
        assert_eq!(generate_name(",,addr"), "Addr");
        assert_eq!(generate_name("hello"), "Hello");
    }

    #[test]
    fn test_generate_name_special() {
        let name = generate_name(";;;");
        assert!(name.starts_with("Snippet: "));
    }

    #[test]
    fn test_parse_espanso_file() {
        let dir = std::env::temp_dir().join("paste_espanso_test");
        let _ = fs::create_dir_all(&dir);

        let yaml = r#"
matches:
  - trigger: ";sig"
    replace: "Best regards,\nJohn"
  - trigger: ";email"
    replace: "john@example.com"
  - trigger: ";date"
    replace: "Today is {{date}}"
"#;
        let path = dir.join("base.yml");
        let mut f = fs::File::create(&path).unwrap();
        f.write_all(yaml.as_bytes()).unwrap();

        let snippets = parse_espanso_file(&path).unwrap();
        assert_eq!(snippets.len(), 3);
        assert_eq!(snippets[0].abbreviation, ";sig");
        assert_eq!(snippets[0].content, "Best regards,\\nJohn"); // YAML literal
        assert_eq!(snippets[1].abbreviation, ";email");
        assert_eq!(snippets[2].content, "Today is %Y-%m-%d");

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_parse_espanso_file_skip_no_trigger() {
        let dir = std::env::temp_dir().join("paste_espanso_test2");
        let _ = fs::create_dir_all(&dir);

        let yaml = r#"
matches:
  - trigger: ";ok"
    replace: "works"
  - replace: "no trigger"
"#;
        let path = dir.join("partial.yml");
        fs::write(&path, yaml).unwrap();

        let snippets = parse_espanso_file(&path).unwrap();
        assert_eq!(snippets.len(), 1);
        assert_eq!(snippets[0].abbreviation, ";ok");

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_parse_espanso_dir() {
        let dir = std::env::temp_dir().join("paste_espanso_dir_test");
        let _ = fs::create_dir_all(&dir);

        fs::write(
            dir.join("a.yml"),
            "matches:\n  - trigger: \";a\"\n    replace: \"alpha\"\n",
        ).unwrap();
        fs::write(
            dir.join("b.yaml"),
            "matches:\n  - trigger: \";b\"\n    replace: \"beta\"\n",
        ).unwrap();
        fs::write(dir.join("readme.txt"), "not yaml").unwrap();

        let snippets = parse_espanso_dir(&dir).unwrap();
        assert_eq!(snippets.len(), 2);

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_parse_nonexistent_dir() {
        let result = parse_espanso_dir(Path::new("/tmp/nonexistent_paste_test_xyz"));
        assert!(result.is_err());
    }
}
