use std::{collections::HashMap, path::Path};

use crossterm::style;
use plist::{Dictionary, Value};

use crate::{error::ThemeParseError, utils::hex_to_crossterm_color};

#[derive(Debug, Clone, Default)]
pub struct Theme {
    pub name: String,
    pub author: Option<String>,
    pub background: String,
    pub foreground: String,
    pub caret: Option<String>,
    pub invisibles: Option<String>,
    pub settings: Vec<ThemeSetting>,
    pub gutter_foreground: Option<String>,
    pub gutter_background: Option<String>,
    pub gutter_foreground_highlight: Option<String>,
    pub gutter_background_highlight: Option<String>,
    pub line_highlight: Option<String>,
    pub selection: Option<String>,
}

#[derive(Debug, Clone)]
pub struct ThemeSetting {
    pub scopes: Vec<String>,
    pub settings: SettingAttributes,
}

#[derive(Debug, Clone, Default)]
pub struct SettingAttributes {
    pub background: Option<String>,
    pub foreground: Option<String>,
    pub font_style: Option<FontStyle>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FontStyle {
    Normal,
    Bold,
    Italic,
    BoldItalic,
    Underline,
}

impl Theme {
    pub fn default_colors(&self) -> (style::Color, style::Color) {
        (
            hex_to_crossterm_color(&self.foreground).unwrap(),
            hex_to_crossterm_color(&self.background).unwrap(),
        )
    }

    pub fn get_scope(&self, scope: &str) -> Option<&ThemeSetting> {
        let scope = scope.to_string();
        self.settings.iter().find(|s| s.scopes.contains(&scope))
    }

    pub fn scope_color(&self, scope: &str) -> (style::Color, style::Color) {
        let Some(setting) = self.get_scope(scope) else {
            return self.default_colors();
        };

        let background = setting
            .settings
            .background
            .as_ref()
            .map(|s| hex_to_crossterm_color(s).unwrap())
            .unwrap_or_else(|| hex_to_crossterm_color(&self.background).unwrap());
        let foreground = setting
            .settings
            .foreground
            .as_ref()
            .map(|s| hex_to_crossterm_color(s).unwrap())
            .unwrap_or_else(|| hex_to_crossterm_color(&self.foreground).unwrap());

        (background, foreground)
    }

    pub fn parse_vscode<P: AsRef<Path>>(path: P) -> anyhow::Result<Self> {
        let mappings = [
            ("attribute", "entity.other.attribute-name"),
            ("boolean", "constant.language"),
            //("carriage-return", "None"), // No direct mapping
            ("comment", "comment"),
            ("comment.documentation", "comment.block.documentation"),
            ("constant", "variable.other.constant"),
            ("constant.builtin", "*.defaultLibrary"),
            ("constructor", "variable.function.constructor"),
            //("constructor.builtin", "None"), // No direct mapping
            ("embedded", "punctuation.section.embedded"),
            ("error", "invalid"),
            ("escape", "constant.character.escape"),
            ("function", "entity.name.function"),
            ("function.builtin", "support.function"),
            ("keyword", "keyword"),
            ("markup", "markup"),
            ("markup.bold", "markup.bold"),
            ("markup.heading", "heading.1.markdown"),
            ("markup.italic", "markup.italic"),
            ("markup.link", "markup.underline.link"),
            //("markup.link.url", "None"), // No direct mapping
            //("markup.list", "None"), // No direct mapping
            //("markup.list.checked", "None"), // No direct mapping
            //("markup.list.numbered", "None"), // No direct mapping
            //("markup.list.unchecked", "None"), // No direct mapping
            //("markup.list.unnumbered", "None"), // No direct mapping
            ("markup.quote", "markup.quote"),
            ("markup.raw", "markup.inline.raw"), // Closest match
            //("markup.raw.block", "None"), // No direct mapping
            //("markup.raw.inline", "None"), // No direct mapping
            //("markup.strikethrough", "None"), // No direct mapping
            ("module", "entity.name.tag"), // Interpretive mapping
            ("number", "constant.numeric"),
            ("operator", "keyword.operator"),
            ("property", "variable.other.property"),
            ("property.builtin", "property.defaultLibrary"),
            ("punctuation", "punctuation"),
            //("punctuation.bracket", "None"), // No direct mapping
            //("punctuation.delimiter", "None"), // No direct mapping
            //("punctuation.special", "None"), // No direct mapping
            ("string", "string"),
            ("string.escape", "constant.character.escape"),
            ("string.regexp", "string.regexp"),
            //("string.special", "None"), // No direct mapping
            ("string.special.symbol", "constant.other.symbol"),
            ("tag", "entity.name.tag"),
            ("type", "support.type"),
            ("type.builtin", "support.type.sys-types"),
            ("variable", "variable"),
            ("variable.builtin", "variable.defaultLibrary"),
            ("variable.member", "variable.other.member"),
            ("variable.parameter", "variable.parameter"),
        ];

        let contents = std::fs::read_to_string(&path)?;
        let theme = serde_jsonrc::from_str::<serde_jsonrc::Value>(&contents)?;
        let Some(theme) = theme.as_object() else {
            // TODO: use a invalid field error instead
            return Err(ThemeParseError::MissingField("theme".to_string()).into());
        };

        let mut scopes = HashMap::new();

        // parses colors
        if let Some(colors) = theme.get("colors").and_then(|v| v.as_object()) {
            for (key, value) in colors.iter() {
                let Some(value) = value.as_object() else {
                    continue;
                };
                scopes.insert(key.clone(), value);
            }
        }

        // parses semanticTokenColors
        if let Some(semantic_token_colors) =
            theme.get("semanticTokenColors").and_then(|v| v.as_object())
        {
            for (key, value) in semantic_token_colors.iter() {
                let Some(value) = value.as_object() else {
                    continue;
                };
                scopes.insert(key.clone(), value);
            }
        }

        let Some(token_colors) = theme.get("tokenColors").and_then(|v| v.as_array()) else {
            return Err(ThemeParseError::MissingField("tokenColors".to_string()).into());
        };
        token_colors
            .iter()
            .filter_map(|color| {
                color.as_object().and_then(|info| {
                    info["settings"].as_object().and_then(|settings| {
                        info.get("scope").and_then(|v| {
                            v.as_array().map(|scope| {
                                scope
                                    .iter()
                                    .filter_map(|s| {
                                        s.as_str().map(|s| {
                                            s.split(',')
                                                .map(|s| s.trim().to_string())
                                                .collect::<Vec<_>>()
                                        })
                                    })
                                    .flatten()
                                    .for_each(|scope| {
                                        scopes.insert(scope, settings);
                                    })
                            })
                        })
                    })
                })
            })
            .for_each(drop);

        println!("{:#?}", scopes);

        let mut settings = Vec::new();
        for (from, to) in mappings.iter() {
            let from = from.to_string();
            let to = to.to_string();

            if let Some(from) = scopes.get(&from) {
                let background = from
                    .get("background")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string());
                let foreground = from
                    .get("foreground")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string());
                let font_style =
                    from.get("fontStyle")
                        .and_then(|v| v.as_str())
                        .and_then(|s| match s {
                            "bold" => Some(FontStyle::Bold),
                            "italic" => Some(FontStyle::Italic),
                            "bold italic" => Some(FontStyle::BoldItalic),
                            "underline" => Some(FontStyle::Underline),
                            _ => None,
                        });
                settings.push(ThemeSetting {
                    scopes: vec![to.clone()],
                    settings: SettingAttributes {
                        background,
                        foreground,
                        font_style,
                    },
                });
                scopes.insert(to, from);
            }
        }

        let name = theme["name"]
            .as_str()
            .unwrap_or(
                path.as_ref()
                    .file_name()
                    .unwrap_or_default()
                    .to_str()
                    .unwrap_or("unknown"),
            )
            .to_string();
        let author = theme
            .get("author")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());
        let background = theme["colors"]["editor.background"]
            .as_str()
            .unwrap_or("#000000")
            .to_string();
        let foreground = theme["colors"]["editor.foreground"]
            .as_str()
            .unwrap_or("#ffffff")
            .to_string();
        let invisibles = theme["colors"]["editorInvisibles.foreground"]
            .as_str()
            .map(|s| s.to_string());
        let line_highlight = theme["colors"]["editorLineNumber.activeForeground"]
            .as_str()
            .map(|s| s.to_string());
        // let selection = theme["colors"]["editorLineNumber.foreground"]
        //     .as_str()
        //     .map(|s| s.to_string());
        let gutter_foreground = theme["colors"]["editorGutter.foreground"]
            .as_str()
            .map(|s| s.to_string());
        let gutter_background = theme["colors"]["editorGutter.background"]
            .as_str()
            .map(|s| s.to_string());
        Ok(Self {
            name,
            author,
            background,
            foreground,
            invisibles,
            line_highlight,
            gutter_foreground,
            gutter_background,
            settings,
            ..Default::default()
        })
    }

    pub fn parse<P: AsRef<Path>>(path: P) -> anyhow::Result<Self> {
        let file_name = path.as_ref().to_str().unwrap().to_string();
        let data = plist::Value::from_file(path)?;
        let data = data.as_dictionary().unwrap();

        let name = data
            .get("name")
            .unwrap_or(&Value::String(file_name))
            .as_string()
            .unwrap()
            .to_string();
        let author = data
            .get("author")
            .and_then(|v| v.as_string())
            .map(|s| s.to_string());

        let Some(settings) = data.get("settings").and_then(|s| s.as_array()) else {
            return Err(ThemeParseError::MissingDictionaryField(
                data.clone(),
                "settings".to_string(),
            )
            .into());
        };

        let (main, settings): (Vec<_>, Vec<_>) = settings
            .iter()
            .partition(|s| s.as_dictionary().and_then(|d| d.get("name")).is_none());

        let Some(main) = main
            .first()
            .and_then(|s| s.as_dictionary())
            .and_then(|s| s.get("settings"))
            .and_then(|s| s.as_dictionary())
        else {
            return Err(ThemeParseError::MissingField("main".to_string()).into());
        };

        fn get_mandatory_setting(d: &Dictionary, key: &str) -> anyhow::Result<String> {
            d.get(key)
                .and_then(|v| v.as_string())
                .and_then(|s| Some(s.to_string()))
                .ok_or(ThemeParseError::MissingDictionaryField(d.clone(), key.to_string()).into())
        }

        fn get_setting(d: &Dictionary, key: &str) -> Option<String> {
            d.get(key)
                .and_then(|v| v.as_string())
                .map(|s| s.to_string())
        }

        let background = get_mandatory_setting(&main, "background")?;
        let foreground = get_mandatory_setting(&main, "foreground")?;

        let caret = get_setting(&main, "caret");
        let invisibles = get_setting(&main, "invisibles");

        // gutter settings
        let gutter_foreground = get_setting(&main, "gutterForeground");
        let gutter_background = get_setting(&main, "gutterBackground");
        let gutter_foregound_highlight = get_setting(&main, "gutterForegroundHighlight");
        let gutter_background_highlight = get_setting(&main, "gutterBackgroundHighlight");
        let line_highlight = get_setting(&main, "lineHighlight");
        let selection = get_setting(&main, "selection");

        // TODO: add other optional settings

        let settings = settings
            .iter()
            .filter_map(|s| {
                let s = s.as_dictionary().unwrap();
                let Some(scope) = get_setting(&s, "scope") else {
                    return None;
                };

                let settings = s.get("settings").and_then(|v| v.as_dictionary()).unwrap();
                let background = settings
                    .get("background")
                    .and_then(|v| v.as_string())
                    .and_then(|s| Some(s.to_string()));
                let foreground = settings
                    .get("foreground")
                    .and_then(|v| v.as_string())
                    .and_then(|s| Some(s.to_string()));
                let font_style = settings
                    .get("fontStyle")
                    .and_then(|v| v.as_string())
                    .and_then(|s| match s {
                        "bold" => Some(FontStyle::Bold),
                        "italic" => Some(FontStyle::Italic),
                        "bold italic" => Some(FontStyle::BoldItalic),
                        "underline" => Some(FontStyle::Underline),
                        _ => None,
                    });

                let scopes = scope.split(",").map(|s| s.trim().to_string()).collect();

                Some(ThemeSetting {
                    scopes,
                    settings: SettingAttributes {
                        background,
                        foreground,
                        font_style,
                    },
                })
            })
            .collect();

        Ok(Theme {
            name,
            author,
            background,
            foreground,
            caret,
            invisibles,
            line_highlight,
            selection,
            gutter_foreground,
            gutter_background,
            gutter_foreground_highlight: gutter_foregound_highlight,
            gutter_background_highlight,
            settings,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse() {
        let theme = Theme::parse_vscode("src/fixtures/tokyo-night-color-theme.json").unwrap();
        println!("{:#?}", theme);
    }
}
