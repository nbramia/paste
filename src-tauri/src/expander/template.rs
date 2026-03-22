use std::collections::HashMap;
use std::process::Command;
use std::sync::Arc;

use chrono::{Datelike, Duration, Local};
use serde::{Deserialize, Serialize};

/// Specification for a fill-in text field.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FillInSpec {
    pub name: String,
    pub default_value: Option<String>,
}

/// Specification for a fill-in popup/dropdown field.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FillPopupSpec {
    pub name: String,
    pub options: Vec<String>,
}

/// A fill-in field descriptor (for sending to the frontend dialog).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum FillInField {
    #[serde(rename = "text")]
    Text {
        name: String,
        default_value: Option<String>,
    },
    #[serde(rename = "textarea")]
    TextArea {
        name: String,
        default_value: Option<String>,
    },
    #[serde(rename = "popup")]
    Popup {
        name: String,
        options: Vec<String>,
    },
}

/// A parsed template token.
#[derive(Debug, Clone, PartialEq)]
pub enum TemplateToken {
    /// Plain text.
    Literal(String),
    /// Date/time format code (e.g., "Y" for %Y, "m" for %m).
    DateFormat(String),
    /// Date math: offset from current date/time (e.g., +5d, -1w, +3M).
    DateMath {
        offset: i64,
        unit: DateMathUnit,
        format: String, // output format, default "%Y-%m-%d"
    },
    /// Insert current clipboard content.
    Clipboard,
    /// Cursor position marker.
    CursorPosition,
    /// Fill-in: single-line text input.
    FillIn(FillInSpec),
    /// Fill-in: multi-line text area.
    FillArea(FillInSpec),
    /// Fill-in: popup/dropdown menu.
    FillPopup(FillPopupSpec),
    /// Shell command — execute and use stdout.
    ShellCommand(String),
    /// Nested snippet — look up by abbreviation and expand recursively.
    NestedSnippet(String),
}

/// Units for date math.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum DateMathUnit {
    Minutes,
    Hours,
    Days,
    Weeks,
    Months,
    Years,
}

/// Context for template evaluation.
pub struct ExpansionContext {
    /// Current clipboard text content (for %clipboard macro).
    pub clipboard_content: String,
    /// User-provided values for fill-in fields (keyed by field name).
    pub fill_values: HashMap<String, String>,
    /// Lookup function for nested snippets: abbreviation -> template content.
    /// This avoids coupling the template engine to the storage layer.
    pub snippet_lookup: Option<Arc<dyn Fn(&str) -> Option<String> + Send + Sync>>,
    /// Current recursion depth (for nested snippet cycle detection).
    pub depth: usize,
    /// Maximum recursion depth for nested snippets.
    pub max_depth: usize,
    /// Abbreviations currently being expanded (for cycle detection).
    pub expanding: Vec<String>,
}

impl Default for ExpansionContext {
    fn default() -> Self {
        Self {
            clipboard_content: String::new(),
            fill_values: HashMap::new(),
            snippet_lookup: None,
            depth: 0,
            max_depth: 10,
            expanding: Vec::new(),
        }
    }
}

/// Result of template expansion.
#[derive(Debug, Clone)]
pub struct ExpansionResult {
    /// The expanded text.
    pub text: String,
    /// Optional cursor position (byte offset from start of text).
    /// If set, the cursor should be moved to this position after injection.
    pub cursor_offset: Option<usize>,
}

/// Parse a template string into tokens.
///
/// Macro syntax:
/// - `%Y`, `%m`, `%d`, `%H`, `%M`, `%S`, `%A`, `%B`, `%p`, etc. -- date/time format codes
/// - `%date(+5d)`, `%date(-1w)`, `%date(+3M)` -- date math
/// - `%clipboard` -- insert clipboard content
/// - `%|` -- cursor position marker
/// - `%%` -- literal percent sign
/// - Unknown `%X` -- left as literal text
pub fn parse_template(template: &str) -> Vec<TemplateToken> {
    let mut tokens: Vec<TemplateToken> = Vec::new();
    let mut chars = template.chars().peekable();
    let mut current_literal = String::new();

    while let Some(ch) = chars.next() {
        if ch != '%' {
            current_literal.push(ch);
            continue;
        }

        // We have a '%' -- look at the next character
        let Some(&next) = chars.peek() else {
            // '%' at end of string -- treat as literal
            current_literal.push('%');
            continue;
        };

        match next {
            // %% -> literal %
            '%' => {
                chars.next();
                current_literal.push('%');
            }
            // %| -> cursor position
            '|' => {
                chars.next();
                flush_literal(&mut current_literal, &mut tokens);
                tokens.push(TemplateToken::CursorPosition);
            }
            // %d or %date(...)
            'd' => {
                // Check if it's %date(...)
                let rest: String = chars.clone().take(4).collect();
                if rest.starts_with("date") {
                    // Look ahead further to see if there's a '(' after "date"
                    let rest5: String = chars.clone().take(5).collect();
                    if rest5.starts_with("date(") {
                        // Consume "date("
                        for _ in 0..5 {
                            chars.next();
                        }
                        // Read until closing ')'
                        let mut expr = String::new();
                        let mut found_close = false;
                        for c in chars.by_ref() {
                            if c == ')' {
                                found_close = true;
                                break;
                            }
                            expr.push(c);
                        }
                        if found_close {
                            flush_literal(&mut current_literal, &mut tokens);
                            if let Some(token) = parse_date_math(&expr) {
                                tokens.push(token);
                            } else {
                                // Invalid date math -- emit as literal
                                current_literal.push_str(&format!("%date({expr})"));
                            }
                        } else {
                            // No closing paren -- treat as literal
                            current_literal.push_str(&format!("%date({expr}"));
                        }
                    } else {
                        // %date without parens -- treat %d as day-of-month format code
                        chars.next();
                        flush_literal(&mut current_literal, &mut tokens);
                        tokens.push(TemplateToken::DateFormat("d".to_string()));
                    }
                } else {
                    // Just %d -- day of month
                    chars.next();
                    flush_literal(&mut current_literal, &mut tokens);
                    tokens.push(TemplateToken::DateFormat("d".to_string()));
                }
            }
            // %clipboard
            'c' => {
                let rest: String = chars.clone().take(9).collect();
                if rest.starts_with("clipboard") {
                    // Consume "clipboard" (9 chars)
                    for _ in 0..9 {
                        chars.next();
                    }
                    flush_literal(&mut current_literal, &mut tokens);
                    tokens.push(TemplateToken::Clipboard);
                } else {
                    current_literal.push('%');
                }
            }
            // Single-char date format codes
            'Y' | 'm' | 'H' | 'M' | 'S' | 'A' | 'B' | 'p' | 'a' | 'b' | 'I' | 'j' | 'u'
            | 'w' | 'e' | 'k' | 'l' | 'Z' | 'z' => {
                chars.next();
                flush_literal(&mut current_literal, &mut tokens);
                tokens.push(TemplateToken::DateFormat(next.to_string()));
            }
            // %fill(...), %fillarea(...), %fillpopup(...)
            'f' => {
                // Note: chars.clone() includes the peeked 'f', so rest starts with 'f'
                let rest: String = chars.clone().take(10).collect();
                if rest.starts_with("fillpopup(") {
                    // %fillpopup(name:opt1:opt2:opt3) — consume "fillpopup("
                    for _ in 0..10 {
                        chars.next();
                    }
                    let mut inner = String::new();
                    let mut found_close = false;
                    while let Some(c) = chars.next() {
                        if c == ')' {
                            found_close = true;
                            break;
                        }
                        inner.push(c);
                    }
                    if found_close {
                        flush_literal(&mut current_literal, &mut tokens);
                        let parts: Vec<&str> = inner.split(':').collect();
                        if !parts.is_empty() {
                            let name = parts[0].to_string();
                            let options: Vec<String> =
                                parts[1..].iter().map(|s| s.to_string()).collect();
                            tokens
                                .push(TemplateToken::FillPopup(FillPopupSpec { name, options }));
                        }
                    } else {
                        current_literal.push_str(&format!("%fillpopup({inner}"));
                    }
                } else if rest.starts_with("fillarea(") {
                    // %fillarea(name) — consume "fillarea("
                    for _ in 0..9 {
                        chars.next();
                    }
                    let mut inner = String::new();
                    let mut found_close = false;
                    while let Some(c) = chars.next() {
                        if c == ')' {
                            found_close = true;
                            break;
                        }
                        inner.push(c);
                    }
                    if found_close {
                        flush_literal(&mut current_literal, &mut tokens);
                        tokens.push(TemplateToken::FillArea(FillInSpec {
                            name: inner.clone(),
                            default_value: None,
                        }));
                    } else {
                        current_literal.push_str(&format!("%fillarea({inner}"));
                    }
                } else if rest.starts_with("fill(") {
                    // %fill(name) or %fill(name:default=value) — consume "fill("
                    for _ in 0..5 {
                        chars.next();
                    }
                    let mut inner = String::new();
                    let mut found_close = false;
                    while let Some(c) = chars.next() {
                        if c == ')' {
                            found_close = true;
                            break;
                        }
                        inner.push(c);
                    }
                    if found_close {
                        flush_literal(&mut current_literal, &mut tokens);
                        tokens.push(parse_fill_in(&inner));
                    } else {
                        current_literal.push_str(&format!("%fill({inner}"));
                    }
                } else {
                    current_literal.push('%');
                }
            }
            // %shell(...) or %snippet(...)
            's' => {
                // Note: chars.clone() includes the peeked 's', so rest starts with 's'
                let rest: String = chars.clone().take(8).collect();
                if rest.starts_with("shell(") {
                    // %shell(command) — consume "shell("
                    for _ in 0..6 {
                        chars.next();
                    }
                    let mut inner = String::new();
                    let mut paren_depth = 1u32;
                    while let Some(c) = chars.next() {
                        if c == '(' {
                            paren_depth += 1;
                        }
                        if c == ')' {
                            paren_depth -= 1;
                            if paren_depth == 0 {
                                break;
                            }
                        }
                        inner.push(c);
                    }
                    if paren_depth == 0 {
                        flush_literal(&mut current_literal, &mut tokens);
                        tokens.push(TemplateToken::ShellCommand(inner));
                    } else {
                        current_literal.push_str(&format!("%shell({inner}"));
                    }
                } else if rest.starts_with("snippet(") {
                    // %snippet(abbreviation) — consume "snippet("
                    for _ in 0..8 {
                        chars.next();
                    }
                    let mut inner = String::new();
                    let mut found_close = false;
                    while let Some(c) = chars.next() {
                        if c == ')' {
                            found_close = true;
                            break;
                        }
                        inner.push(c);
                    }
                    if found_close && !inner.is_empty() {
                        flush_literal(&mut current_literal, &mut tokens);
                        tokens.push(TemplateToken::NestedSnippet(inner));
                    } else {
                        current_literal.push_str(&format!("%snippet({inner}"));
                    }
                } else {
                    // Unknown %s... — leave as literal
                    current_literal.push('%');
                }
            }
            // Unknown %X -- leave as literal
            _ => {
                current_literal.push('%');
            }
        }
    }

    flush_literal(&mut current_literal, &mut tokens);
    tokens
}

/// Flush accumulated literal text into a Literal token.
fn flush_literal(current: &mut String, tokens: &mut Vec<TemplateToken>) {
    if !current.is_empty() {
        tokens.push(TemplateToken::Literal(std::mem::take(current)));
    }
}

/// Parse a fill-in specifier from the inner text of `%fill(...)`.
fn parse_fill_in(inner: &str) -> TemplateToken {
    // Check for :default=value syntax
    if let Some(idx) = inner.find(":default=") {
        let name = inner[..idx].to_string();
        let default_value = inner[idx + 9..].to_string();
        TemplateToken::FillIn(FillInSpec {
            name,
            default_value: Some(default_value),
        })
    } else {
        TemplateToken::FillIn(FillInSpec {
            name: inner.to_string(),
            default_value: None,
        })
    }
}

/// Parse a date math expression like "+5d", "-1w", "+3M".
fn parse_date_math(expr: &str) -> Option<TemplateToken> {
    let expr = expr.trim();
    if expr.is_empty() {
        return None;
    }

    // Parse sign
    let (sign, rest) = if let Some(stripped) = expr.strip_prefix('+') {
        (1i64, stripped)
    } else if let Some(stripped) = expr.strip_prefix('-') {
        (-1i64, stripped)
    } else {
        (1i64, expr)
    };

    // Split into number and unit
    let num_end = rest
        .find(|c: char| !c.is_ascii_digit())
        .unwrap_or(rest.len());
    if num_end == 0 {
        return None;
    }

    let number: i64 = rest[..num_end].parse().ok()?;
    let unit_str = &rest[num_end..];

    let unit = match unit_str {
        "m" => DateMathUnit::Minutes,
        "h" => DateMathUnit::Hours,
        "d" => DateMathUnit::Days,
        "w" => DateMathUnit::Weeks,
        "M" => DateMathUnit::Months,
        "y" => DateMathUnit::Years,
        _ => return None,
    };

    Some(TemplateToken::DateMath {
        offset: sign * number,
        unit,
        format: "%Y-%m-%d".to_string(),
    })
}

/// Evaluate parsed tokens into an ExpansionResult.
pub fn evaluate_tokens(tokens: &[TemplateToken], ctx: &ExpansionContext) -> ExpansionResult {
    let now = Local::now();
    let mut text = String::new();
    let mut cursor_offset: Option<usize> = None;

    for token in tokens {
        match token {
            TemplateToken::Literal(s) => {
                text.push_str(s);
            }
            TemplateToken::DateFormat(code) => {
                let fmt = format!("%{code}");
                text.push_str(&now.format(&fmt).to_string());
            }
            TemplateToken::DateMath {
                offset,
                unit,
                format,
            } => {
                let target = apply_date_math(&now, *offset, *unit);
                text.push_str(&target.format(format).to_string());
            }
            TemplateToken::Clipboard => {
                text.push_str(&ctx.clipboard_content);
            }
            TemplateToken::CursorPosition => {
                cursor_offset = Some(text.len());
            }
            TemplateToken::FillIn(spec) => {
                if let Some(val) = ctx.fill_values.get(&spec.name) {
                    text.push_str(val);
                } else if let Some(ref default) = spec.default_value {
                    text.push_str(default);
                }
            }
            TemplateToken::FillArea(spec) => {
                if let Some(val) = ctx.fill_values.get(&spec.name) {
                    text.push_str(val);
                } else if let Some(ref default) = spec.default_value {
                    text.push_str(default);
                }
            }
            TemplateToken::FillPopup(spec) => {
                if let Some(val) = ctx.fill_values.get(&spec.name) {
                    text.push_str(val);
                } else if let Some(first) = spec.options.first() {
                    text.push_str(first);
                }
            }
            TemplateToken::ShellCommand(cmd) => {
                text.push_str(&execute_shell_command(cmd));
            }
            TemplateToken::NestedSnippet(abbr) => {
                text.push_str(&expand_nested_snippet(abbr, ctx));
            }
        }
    }

    ExpansionResult {
        text,
        cursor_offset,
    }
}

/// Apply date math to a datetime.
fn apply_date_math(
    base: &chrono::DateTime<Local>,
    offset: i64,
    unit: DateMathUnit,
) -> chrono::DateTime<Local> {
    match unit {
        DateMathUnit::Minutes => *base + Duration::minutes(offset),
        DateMathUnit::Hours => *base + Duration::hours(offset),
        DateMathUnit::Days => *base + Duration::days(offset),
        DateMathUnit::Weeks => *base + Duration::weeks(offset),
        DateMathUnit::Months => {
            // chrono doesn't have Duration::months, so manually adjust
            let target_month = base.month0() as i64 + offset;
            let years_offset = target_month.div_euclid(12);
            let new_month0 = target_month.rem_euclid(12) as u32;
            let new_year = base.year() + years_offset as i32;
            // Clamp day to valid range for the target month
            let max_day = days_in_month(new_year, new_month0 + 1);
            let new_day = base.day().min(max_day);
            base.with_year(new_year)
                .and_then(|d| d.with_month0(new_month0))
                .and_then(|d| d.with_day(new_day))
                .unwrap_or(*base)
        }
        DateMathUnit::Years => {
            let new_year = base.year() + offset as i32;
            // Handle Feb 29 -> Feb 28 in non-leap years
            let max_day = days_in_month(new_year, base.month());
            let new_day = base.day().min(max_day);
            base.with_year(new_year)
                .and_then(|d| d.with_day(new_day))
                .unwrap_or(*base)
        }
    }
}

/// Get the number of days in a month.
fn days_in_month(year: i32, month: u32) -> u32 {
    match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 => {
            if (year % 4 == 0 && year % 100 != 0) || (year % 400 == 0) {
                29
            } else {
                28
            }
        }
        _ => 30,
    }
}

/// Execute a shell command and return its stdout.
/// Times out after 5 seconds. Returns error text on failure.
fn execute_shell_command(command: &str) -> String {
    use std::process::Stdio;
    use std::sync::mpsc;
    use std::thread;
    use std::time::Duration;

    let cmd = command.to_string();
    let (tx, rx) = mpsc::channel();

    thread::spawn(move || {
        let result = Command::new("sh")
            .args(["-c", &cmd])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output();
        let _ = tx.send(result);
    });

    match rx.recv_timeout(Duration::from_secs(5)) {
        Ok(Ok(output)) => {
            if output.status.success() {
                let mut stdout = String::from_utf8_lossy(&output.stdout).to_string();
                // Trim trailing newline (common in command output)
                if stdout.ends_with('\n') {
                    stdout.pop();
                    if stdout.ends_with('\r') {
                        stdout.pop();
                    }
                }
                stdout
            } else {
                let stderr = String::from_utf8_lossy(&output.stderr);
                format!("[shell error: exit {}, {}]", output.status, stderr.trim())
            }
        }
        Ok(Err(e)) => format!("[shell error: {e}]"),
        Err(_) => "[shell error: timeout (5s)]".to_string(),
    }
}

/// Expand a nested snippet by looking up its abbreviation and recursively evaluating.
/// Detects cycles and enforces maximum recursion depth.
fn expand_nested_snippet(abbreviation: &str, ctx: &ExpansionContext) -> String {
    // Check max depth
    if ctx.depth >= ctx.max_depth {
        return format!("[snippet error: max depth ({}) exceeded]", ctx.max_depth);
    }

    // Check for cycles
    if ctx.expanding.contains(&abbreviation.to_string()) {
        return format!("[snippet error: circular reference to '{abbreviation}']");
    }

    // Look up the snippet
    let Some(ref lookup) = ctx.snippet_lookup else {
        return format!("[snippet error: no lookup configured]");
    };

    let Some(template) = lookup(abbreviation) else {
        return format!("[snippet error: '{abbreviation}' not found]");
    };

    // Parse and evaluate recursively with increased depth
    let tokens = parse_template(&template);
    let mut expanding = ctx.expanding.clone();
    expanding.push(abbreviation.to_string());

    let nested_ctx = ExpansionContext {
        clipboard_content: ctx.clipboard_content.clone(),
        fill_values: ctx.fill_values.clone(),
        snippet_lookup: Some(Arc::clone(lookup)),
        depth: ctx.depth + 1,
        max_depth: ctx.max_depth,
        expanding,
    };
    evaluate_tokens(&tokens, &nested_ctx).text
}

/// Convenience: parse and evaluate a template with default context.
pub fn expand_template(template: &str) -> String {
    let tokens = parse_template(template);
    let ctx = ExpansionContext::default();
    evaluate_tokens(&tokens, &ctx).text
}

/// Parse and evaluate with a full context.
pub fn expand_template_with_context(template: &str, ctx: &ExpansionContext) -> ExpansionResult {
    let tokens = parse_template(template);
    evaluate_tokens(&tokens, ctx)
}

/// Extract fill-in field specifications from parsed tokens.
/// Returns the list of fields that need user input, deduplicated by name.
pub fn extract_fill_in_fields(tokens: &[TemplateToken]) -> Vec<FillInField> {
    let mut fields = Vec::new();
    let mut seen_names = std::collections::HashSet::new();

    for token in tokens {
        match token {
            TemplateToken::FillIn(spec) => {
                if seen_names.insert(spec.name.clone()) {
                    fields.push(FillInField::Text {
                        name: spec.name.clone(),
                        default_value: spec.default_value.clone(),
                    });
                }
            }
            TemplateToken::FillArea(spec) => {
                if seen_names.insert(spec.name.clone()) {
                    fields.push(FillInField::TextArea {
                        name: spec.name.clone(),
                        default_value: spec.default_value.clone(),
                    });
                }
            }
            TemplateToken::FillPopup(spec) => {
                if seen_names.insert(spec.name.clone()) {
                    fields.push(FillInField::Popup {
                        name: spec.name.clone(),
                        options: spec.options.clone(),
                    });
                }
            }
            _ => {}
        }
    }
    fields
}

/// Check if a template has any fill-in fields.
pub fn has_fill_in_fields(template: &str) -> bool {
    let tokens = parse_template(template);
    tokens.iter().any(|t| {
        matches!(
            t,
            TemplateToken::FillIn(_) | TemplateToken::FillArea(_) | TemplateToken::FillPopup(_)
        )
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    // --- Parsing tests ---

    #[test]
    fn test_parse_plain_literal() {
        let tokens = parse_template("Hello, world!");
        assert_eq!(tokens, vec![TemplateToken::Literal("Hello, world!".into())]);
    }

    #[test]
    fn test_parse_empty() {
        let tokens = parse_template("");
        assert!(tokens.is_empty());
    }

    #[test]
    fn test_parse_date_format_codes() {
        let tokens = parse_template("%Y-%m-%d");
        assert_eq!(tokens.len(), 5);
        assert_eq!(tokens[0], TemplateToken::DateFormat("Y".into()));
        assert_eq!(tokens[1], TemplateToken::Literal("-".into()));
        assert_eq!(tokens[2], TemplateToken::DateFormat("m".into()));
        assert_eq!(tokens[3], TemplateToken::Literal("-".into()));
        assert_eq!(tokens[4], TemplateToken::DateFormat("d".into()));
    }

    #[test]
    fn test_parse_time_format() {
        let tokens = parse_template("%H:%M:%S");
        assert_eq!(tokens.len(), 5);
        assert_eq!(tokens[0], TemplateToken::DateFormat("H".into()));
        assert_eq!(tokens[2], TemplateToken::DateFormat("M".into()));
        assert_eq!(tokens[4], TemplateToken::DateFormat("S".into()));
    }

    #[test]
    fn test_parse_clipboard() {
        let tokens = parse_template("Hello %clipboard!");
        assert_eq!(tokens.len(), 3);
        assert_eq!(tokens[0], TemplateToken::Literal("Hello ".into()));
        assert_eq!(tokens[1], TemplateToken::Clipboard);
        assert_eq!(tokens[2], TemplateToken::Literal("!".into()));
    }

    #[test]
    fn test_parse_cursor_position() {
        let tokens = parse_template("Dear %|,");
        assert_eq!(tokens.len(), 3);
        assert_eq!(tokens[0], TemplateToken::Literal("Dear ".into()));
        assert_eq!(tokens[1], TemplateToken::CursorPosition);
        assert_eq!(tokens[2], TemplateToken::Literal(",".into()));
    }

    #[test]
    fn test_parse_escaped_percent() {
        let tokens = parse_template("100%%");
        assert_eq!(tokens, vec![TemplateToken::Literal("100%".into())]);
    }

    #[test]
    fn test_parse_date_math() {
        let tokens = parse_template("%date(+5d)");
        assert_eq!(tokens.len(), 1);
        match &tokens[0] {
            TemplateToken::DateMath { offset, unit, .. } => {
                assert_eq!(*offset, 5);
                assert_eq!(*unit, DateMathUnit::Days);
            }
            _ => panic!("Expected DateMath"),
        }
    }

    #[test]
    fn test_parse_date_math_negative() {
        let tokens = parse_template("%date(-1w)");
        match &tokens[0] {
            TemplateToken::DateMath { offset, unit, .. } => {
                assert_eq!(*offset, -1);
                assert_eq!(*unit, DateMathUnit::Weeks);
            }
            _ => panic!("Expected DateMath"),
        }
    }

    #[test]
    fn test_parse_date_math_months() {
        let tokens = parse_template("%date(+3M)");
        match &tokens[0] {
            TemplateToken::DateMath { offset, unit, .. } => {
                assert_eq!(*offset, 3);
                assert_eq!(*unit, DateMathUnit::Months);
            }
            _ => panic!("Expected DateMath"),
        }
    }

    #[test]
    fn test_parse_date_math_years() {
        let tokens = parse_template("%date(+1y)");
        match &tokens[0] {
            TemplateToken::DateMath { offset, unit, .. } => {
                assert_eq!(*offset, 1);
                assert_eq!(*unit, DateMathUnit::Years);
            }
            _ => panic!("Expected DateMath"),
        }
    }

    #[test]
    fn test_parse_invalid_date_math_as_literal() {
        let tokens = parse_template("%date(+5x)");
        // Invalid unit 'x' -- should be left as literal
        assert_eq!(tokens.len(), 1);
        assert_eq!(tokens[0], TemplateToken::Literal("%date(+5x)".into()));
    }

    #[test]
    fn test_parse_unknown_macro_as_literal() {
        let tokens = parse_template("hello %Q world");
        // %Q is unknown -- left as literal "%Q"
        assert_eq!(tokens.len(), 1);
        assert_eq!(
            tokens[0],
            TemplateToken::Literal("hello %Q world".into())
        );
    }

    #[test]
    fn test_parse_percent_at_end() {
        let tokens = parse_template("hello%");
        assert_eq!(tokens, vec![TemplateToken::Literal("hello%".into())]);
    }

    #[test]
    fn test_parse_complex_template() {
        let tokens = parse_template("Meeting on %Y-%m-%d at %H:%M with %clipboard");
        // Should have: "Meeting on ", %Y, "-", %m, "-", %d, " at ", %H, ":", %M, " with ", %clipboard
        assert!(tokens.len() >= 10);
        assert!(tokens.contains(&TemplateToken::Clipboard));
    }

    // --- Evaluation tests ---

    #[test]
    fn test_evaluate_date_year() {
        let now = Local::now();
        let result = expand_template("%Y");
        assert_eq!(result, now.format("%Y").to_string());
    }

    #[test]
    fn test_evaluate_date_full() {
        let now = Local::now();
        let result = expand_template("%Y-%m-%d");
        assert_eq!(result, now.format("%Y-%m-%d").to_string());
    }

    #[test]
    fn test_evaluate_time() {
        let now = Local::now();
        let result = expand_template("%H:%M");
        let expected = now.format("%H:%M").to_string();
        assert_eq!(result, expected);
    }

    #[test]
    fn test_evaluate_clipboard() {
        let ctx = ExpansionContext {
            clipboard_content: "pasted text".to_string(),
            ..Default::default()
        };
        let result = expand_template_with_context("Start: %clipboard :end", &ctx);
        assert_eq!(result.text, "Start: pasted text :end");
    }

    #[test]
    fn test_evaluate_empty_clipboard() {
        let ctx = ExpansionContext {
            clipboard_content: String::new(),
            ..Default::default()
        };
        let result = expand_template_with_context("(%clipboard)", &ctx);
        assert_eq!(result.text, "()");
    }

    #[test]
    fn test_evaluate_cursor_position() {
        let ctx = ExpansionContext::default();
        let result = expand_template_with_context("Dear %|,\nBest regards", &ctx);
        assert_eq!(result.text, "Dear ,\nBest regards");
        assert_eq!(result.cursor_offset, Some(5)); // after "Dear "
    }

    #[test]
    fn test_evaluate_no_cursor() {
        let ctx = ExpansionContext::default();
        let result = expand_template_with_context("Hello world", &ctx);
        assert_eq!(result.text, "Hello world");
        assert!(result.cursor_offset.is_none());
    }

    #[test]
    fn test_evaluate_date_math_days() {
        let now = Local::now();
        let expected = (now + Duration::days(5)).format("%Y-%m-%d").to_string();
        let result = expand_template("%date(+5d)");
        assert_eq!(result, expected);
    }

    #[test]
    fn test_evaluate_date_math_negative_days() {
        let now = Local::now();
        let expected = (now - Duration::days(3)).format("%Y-%m-%d").to_string();
        let result = expand_template("%date(-3d)");
        assert_eq!(result, expected);
    }

    #[test]
    fn test_evaluate_date_math_weeks() {
        let now = Local::now();
        let expected = (now + Duration::weeks(2)).format("%Y-%m-%d").to_string();
        let result = expand_template("%date(+2w)");
        assert_eq!(result, expected);
    }

    #[test]
    fn test_evaluate_escaped_percent() {
        let result = expand_template("100%% complete");
        assert_eq!(result, "100% complete");
    }

    #[test]
    fn test_evaluate_mixed_template() {
        let ctx = ExpansionContext {
            clipboard_content: "World".to_string(),
            ..Default::default()
        };
        let result = expand_template_with_context("Hello %clipboard on %Y-%m-%d", &ctx);
        let now = Local::now();
        let expected = format!("Hello World on {}", now.format("%Y-%m-%d"));
        assert_eq!(result.text, expected);
    }

    #[test]
    fn test_evaluate_plain_text_unchanged() {
        let result = expand_template("No macros here");
        assert_eq!(result, "No macros here");
    }

    // --- Date math edge cases ---

    #[test]
    fn test_days_in_month() {
        assert_eq!(days_in_month(2024, 2), 29); // leap year
        assert_eq!(days_in_month(2023, 2), 28); // non-leap
        assert_eq!(days_in_month(2024, 1), 31);
        assert_eq!(days_in_month(2024, 4), 30);
    }

    #[test]
    fn test_date_math_month_overflow() {
        // Adding months should handle year rollover
        let _tokens = parse_template("%date(+13M)");
        // Should produce a valid date ~13 months from now
        let result = expand_template("%date(+13M)");
        // Just verify it's a valid date format
        assert_eq!(result.len(), 10); // YYYY-MM-DD
        assert!(result.contains('-'));
    }

    // --- Backward compatibility ---

    #[test]
    fn test_expand_template_still_works() {
        // The convenience function should still work for plain text
        let result = expand_template("Hello");
        assert_eq!(result, "Hello");
    }

    #[test]
    fn test_multiline_template() {
        let template = "Line 1\nLine 2\nLine 3";
        let result = expand_template(template);
        assert_eq!(result, template);
    }

    #[test]
    fn test_empty_template() {
        let result = expand_template("");
        assert_eq!(result, "");
    }

    // --- Fill-in field tests ---

    #[test]
    fn test_parse_fill_in() {
        let tokens = parse_template("Hello %fill(name)!");
        assert_eq!(tokens.len(), 3);
        match &tokens[1] {
            TemplateToken::FillIn(spec) => {
                assert_eq!(spec.name, "name");
                assert!(spec.default_value.is_none());
            }
            _ => panic!("Expected FillIn"),
        }
    }

    #[test]
    fn test_parse_fill_in_with_default() {
        let tokens = parse_template("%fill(name:default=World)");
        match &tokens[0] {
            TemplateToken::FillIn(spec) => {
                assert_eq!(spec.name, "name");
                assert_eq!(spec.default_value.as_deref(), Some("World"));
            }
            _ => panic!("Expected FillIn"),
        }
    }

    #[test]
    fn test_parse_fillarea() {
        let tokens = parse_template("%fillarea(notes)");
        match &tokens[0] {
            TemplateToken::FillArea(spec) => {
                assert_eq!(spec.name, "notes");
            }
            _ => panic!("Expected FillArea"),
        }
    }

    #[test]
    fn test_parse_fillpopup() {
        let tokens = parse_template("%fillpopup(tone:Professional:Casual:Formal)");
        match &tokens[0] {
            TemplateToken::FillPopup(spec) => {
                assert_eq!(spec.name, "tone");
                assert_eq!(spec.options, vec!["Professional", "Casual", "Formal"]);
            }
            _ => panic!("Expected FillPopup"),
        }
    }

    #[test]
    fn test_evaluate_fill_in_with_values() {
        let mut ctx = ExpansionContext::default();
        ctx.fill_values.insert("name".into(), "Alice".into());
        let result = expand_template_with_context("Hello %fill(name)!", &ctx);
        assert_eq!(result.text, "Hello Alice!");
    }

    #[test]
    fn test_evaluate_fill_in_default() {
        let ctx = ExpansionContext::default();
        let result = expand_template_with_context("Hello %fill(name:default=World)!", &ctx);
        assert_eq!(result.text, "Hello World!");
    }

    #[test]
    fn test_evaluate_fillpopup_with_value() {
        let mut ctx = ExpansionContext::default();
        ctx.fill_values.insert("tone".into(), "Casual".into());
        let result =
            expand_template_with_context("Tone: %fillpopup(tone:Pro:Casual:Formal)", &ctx);
        assert_eq!(result.text, "Tone: Casual");
    }

    #[test]
    fn test_evaluate_fillpopup_default_first() {
        let ctx = ExpansionContext::default();
        let result = expand_template_with_context("%fillpopup(tone:Pro:Casual)", &ctx);
        assert_eq!(result.text, "Pro");
    }

    #[test]
    fn test_extract_fill_in_fields() {
        let tokens =
            parse_template("Dear %fill(name), %fillpopup(tone:Pro:Casual). %fillarea(body)");
        let fields = extract_fill_in_fields(&tokens);
        assert_eq!(fields.len(), 3);
    }

    #[test]
    fn test_extract_deduplicates() {
        let tokens = parse_template("%fill(name) and %fill(name) again");
        let fields = extract_fill_in_fields(&tokens);
        assert_eq!(fields.len(), 1); // same name, deduplicated
    }

    #[test]
    fn test_has_fill_in_fields() {
        assert!(has_fill_in_fields("Hello %fill(name)"));
        assert!(!has_fill_in_fields("Hello %Y-%m-%d"));
    }

    // --- Shell command tests ---

    #[test]
    fn test_parse_shell_command() {
        let tokens = parse_template("%shell(echo hello)");
        assert_eq!(tokens.len(), 1);
        assert_eq!(tokens[0], TemplateToken::ShellCommand("echo hello".into()));
    }

    #[test]
    fn test_parse_shell_with_nested_parens() {
        let tokens = parse_template("%shell(echo $(date))");
        assert_eq!(tokens.len(), 1);
        assert_eq!(
            tokens[0],
            TemplateToken::ShellCommand("echo $(date)".into())
        );
    }

    #[test]
    fn test_parse_shell_unclosed() {
        let tokens = parse_template("%shell(echo hello");
        assert_eq!(tokens.len(), 1);
        // Unclosed paren — treated as literal
        assert_eq!(
            tokens[0],
            TemplateToken::Literal("%shell(echo hello".into())
        );
    }

    #[test]
    fn test_parse_snippet() {
        let tokens = parse_template("Hello %snippet(;sig)!");
        assert_eq!(tokens.len(), 3);
        assert_eq!(tokens[0], TemplateToken::Literal("Hello ".into()));
        assert_eq!(tokens[1], TemplateToken::NestedSnippet(";sig".into()));
        assert_eq!(tokens[2], TemplateToken::Literal("!".into()));
    }

    #[test]
    fn test_parse_snippet_empty_name() {
        let tokens = parse_template("%snippet()");
        // Empty name — treated as literal
        assert_eq!(tokens.len(), 1);
        assert_eq!(tokens[0], TemplateToken::Literal("%snippet(".into()));
    }

    #[test]
    fn test_parse_shell_in_context() {
        let tokens = parse_template("Today is %shell(date +%%F) ok");
        assert_eq!(tokens.len(), 3);
        assert_eq!(tokens[0], TemplateToken::Literal("Today is ".into()));
        assert_eq!(
            tokens[1],
            TemplateToken::ShellCommand("date +%%F".into())
        );
        assert_eq!(tokens[2], TemplateToken::Literal(" ok".into()));
    }

    #[test]
    fn test_evaluate_shell_echo() {
        let result = expand_template("%shell(echo hello)");
        assert_eq!(result, "hello");
    }

    #[test]
    fn test_evaluate_shell_strips_trailing_newline() {
        let result = expand_template("%shell(printf 'hello\\n')");
        assert_eq!(result, "hello");
    }

    #[test]
    fn test_evaluate_shell_failure() {
        let result = expand_template("%shell(false)");
        assert!(result.starts_with("[shell error:"));
    }

    #[test]
    fn test_evaluate_shell_multiline() {
        let result = expand_template("%shell(printf 'line1\\nline2')");
        assert_eq!(result, "line1\nline2");
    }

    // --- Nested snippet tests ---

    #[test]
    fn test_evaluate_nested_snippet() {
        let lookup: Arc<dyn Fn(&str) -> Option<String> + Send + Sync> =
            Arc::new(|abbr: &str| match abbr {
                ";sig" => Some("Best regards".into()),
                _ => None,
            });
        let ctx = ExpansionContext {
            snippet_lookup: Some(lookup),
            ..Default::default()
        };
        let result = expand_template_with_context("Hello, %snippet(;sig)", &ctx);
        assert_eq!(result.text, "Hello, Best regards");
    }

    #[test]
    fn test_nested_snippet_not_found() {
        let lookup: Arc<dyn Fn(&str) -> Option<String> + Send + Sync> = Arc::new(|_| None);
        let ctx = ExpansionContext {
            snippet_lookup: Some(lookup),
            ..Default::default()
        };
        let result = expand_template_with_context("%snippet(;nope)", &ctx);
        assert!(result.text.contains("not found"));
    }

    #[test]
    fn test_nested_snippet_cycle_detection() {
        let lookup: Arc<dyn Fn(&str) -> Option<String> + Send + Sync> =
            Arc::new(|abbr: &str| match abbr {
                ";a" => Some("A then %snippet(;b)".into()),
                ";b" => Some("B then %snippet(;a)".into()),
                _ => None,
            });
        let ctx = ExpansionContext {
            snippet_lookup: Some(lookup),
            ..Default::default()
        };
        let result = expand_template_with_context("%snippet(;a)", &ctx);
        assert!(result.text.contains("circular reference"));
    }

    #[test]
    fn test_nested_snippet_max_depth() {
        // Use a chain of different abbreviations to avoid cycle detection
        // and test max depth instead: ;a -> ;b -> ;c -> ;d (depth exceeds 3)
        let lookup: Arc<dyn Fn(&str) -> Option<String> + Send + Sync> =
            Arc::new(|abbr: &str| match abbr {
                ";a" => Some("A-%snippet(;b)".into()),
                ";b" => Some("B-%snippet(;c)".into()),
                ";c" => Some("C-%snippet(;d)".into()),
                ";d" => Some("D-%snippet(;e)".into()),
                ";e" => Some("E".into()),
                _ => None,
            });
        let ctx = ExpansionContext {
            snippet_lookup: Some(lookup),
            max_depth: 3,
            ..Default::default()
        };
        let result = expand_template_with_context("%snippet(;a)", &ctx);
        assert!(result.text.contains("max depth"));
    }

    #[test]
    fn test_nested_snippet_multi_level() {
        let lookup: Arc<dyn Fn(&str) -> Option<String> + Send + Sync> =
            Arc::new(|abbr: &str| match abbr {
                ";phone" => Some("555-1234".into()),
                ";sig" => Some("John\nPhone: %snippet(;phone)".into()),
                _ => None,
            });
        let ctx = ExpansionContext {
            snippet_lookup: Some(lookup),
            ..Default::default()
        };
        let result = expand_template_with_context("%snippet(;sig)", &ctx);
        assert_eq!(result.text, "John\nPhone: 555-1234");
    }

    #[test]
    fn test_no_lookup_configured() {
        let ctx = ExpansionContext::default();
        let result = expand_template_with_context("%snippet(;test)", &ctx);
        assert!(result.text.contains("no lookup"));
    }

    #[test]
    fn test_shell_and_snippet_combined() {
        let lookup: Arc<dyn Fn(&str) -> Option<String> + Send + Sync> =
            Arc::new(|abbr: &str| match abbr {
                ";name" => Some("Alice".into()),
                _ => None,
            });
        let ctx = ExpansionContext {
            snippet_lookup: Some(lookup),
            ..Default::default()
        };
        let result =
            expand_template_with_context("Hi %snippet(;name), today is %shell(echo Monday)", &ctx);
        assert_eq!(result.text, "Hi Alice, today is Monday");
    }
}
