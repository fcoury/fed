use std::{cmp, collections::HashMap, io::stdout, str::FromStr};

use crossterm::{
    cursor,
    style::{self},
    QueueableCommand,
};
use lazy_static::lazy_static;
use strum_macros::{Display, EnumString};
use tree_sitter_highlight::{HighlightConfiguration, HighlightEvent, Highlighter};

use crate::{log, theme::Theme, utils::hex_to_crossterm_color};

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

#[derive(Clone, Debug, PartialEq, EnumString, Display)]
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

#[derive(Debug, Clone)]
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

impl<'a> Chunk<'a> {
    fn with_type(typ: ChunkType) -> Self {
        Chunk {
            typ,
            ..Chunk::default()
        }
    }

    fn from_source(start: usize, end: usize, contents: &'a str) -> Self {
        Chunk {
            contents,
            start,
            end,
            ..Chunk::default()
        }
    }

    fn split(&self, ch: char) -> Vec<Chunk<'a>> {
        let mut chunks = vec![];

        for (i, line) in self.contents.split(ch).enumerate() {
            let mut chunk = Chunk::from_source(self.start, self.end, line);
            if i == 0 {
                chunk.typ = self.typ.clone();
            }
            chunks.push(chunk);
        }

        chunks
    }
}

#[derive(Debug, Clone)]
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

    pub fn clamp_lines<'a, T>(&self, buffer: &'a [T]) -> anyhow::Result<&'a [T]> {
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

fn split_chunks(chunks: Vec<Chunk>) -> Vec<Vec<Chunk>> {
    let mut lines: Vec<Vec<Chunk>> = vec![];
    let mut current_line: Vec<Chunk> = vec![];

    for chunk in chunks {
        if chunk.contents.contains('\n') {
            let mut chunks = chunk.split('\n');
            // pushes the first chunk to the current line
            current_line.push(chunks.remove(0));
            // this line is done, because the previous chunk ended with \n
            lines.push(current_line);

            // pushes all the intermediary items alone to lines
            for chunk in chunks[0..chunks.len() - 1].iter() {
                lines.push(vec![chunk.clone()]);
            }

            // and the last item to current_line
            current_line = vec![chunks[chunks.len() - 1].clone()];
        } else {
            current_line.push(chunk);
        }
    }

    lines.push(current_line);

    lines
}

fn clear_line(theme: &Theme, viewport: &Viewport) -> anyhow::Result<()> {
    let fg = hex_to_crossterm_color(&theme.foreground)?;
    let bg = hex_to_crossterm_color(&theme.background)?;

    stdout().queue(style::SetForegroundColor(fg))?;
    stdout().queue(style::SetBackgroundColor(bg))?;

    stdout().queue(cursor::MoveToColumn(viewport.left as u16))?;
    stdout().queue(style::Print(" ".repeat(viewport.width)))?;
    stdout().queue(cursor::MoveToColumn(viewport.left as u16))?;

    Ok(())
}

pub fn highlight(buffer: &[String], theme: &Theme, viewport: &Viewport) -> anyhow::Result<()> {
    let rust_parser = rust_parser();
    let buffer = buffer.join("\n");
    let chunks = parse(&buffer, &rust_parser)?;
    let chunks = split_chunks(chunks);
    let lines = viewport.clamp_lines(&chunks)?;

    stdout().queue(cursor::MoveTo(viewport.left as u16, 0))?;

    for line in lines {
        clear_line(theme, viewport)?;

        for chunk in line.iter() {
            let chunk_type = chunk.typ.to_string();
            let mut fg = &theme.foreground;
            let mut bg = &theme.background;

            // checks for the theme color
            if let Some(scope) = TS_TO_THEME.get(&chunk_type) {
                if let Some(setting) = theme.get_scope(scope) {
                    if let Some(setting_fg) = &setting.settings.foreground {
                        fg = setting_fg;
                    }

                    if let Some(setting_bg) = &setting.settings.background {
                        bg = setting_bg;
                    }
                }
            }

            let setting_fg = hex_to_crossterm_color(fg)?;
            let setting_bg = hex_to_crossterm_color(bg)?;
            stdout().queue(style::SetForegroundColor(setting_fg))?;
            stdout().queue(style::SetBackgroundColor(setting_bg))?;

            // log!("chunk {:?}: {:?} {fg}:{bg}", chunk.typ, chunk.contents);
            stdout().queue(style::Print(chunk.contents))?;
        }

        stdout().queue(cursor::MoveToNextLine(1))?;
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
    let mut chunk: Option<Chunk<'_>> = None;

    for event in highlights {
        let event = event?;
        match event {
            HighlightEvent::Source { start, end } => {
                if let Some(ref mut chunk) = chunk {
                    chunk.contents = &source[start..end];
                    chunk.start = start;
                    chunk.end = end;
                } else {
                    chunk = Some(Chunk::from_source(start, end, &source[start..end]));
                }
            }
            HighlightEvent::HighlightStart(s) => {
                if let Some(chunk) = chunk.take() {
                    // Push the previous chunk if it has content
                    chunks.push(chunk);
                }

                chunk = Some(Chunk::with_type(
                    ChunkType::from_str(HIGHLIGHT_NAMES[s.0]).expect("Invalid highlighting type"),
                ));
            }
            HighlightEvent::HighlightEnd => {
                if let Some(chunk) = chunk.take() {
                    chunks.push(chunk);
                }
                chunk = None;
            }
        }
    }

    if let Some(chunk) = chunk.take() {
        chunks.push(chunk);
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

    #[test]
    fn test_split_chunk() {
        let chunk = Chunk {
            contents: "Hello, world!\nThis is a test",
            start: 0,
            end: 0,
            typ: ChunkType::None,
        };

        let chunks = chunk.split('\n');
        assert_eq!(chunks.len(), 2);
        assert_eq!(chunks[0].contents, "Hello, world!");
        assert_eq!(chunks[1].contents, "This is a test");
    }

    #[test]
    fn test_split_chunks() {
        let chunks = vec![
            Chunk {
                contents: "function main() {\n    ",
                start: 0,
                end: 0,
                typ: ChunkType::None,
            },
            Chunk {
                contents: "println!(\"Hello, world!\");\n}",
                start: 0,
                end: 0,
                typ: ChunkType::None,
            },
        ];

        let lines = split_chunks(chunks);
        println!("lines: {:?}", lines);
        let line = lines[0]
            .iter()
            .map(|c| c.contents)
            .collect::<Vec<&str>>()
            .join("");
        assert_eq!(line, "function main() {");

        let line = lines[1]
            .iter()
            .map(|c| c.contents)
            .collect::<Vec<&str>>()
            .join("");
        assert_eq!(line, "    println!(\"Hello, world!\");");

        let line = lines[2]
            .iter()
            .map(|c| c.contents)
            .collect::<Vec<&str>>()
            .join("");
        assert_eq!(line, "}");
    }
}
