use std::num::ParseIntError;

use crossterm::style;

pub fn hex_to_crossterm_color(hex: &str) -> Result<style::Color, ParseIntError> {
    let hex = hex.trim_start_matches('#');

    let r = u8::from_str_radix(&hex[0..2], 16)?;
    let g = u8::from_str_radix(&hex[2..4], 16)?;
    let b = u8::from_str_radix(&hex[4..6], 16)?;

    Ok(style::Color::Rgb { r, g, b })
}
