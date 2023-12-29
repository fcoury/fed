use std::num::ParseIntError;

use crossterm::style::{self, Color};
use palette::{rgb::Rgb, Darken, Lighten};

pub fn hex_to_crossterm_color(hex: &str) -> Result<style::Color, ParseIntError> {
    let hex = hex.trim_start_matches('#');

    let r = u8::from_str_radix(&hex[0..2], 16)?;
    let g = u8::from_str_radix(&hex[2..4], 16)?;
    let b = u8::from_str_radix(&hex[4..6], 16)?;

    Ok(style::Color::Rgb { r, g, b })
}

pub fn adjust_brightness(color: Color, factor: f32) -> anyhow::Result<style::Color> {
    assert!(factor >= -1.0 && factor <= 1.0 && factor != 0.0);

    let Color::Rgb { r, g, b } = color else {
        return Err(anyhow::anyhow!("Unable to fade non-rgb colors"));
    };

    let rgb: Rgb = Rgb::new(r as f32 / 255.0, g as f32 / 255.0, b as f32 / 255.0);
    let color = if factor > 0.0 {
        rgb.lighten(factor)
    } else {
        rgb.darken(factor.abs())
    };

    let (r, g, b) = color.into_components();
    let r = (r * 255.0) as u8;
    let g = (g * 255.0) as u8;
    let b = (b * 255.0) as u8;

    Ok(style::Color::Rgb { r, g, b })
}

pub fn brigthen(color: Color, factor: f32) -> anyhow::Result<style::Color> {
    adjust_brightness(color, factor)
}

pub fn darken(color: Color, factor: f32) -> anyhow::Result<style::Color> {
    adjust_brightness(color, -factor)
}

pub fn hex_to_rgb(hex: &str) -> Result<[u8; 3], ParseIntError> {
    let hex = hex.trim_start_matches('#');

    let r = u8::from_str_radix(&hex[0..2], 16)?;
    let g = u8::from_str_radix(&hex[2..4], 16)?;
    let b = u8::from_str_radix(&hex[4..6], 16)?;

    Ok([r, g, b])
}
