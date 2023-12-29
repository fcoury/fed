use std::path::Path;

use crossterm::style;
use plist::{Dictionary, Value};

use crate::{error::ThemeParseError, utils::hex_to_crossterm_color};

#[derive(Debug, Clone, Default)]
pub struct Theme {
    pub name: String,
    pub author: Option<String>,
    pub background: String,
    pub caret: String,
    pub foreground: String,
    pub invisibles: String,
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
            hex_to_crossterm_color(&self.background).unwrap(),
            hex_to_crossterm_color(&self.foreground).unwrap(),
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
        let caret = get_mandatory_setting(&main, "caret")?;
        let foreground = get_mandatory_setting(&main, "foreground")?;
        let invisibles = get_mandatory_setting(&main, "invisibles")?;

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
            caret,
            foreground,
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
