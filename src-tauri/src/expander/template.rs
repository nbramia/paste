/// A parsed template token.
#[derive(Debug, Clone, PartialEq)]
pub enum TemplateToken {
    Literal(String),
    // Future tokens: DateFormat, Clipboard, CursorPosition, Shell, Nested, FillIn
}

/// Parse a template string into tokens.
/// For this initial implementation, the entire string is a single Literal token.
/// Macro support (date, clipboard, etc.) will be added in later issues.
pub fn parse_template(template: &str) -> Vec<TemplateToken> {
    vec![TemplateToken::Literal(template.to_string())]
}

/// Evaluate parsed tokens into the final expansion text.
pub fn evaluate_tokens(tokens: &[TemplateToken]) -> String {
    let mut output = String::new();
    for token in tokens {
        match token {
            TemplateToken::Literal(s) => output.push_str(s),
        }
    }
    output
}

/// Convenience: parse and evaluate a template in one step.
pub fn expand_template(template: &str) -> String {
    let tokens = parse_template(template);
    evaluate_tokens(&tokens)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_literal_template() {
        let tokens = parse_template("Hello, world!");
        assert_eq!(tokens, vec![TemplateToken::Literal("Hello, world!".into())]);
    }

    #[test]
    fn test_evaluate_literal() {
        let result = expand_template("Best regards,\nJohn");
        assert_eq!(result, "Best regards,\nJohn");
    }

    #[test]
    fn test_empty_template() {
        let result = expand_template("");
        assert_eq!(result, "");
    }

    #[test]
    fn test_multiline_template() {
        let template = "Line 1\nLine 2\nLine 3";
        let result = expand_template(template);
        assert_eq!(result, template);
    }
}
