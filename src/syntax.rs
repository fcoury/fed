use std::{cmp, collections::HashMap, io::stdout, num::ParseIntError, str::FromStr};

use crossterm::{
    cursor,
    style::{self},
    QueueableCommand,
};
use lazy_static::lazy_static;
use strum_macros::{Display, EnumString};
use tree_sitter_highlight::{HighlightConfiguration, HighlightEvent, Highlighter};

use crate::{log, theme::Theme};

const HIGHLIGHT_NAMES: [&str; 52] = [
    "attribute",
    "boolean",
    "carriage-return",
    "comment",
    "comment.documentation",
    "constant",
    "constant.builtin",
    "constructor",
    "constructor.builtin",
    "embedded",
    "error",
    "escape",
    "function",
    "function.builtin",
    "keyword",
    "markup",
    "markup.bold",
    "markup.heading",
    "markup.italic",
    "markup.link",
    "markup.link.url",
    "markup.list",
    "markup.list.checked",
    "markup.list.numbered",
    "markup.list.unchecked",
    "markup.list.unnumbered",
    "markup.quote",
    "markup.raw",
    "markup.raw.block",
    "markup.raw.inline",
    "markup.strikethrough",
    "module",
    "number",
    "operator",
    "property",
    "property.builtin",
    "punctuation",
    "punctuation.bracket",
    "punctuation.delimiter",
    "punctuation.special",
    "string",
    "string.escape",
    "string.regexp",
    "string.special",
    "string.special.symbol",
    "tag",
    "type",
    "type.builtin",
    "variable",
    "variable.builtin",
    "variable.member",
    "variable.parameter",
];

lazy_static! {
    static ref TS_TO_THEME: HashMap<String, String> = HashMap::from_iter(
        vec![
            ("attribute", "entity.other.attribute-name"),
            ("boolean", "constant.language.boolean"),
            ("carriage-return", "No direct equivalent"),
            ("comment", "comment"),
            ("comment.documentation", "comment.block.documentation"),
            ("constant", "constant"),
            ("constant.builtin", "constant.language"),
            ("constructor", "entity.name.function.constructor"),
            ("constructor.builtin", "No direct equivalent"),
            ("embedded", "text.html"),
            ("error", "invalid"),
            ("escape", "constant.character.escape"),
            ("function", "entity.name.function"),
            ("function.builtin", "support.function"),
            ("keyword", "keyword"),
            ("markup", "markup"),
            ("markup.bold", "markup.bold"),
            ("markup.heading", "markup.heading"),
            ("markup.italic", "markup.italic"),
            ("markup.link", "markup.underline.link"),
            ("markup.link.url", "markup.underline.link"),
            ("markup.list", "markup.list"),
            ("markup.list.checked", "No direct equivalent"),
            ("markup.list.numbered", "markup.list.numbered"),
            ("markup.list.unchecked", "No direct equivalent"),
            ("markup.list.unnumbered", "markup.list.unnumbered"),
            ("markup.quote", "markup.quote"),
            ("markup.raw", "markup.raw"),
            ("markup.raw.block", "markup.raw.block"),
            ("markup.raw.inline", "markup.raw.inline"),
            ("markup.strikethrough", "markup.strikethrough"),
            ("module", "No direct equivalent"),
            ("number", "constant.numeric"),
            ("operator", "keyword.operator"),
            ("property", "variable.other.property"),
            ("property.builtin", "support.type.property-name"),
            ("punctuation", "punctuation"),
            ("punctuation.bracket", "punctuation.section"),
            ("punctuation.delimiter", "punctuation.separator"),
            ("punctuation.special", "No direct equivalent"),
            ("string", "string"),
            ("string.escape", "constant.character.escape"),
            ("string.regexp", "string.regexp"),
            ("string.special", "No direct equivalent"),
            ("string.special.symbol", "No direct equivalent"),
            ("tag", "entity.name.tag"),
            ("type", "entity.name.type"),
            ("type.builtin", "support.type"),
            ("variable", "variable"),
            ("variable.builtin", "variable.language"),
            ("variable.member", "variable.other.member"),
            ("variable.parameter", "variable.parameter"),
        ]
        .iter()
        .map(|(k, v)| (k.to_string(), v.to_string()))
    );
}

#[derive(Debug, PartialEq, EnumString, Display)]
#[strum(serialize_all = "snake_case")]
enum ChunkType {
    None,
    Attribute,
    Boolean,
    CarriageReturn,
    Comment,
    #[strum(serialize = "comment.documentation")]
    CommentDocumentation,
    Constant,
    #[strum(serialize = "constant.builtin")]
    ConstantBuiltin,
    Constructor,
    #[strum(serialize = "constructor.builtin")]
    ConstructorBuiltin,
    Embedded,
    Error,
    Escape,
    Function,
    #[strum(serialize = "function.builtin")]
    FunctionBuiltin,
    Keyword,
    Markup,
    #[strum(serialize = "markup.bold")]
    MarkupBold,
    #[strum(serialize = "markup.heading")]
    MarkupHeading,
    #[strum(serialize = "markup.italic")]
    MarkupItalic,
    #[strum(serialize = "markup.link")]
    MarkupLink,
    #[strum(serialize = "markup.link.url")]
    MarkupLinkUrl,
    #[strum(serialize = "markup.list")]
    MarkupList,
    #[strum(serialize = "markup.list.checked")]
    MarkupListChecked,
    #[strum(serialize = "markup.list.numbered")]
    MarkupListNumbered,
    #[strum(serialize = "markup.list.unchecked")]
    MarkupListUnchecked,
    #[strum(serialize = "markup.list.unnumbered")]
    MarkupListUnnumbered,
    #[strum(serialize = "markup.quote")]
    MarkupQuote,
    #[strum(serialize = "markup.raw")]
    MarkupRaw,
    #[strum(serialize = "markup.raw.block")]
    MarkupRawBlock,
    #[strum(serialize = "markup.raw.inline")]
    MarkupRawInline,
    #[strum(serialize = "markup.strikethrough")]
    MarkupStrikethrough,
    Module,
    Number,
    Operator,
    Property,
    #[strum(serialize = "property.builtin")]
    PropertyBuiltin,
    Punctuation,
    #[strum(serialize = "punctuation.bracket")]
    PunctuationBracket,
    #[strum(serialize = "punctuation.delimiter")]
    PunctuationDelimiter,
    #[strum(serialize = "punctuation.special")]
    PunctuationSpecial,
    String,
    #[strum(serialize = "string.escape")]
    StringEscape,
    #[strum(serialize = "string.regexp")]
    StringRegexp,
    #[strum(serialize = "string.special")]
    StringSpecial,
    #[strum(serialize = "string.special.symbol")]
    StringSpecialSymbol,
    Tag,
    Type,
    #[strum(serialize = "type.builtin")]
    TypeBuiltin,
    Variable,
    #[strum(serialize = "variable.builtin")]
    VariableBuiltin,
    #[strum(serialize = "variable.member")]
    VariableMember,
    #[strum(serialize = "variable.parameter")]
    VariableParameter,
}

#[derive(Debug)]
struct Chunk<'a> {
    contents: &'a str,
    start: usize,
    end: usize,
    typ: ChunkType,
}

impl Default for Chunk<'_> {
    fn default() -> Self {
        Chunk {
            contents: "",
            start: 0,
            end: 0,
            typ: ChunkType::None,
        }
    }
}

pub struct Viewport {
    top: usize,
    left: usize,
    width: usize,
    height: usize,
}

impl Viewport {
    pub fn new(top: usize, left: usize, width: usize, height: usize) -> Self {
        Viewport {
            top,
            left,
            width,
            height,
        }
    }

    pub fn clamp_lines<'a>(&self, buffer: &'a [String]) -> anyhow::Result<&'a [String]> {
        let y0 = self.top;
        let y1 = cmp::min(self.top + self.height, buffer.len());
        Ok(&buffer[y0..y1])
    }
}

pub fn rust_parser() -> HighlightConfiguration {
    let rust_language = tree_sitter_rust::language();

    let mut rust_config = HighlightConfiguration::new(
        rust_language,
        tree_sitter_rust::HIGHLIGHT_QUERY,
        tree_sitter_rust::INJECTIONS_QUERY,
        "",
    )
    .unwrap();

    rust_config.configure(&HIGHLIGHT_NAMES);
    rust_config
}

fn hex_to_crossterm_color(hex: &str) -> Result<style::Color, ParseIntError> {
    let hex = hex.trim_start_matches('#');

    let r = u8::from_str_radix(&hex[0..2], 16)?;
    let g = u8::from_str_radix(&hex[2..4], 16)?;
    let b = u8::from_str_radix(&hex[4..6], 16)?;

    Ok(style::Color::Rgb { r, g, b })
}

pub fn highlight(buffer: &[String], theme: &Theme, viewport: &Viewport) -> anyhow::Result<()> {
    let rust_parser = rust_parser();
    let visible_lines = viewport.clamp_lines(buffer)?;

    for (y, line) in visible_lines.iter().enumerate() {
        let fg = hex_to_crossterm_color(&theme.foreground)?;
        let bg = hex_to_crossterm_color(&theme.background)?;

        stdout().queue(style::SetForegroundColor(fg))?;
        stdout().queue(style::SetBackgroundColor(bg))?;
        stdout().queue(cursor::MoveTo(0, y as u16))?;
        stdout().queue(style::Print(" ".repeat(viewport.width)))?;
        stdout().queue(cursor::MoveTo(0, y as u16))?;

        let chunks = parse(line, &rust_parser)?;
        for chunk in chunks {
            let chunk_type = chunk.typ.to_string();
            if let Some(scope) = TS_TO_THEME.get(&chunk_type) {
                if let Some(setting) = theme.get_scope(scope) {
                    if let Some(fg) = &setting.settings.foreground {
                        let fg = hex_to_crossterm_color(fg)?;
                        stdout().queue(style::SetForegroundColor(fg))?;
                    }

                    if let Some(bg) = &setting.settings.background {
                        let bg = hex_to_crossterm_color(bg)?;
                        stdout().queue(style::SetBackgroundColor(bg))?;
                    }
                }
            }

            log!("chunk: {:?}", chunk.typ);
            stdout().queue(style::Print(chunk.contents))?;
        }
    }

    Ok(())
}

fn parse<'a>(
    source: &'a str,
    lang_config: &'a HighlightConfiguration,
) -> anyhow::Result<Vec<Chunk<'a>>> {
    let mut highlighter = Highlighter::new();

    let highlights = highlighter
        .highlight(&lang_config, source.as_bytes(), None, |_| None)
        .unwrap();

    let mut chunks = vec![];
    let mut chunk = Chunk::default();

    for event in highlights {
        match event.unwrap() {
            HighlightEvent::Source { start, end } => {
                chunk.contents = &source[start..end];
                chunk.start = start;
                chunk.end = end;
            }
            HighlightEvent::HighlightStart(s) => {
                if !chunk.contents.is_empty() {
                    // Push the previous chunk if it has content
                    chunks.push(chunk);
                    chunk = Chunk::default();
                }

                chunk.typ = ChunkType::from_str(HIGHLIGHT_NAMES[s.0]).unwrap();
            }
            HighlightEvent::HighlightEnd => {
                chunks.push(chunk);
                chunk = Chunk::default();
            }
        }
    }

    Ok(chunks)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_highlight() {
        let theme = Theme::parse("src/fixtures/GitHub.tmTheme").unwrap();
        let viewport = Viewport {
            top: 0,
            left: 0,
            width: 80,
            height: 24,
        };

        let buffer = r#"
        fn main() {
            println!("Hello, world!");
        }
        "#
        .lines()
        .map(|s| s.to_string())
        .collect::<Vec<String>>();

        highlight(&buffer, &theme, &viewport).unwrap();
    }

    #[test]
    fn test_parse() {
        let javascript_language = tree_sitter_javascript::language();

        let mut javascript_config = HighlightConfiguration::new(
            javascript_language,
            tree_sitter_javascript::HIGHLIGHT_QUERY,
            tree_sitter_javascript::INJECTION_QUERY,
            tree_sitter_javascript::LOCALS_QUERY,
        )
        .unwrap();

        javascript_config.configure(&HIGHLIGHT_NAMES);

        let source = r#"
        function x() { 
            let x = 1 + 2; 
        }
        "#;

        let chunks = parse(&source, &mut javascript_config).unwrap();
        assert_eq!(chunks.len(), 23);
        assert_eq!(chunks[0].typ, ChunkType::None); // space and return before function
        assert_eq!(chunks[1].typ, ChunkType::Keyword);
        assert_eq!(chunks[1].contents, "function");
    }
}
