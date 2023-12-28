use std::path::Path;

use plist::{Dictionary, Value};
use tree_sitter_highlight::{HighlightConfiguration, HighlightEvent, Highlighter};

use crate::error::ThemeParseError;

#[derive(Debug, Clone, Default)]
pub struct Theme {
    pub name: String,
    pub author: Option<String>,
    pub background: String,
    pub caret: String,
    pub foreground: String,
    pub invisibles: String,
    pub line_highlight: String,
    pub selection: String,
    pub settings: Vec<ThemeSetting>,
}

#[derive(Debug, Clone)]
pub struct ThemeSetting {
    pub scope: String,
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
    pub fn get_scope(&self, scope: &str) -> Option<&ThemeSetting> {
        self.settings.iter().find(|s| s.scope == scope)
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
            return Err(ThemeParseError::MissingField("settings".to_string()).into());
        };

        let (main, settings): (Vec<_>, Vec<_>) = settings
            .iter()
            .partition(|s| s.as_dictionary().and_then(|d| d.get("name")).is_none());

        println!("settings: {:#?}", settings);

        let Some(main) = main
            .first()
            .and_then(|s| s.as_dictionary())
            .and_then(|s| s.get("settings"))
            .and_then(|s| s.as_dictionary())
        else {
            return Err(ThemeParseError::MissingField("main".to_string()).into());
        };

        fn get_setting(d: &Dictionary, key: &str) -> anyhow::Result<String> {
            d.get(key)
                .and_then(|v| v.as_string())
                .and_then(|s| Some(s.to_string()))
                .ok_or(ThemeParseError::MissingField(key.to_string()).into())
        }

        let background = get_setting(&main, "background")?;
        let caret = get_setting(&main, "caret")?;
        let foreground = get_setting(&main, "foreground")?;
        let invisibles = get_setting(&main, "invisibles")?;
        let line_highlight = get_setting(&main, "lineHighlight")?;
        let selection = get_setting(&main, "selection")?;

        let settings = settings
            .iter()
            .map(|s| {
                let s = s.as_dictionary().unwrap();
                let scope = get_setting(&s, "scope").unwrap();

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

                ThemeSetting {
                    scope,
                    settings: SettingAttributes {
                        background,
                        foreground,
                        font_style,
                    },
                }
            })
            .collect();

        Ok(Theme {
            name,
            author,
            background,
            caret,
            foreground,
            invisibles,
            line_highlight,
            selection,
            settings,
        })
    }
}
