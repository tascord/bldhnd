//! Syntax-highlighted code block.
//!
//! ```rust
//! use bobatea::components::syntax::SyntaxBlock;
//! let block = SyntaxBlock::new("rust", "fn main() {\n    println!(\"hello\");\n}");
//! ```

use {
    crate::components::{Component, block::BobaBlock, style::BobaStyle},
    ratatui::{
        prelude::{Buffer, Frame, Rect},
        style::Color,
        text::{Line, Span},
        widgets::{Paragraph, Widget},
    },
    std::collections::HashSet,
};

/// Simple syntax highlighter for common tokens.
pub struct SyntaxBlock {
    lang: String,
    code: String,
    show_line_numbers: bool,
}

impl SyntaxBlock {
    pub fn new(lang: impl Into<String>, code: impl Into<String>) -> Self {
        Self { lang: lang.into(), code: code.into(), show_line_numbers: true }
    }

    pub fn without_line_numbers(mut self) -> Self {
        self.show_line_numbers = false;
        self
    }

    fn tokenize_rust(&self) -> Vec<Line<'static>> {
        let keywords: HashSet<&str> = [
            "fn", "let", "mut", "if", "else", "for", "while", "loop", "match", "use", "pub", "struct", "enum", "impl",
            "return", "async", "await", "const", "static", "where", "trait", "type", "move", "ref", "self", "Self", "super",
            "crate", "as", "in", "break", "continue", "yield", "unsafe", "extern", "box", "dyn",
        ]
        .iter()
        .cloned()
        .collect();
        let types: HashSet<&str> =
            ["String", "Vec", "Option", "Result", "i32", "u32", "f64", "bool", "char", "str", "usize", "isize"]
                .iter()
                .cloned()
                .collect();
        let builtins: HashSet<&str> =
            ["println!", "print!", "format!", "vec!", "assert!", "panic!", "todo!", "unreachable!"]
                .iter()
                .cloned()
                .collect();

        self.tokenize_with_sets(&keywords, &types, &builtins, "//")
    }

    fn tokenize_python(&self) -> Vec<Line<'static>> {
        let keywords: HashSet<&str> = [
            "def", "class", "if", "elif", "else", "for", "while", "return", "import", "from", "as", "with", "try", "except",
            "finally", "raise", "pass", "break", "continue", "lambda", "yield", "await", "async", "global", "nonlocal",
            "assert", "del", "in", "is", "not", "and", "or", "True", "False", "None",
        ]
        .iter()
        .cloned()
        .collect();
        let types: HashSet<&str> =
            ["str", "int", "float", "list", "dict", "set", "tuple", "bool", "bytes"].iter().cloned().collect();
        self.tokenize_with_sets(&keywords, &types, &HashSet::new(), "#")
    }

    fn tokenize_js(&self) -> Vec<Line<'static>> {
        let keywords: HashSet<&str> = [
            "function",
            "const",
            "let",
            "var",
            "if",
            "else",
            "for",
            "while",
            "return",
            "import",
            "export",
            "from",
            "class",
            "extends",
            "new",
            "this",
            "super",
            "try",
            "catch",
            "finally",
            "throw",
            "typeof",
            "instanceof",
            "void",
            "delete",
            "in",
            "of",
            "await",
            "async",
            "yield",
            "true",
            "false",
            "null",
            "undefined",
        ]
        .iter()
        .cloned()
        .collect();
        let types: HashSet<&str> = HashSet::new();
        self.tokenize_with_sets(&keywords, &types, &HashSet::new(), "//")
    }

    fn tokenize_go(&self) -> Vec<Line<'static>> {
        let keywords: HashSet<&str> = [
            "func",
            "package",
            "import",
            "var",
            "const",
            "type",
            "struct",
            "interface",
            "map",
            "chan",
            "go",
            "return",
            "if",
            "else",
            "for",
            "range",
            "switch",
            "case",
            "default",
            "break",
            "continue",
            "defer",
        ]
        .iter()
        .cloned()
        .collect();
        self.tokenize_with_sets(&keywords, &HashSet::new(), &HashSet::new(), "//")
    }

    fn tokenize_c(&self) -> Vec<Line<'static>> {
        let keywords: HashSet<&str> = [
            "int", "char", "float", "double", "void", "struct", "enum", "union", "typedef", "const", "static", "extern",
            "return", "if", "else", "for", "while", "do", "switch", "case", "default", "break", "continue", "sizeof",
            "goto", "register", "volatile", "signed", "unsigned", "long", "short", "auto", "inline",
        ]
        .iter()
        .cloned()
        .collect();
        self.tokenize_with_sets(&keywords, &HashSet::new(), &HashSet::new(), "//")
    }

    fn tokenize_bash(&self) -> Vec<Line<'static>> {
        let keywords: HashSet<&str> = [
            "if", "then", "else", "elif", "fi", "for", "in", "do", "done", "while", "until", "case", "esac", "function",
            "return", "break", "continue", "exit", "export", "local", "readonly", "shift", "source",
        ]
        .iter()
        .cloned()
        .collect();
        self.tokenize_with_sets(&keywords, &HashSet::new(), &HashSet::new(), "#")
    }

    fn tokenize_json(&self) -> Vec<Line<'static>> { self.tokenize_plain() }

    fn tokenize_markdown(&self) -> Vec<Line<'static>> { self.tokenize_plain() }

    fn tokenize_html(&self) -> Vec<Line<'static>> { self.tokenize_plain() }

    fn tokenize_with_sets(
        &self,
        keywords: &HashSet<&str>,
        types: &HashSet<&str>,
        builtins: &HashSet<&str>,
        comment: &str,
    ) -> Vec<Line<'static>> {
        self.code
            .lines()
            .enumerate()
            .map(|(i, line)| {
                let mut spans: Vec<Span<'static>> = vec![];

                if self.show_line_numbers {
                    spans.push(Span::styled(format!("{:4} ", i + 1), BobaStyle::new().muted()));
                }

                let mut remaining: String = line.to_string();
                while !remaining.is_empty() {
                    if remaining.starts_with(comment) {
                        spans.push(Span::styled(remaining, BobaStyle::new().fg(Color::DarkGray).italic()));
                        break;
                    }
                    if remaining.starts_with('"') || remaining.starts_with('\'') {
                        let quote = remaining.chars().next().unwrap();
                        let end = remaining[1..].find(quote).map(|i| i + 2).unwrap_or(remaining.len());
                        let piece: String = remaining.drain(..end).collect();
                        spans.push(Span::styled(piece, BobaStyle::new().fg(Color::Green)));
                        continue;
                    }

                    let mut found = false;
                    for kw in keywords {
                        if remaining.starts_with(kw)
                            && !remaining[kw.len()..].starts_with(|c: char| c.is_alphanumeric() || c == '_')
                        {
                            let kw_len = kw.len();
                            let piece: String = remaining.drain(..kw_len).collect();
                            spans.push(Span::styled(piece, BobaStyle::new().fg(Color::Cyan).bold()));
                            found = true;
                            break;
                        }
                    }
                    if found {
                        continue;
                    }

                    for ty in types {
                        if remaining.starts_with(ty) {
                            let ty_len = ty.len();
                            let piece: String = remaining.drain(..ty_len).collect();
                            spans.push(Span::styled(piece, BobaStyle::new().fg(Color::Yellow)));
                            found = true;
                            break;
                        }
                    }
                    if found {
                        continue;
                    }

                    for b in builtins {
                        if remaining.starts_with(b) {
                            let b_len = b.len();
                            let piece: String = remaining.drain(..b_len).collect();
                            spans.push(Span::styled(piece, BobaStyle::new().fg(Color::Magenta)));
                            found = true;
                            break;
                        }
                    }
                    if found {
                        continue;
                    }

                    // Chomp one token-ish chunk
                    let end = remaining
                        .find(|c: char| c.is_whitespace() || c == '"' || c == '\'')
                        .unwrap_or(remaining.len())
                        .max(1);
                    let piece: String = remaining.drain(..end).collect();
                    spans.push(Span::raw(piece));
                }

                Line::from(spans)
            })
            .collect()
    }

    fn tokenize_plain(&self) -> Vec<Line<'static>> {
        self.code
            .lines()
            .enumerate()
            .map(|(i, line)| {
                let mut spans = vec![];
                if self.show_line_numbers {
                    spans.push(Span::styled(format!("{:4} ", i + 1), BobaStyle::new().muted()));
                }
                spans.push(Span::raw(line.to_string()));
                Line::from(spans)
            })
            .collect()
    }

    fn tokenize(&self) -> Vec<Line<'static>> {
        match self.lang.as_str() {
            "rust" | "rs" => self.tokenize_rust(),
            "python" | "py" => self.tokenize_python(),
            "javascript" | "js" | "typescript" | "ts" => self.tokenize_js(),
            "go" | "golang" => self.tokenize_go(),
            "c" | "cpp" | "c++" | "cxx" | "h" | "hpp" => self.tokenize_c(),
            "bash" | "sh" | "shell" | "zsh" => self.tokenize_bash(),
            "json" => self.tokenize_json(),
            "markdown" | "md" => self.tokenize_markdown(),
            "html" => self.tokenize_html(),
            _ => self.tokenize_plain(),
        }
    }

    pub fn render_to_buf(&self, area: Rect, buf: &mut Buffer, theme: &crate::theme::Theme) {
        for y in area.top()..area.bottom() {
            for x in area.left()..area.right() {
                buf[(x, y)].set_bg(theme.global_bg);
            }
        }

        let lines = self.tokenize();
        let border_style = ratatui::style::Style::default().fg(theme.border_accent);
        let block = BobaBlock::new().rounded().border_style(border_style).title(format!(" {} ", self.lang));
        let block: ratatui::widgets::Block<'_> = block.into();

        Paragraph::new(lines).style(ratatui::style::Style::default().bg(theme.global_bg)).block(block).render(area, buf);
    }
}

impl Component for SyntaxBlock {
    fn render(&mut self, ctx: &mut Frame<'_>, theme: &crate::theme::Theme) {
        let area = ctx.area();
        self.render_to_buf(area, ctx.buffer_mut(), theme);
    }
}
