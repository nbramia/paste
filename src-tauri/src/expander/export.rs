//! Snippet export/import in JSON format.

use serde::{Deserialize, Serialize};

/// Export format for snippets.
#[derive(Debug, Serialize, Deserialize)]
pub struct SnippetExport {
    pub version: u32,
    pub groups: Vec<ExportGroup>,
}

/// A group with its snippets for export.
#[derive(Debug, Serialize, Deserialize)]
pub struct ExportGroup {
    pub name: String,
    pub snippets: Vec<ExportSnippet>,
}

/// A single snippet for export.
#[derive(Debug, Serialize, Deserialize)]
pub struct ExportSnippet {
    pub abbreviation: String,
    pub name: String,
    pub content: String,
    pub content_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

/// Result of a JSON import operation.
#[derive(Debug, Clone, Serialize)]
pub struct JsonImportResult {
    pub imported: usize,
    pub skipped: usize,
    pub errors: Vec<String>,
    pub has_scripts: bool,
}

/// Build an export from storage data.
pub fn build_export(
    snippets: &[crate::storage::models::Snippet],
    groups: &[crate::storage::models::SnippetGroup],
) -> SnippetExport {
    let mut export_groups: Vec<ExportGroup> = Vec::new();

    // Add named groups with their snippets
    for group in groups {
        let group_snippets: Vec<ExportSnippet> = snippets
            .iter()
            .filter(|s| s.group_id.as_deref() == Some(&group.id))
            .map(|s| ExportSnippet {
                abbreviation: s.abbreviation.clone(),
                name: s.name.clone(),
                content: s.content.clone(),
                content_type: s.content_type.clone(),
                description: s.description.clone(),
            })
            .collect();

        export_groups.push(ExportGroup {
            name: group.name.clone(),
            snippets: group_snippets,
        });
    }

    // Add ungrouped snippets under a special group
    let ungrouped: Vec<ExportSnippet> = snippets
        .iter()
        .filter(|s| s.group_id.is_none())
        .map(|s| ExportSnippet {
            abbreviation: s.abbreviation.clone(),
            name: s.name.clone(),
            content: s.content.clone(),
            content_type: s.content_type.clone(),
            description: s.description.clone(),
        })
        .collect();

    if !ungrouped.is_empty() {
        export_groups.push(ExportGroup {
            name: "Ungrouped".to_string(),
            snippets: ungrouped,
        });
    }

    SnippetExport {
        version: 1,
        groups: export_groups,
    }
}

/// Parse an import file.
pub fn parse_import(json: &str) -> Result<SnippetExport, String> {
    let export: SnippetExport =
        serde_json::from_str(json).map_err(|e| format!("Invalid JSON: {e}"))?;

    if export.version != 1 {
        return Err(format!(
            "Unsupported export version: {} (expected 1)",
            export.version
        ));
    }

    Ok(export)
}

/// Check if an export contains any shell script snippets.
pub fn has_script_snippets(export: &SnippetExport) -> bool {
    export
        .groups
        .iter()
        .flat_map(|g| &g.snippets)
        .any(|s| s.content_type == "script" || s.content.contains("%shell("))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_export_empty() {
        let export = build_export(&[], &[]);
        assert_eq!(export.version, 1);
        assert!(export.groups.is_empty());
    }

    #[test]
    fn test_build_export_ungrouped() {
        let snippets = vec![crate::storage::models::Snippet {
            id: "1".into(),
            abbreviation: ";sig".into(),
            name: "Signature".into(),
            content: "Best regards".into(),
            content_type: "plain".into(),
            group_id: None,
            description: None,
            use_count: 0,
            created_at: "2024-01-01".into(),
            updated_at: "2024-01-01".into(),
        }];
        let export = build_export(&snippets, &[]);
        assert_eq!(export.groups.len(), 1);
        assert_eq!(export.groups[0].name, "Ungrouped");
        assert_eq!(export.groups[0].snippets.len(), 1);
        assert_eq!(export.groups[0].snippets[0].abbreviation, ";sig");
    }

    #[test]
    fn test_round_trip() {
        let export = SnippetExport {
            version: 1,
            groups: vec![ExportGroup {
                name: "Work".into(),
                snippets: vec![ExportSnippet {
                    abbreviation: ";sig".into(),
                    name: "Signature".into(),
                    content: "Best regards\nJohn".into(),
                    content_type: "plain".into(),
                    description: Some("Email signature".into()),
                }],
            }],
        };

        let json = serde_json::to_string_pretty(&export).unwrap();
        let parsed = parse_import(&json).unwrap();
        assert_eq!(parsed.version, 1);
        assert_eq!(parsed.groups.len(), 1);
        assert_eq!(parsed.groups[0].snippets[0].abbreviation, ";sig");
        assert_eq!(
            parsed.groups[0].snippets[0].description.as_deref(),
            Some("Email signature")
        );
    }

    #[test]
    fn test_parse_invalid_json() {
        assert!(parse_import("not json").is_err());
    }

    #[test]
    fn test_parse_wrong_version() {
        let json = r#"{"version": 99, "groups": []}"#;
        assert!(parse_import(json).is_err());
    }

    #[test]
    fn test_has_script_snippets() {
        let export = SnippetExport {
            version: 1,
            groups: vec![ExportGroup {
                name: "Test".into(),
                snippets: vec![
                    ExportSnippet {
                        abbreviation: ";a".into(),
                        name: "A".into(),
                        content: "plain text".into(),
                        content_type: "plain".into(),
                        description: None,
                    },
                    ExportSnippet {
                        abbreviation: ";b".into(),
                        name: "B".into(),
                        content: "has %shell(echo hi)".into(),
                        content_type: "plain".into(),
                        description: None,
                    },
                ],
            }],
        };
        assert!(has_script_snippets(&export));
    }

    #[test]
    fn test_no_script_snippets() {
        let export = SnippetExport {
            version: 1,
            groups: vec![ExportGroup {
                name: "Test".into(),
                snippets: vec![ExportSnippet {
                    abbreviation: ";a".into(),
                    name: "A".into(),
                    content: "safe text".into(),
                    content_type: "plain".into(),
                    description: None,
                }],
            }],
        };
        assert!(!has_script_snippets(&export));
    }
}
