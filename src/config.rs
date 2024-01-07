use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize)]
pub struct ConfigFile {
    pub faded_line_numbers: Option<bool>,
    pub tab_size: Option<u8>,
    pub tab_to_spaces: Option<bool>,
    pub mouse_scroll_lines: Option<u8>,
    pub theme: Option<String>,
}

impl From<ConfigFile> for Config {
    fn from(config: ConfigFile) -> Self {
        Self {
            faded_line_numbers: config.faded_line_numbers.unwrap_or(true),
            tab_size: config.tab_size.unwrap_or(4),
            tab_to_spaces: config.tab_to_spaces.unwrap_or(true),
            mouse_scroll_lines: config.mouse_scroll_lines.unwrap_or(3),
            theme: config.theme,
        }
    }
}

#[derive(Debug)]
pub struct Config {
    pub faded_line_numbers: bool,
    pub tab_size: u8,
    pub tab_to_spaces: bool,
    pub mouse_scroll_lines: u8,
    pub theme: Option<String>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            faded_line_numbers: true,
            tab_size: 4,
            tab_to_spaces: true,
            mouse_scroll_lines: 3,
            theme: None,
        }
    }
}

impl Config {
    pub fn read() -> anyhow::Result<Self> {
        let mut path = dirs::home_dir().ok_or_else(|| anyhow::anyhow!("No home directory"))?;
        path.push(".config");

        let config = path.join("fed.toml");
        let config = config.to_str().unwrap();
        Self::read_from_file(config)
    }

    pub fn read_from_file(file: &str) -> anyhow::Result<Self> {
        if !std::path::Path::new(file).exists() {
            return Ok(Self::default());
        }
        let config = std::fs::read_to_string(file)
            .map_err(|e| anyhow::anyhow!("error opening config file: {}", e))?;
        let config = toml::from_str::<ConfigFile>(&config)?;
        Ok(config.into())
    }
}
