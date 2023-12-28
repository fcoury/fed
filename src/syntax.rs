use std::str::FromStr;

use strum_macros::EnumString;
use tree_sitter_highlight::{HighlightConfiguration, HighlightEvent, Highlighter};

use crate::theme::Theme;

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

#[derive(Debug, PartialEq, EnumString)]
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

pub fn highlight(buffer: &[String], theme: &Theme) {}

fn parse<'a>(
    source: &'a str,
    lang_config: &'a mut HighlightConfiguration,
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
