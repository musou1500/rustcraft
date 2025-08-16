use std::collections::HashMap;
use std::fs;
use std::path::Path;
use serde::Deserialize;
use toml;

/// TOML texture file structure
#[derive(Debug, Deserialize)]
struct TextureToml {
    texture: TextureInfo,
    palette: HashMap<String, String>,
    pixels: PixelData,
}

#[derive(Debug, Deserialize)]
struct TextureInfo {
    name: String,
    description: String,
    size: [u32; 2],
}

#[derive(Debug, Deserialize)]
struct PixelData {
    data: String,
}

/// Represents a parsed texture with RGBA pixel data
#[derive(Debug, Clone)]
pub struct ParsedTexture {
    pub name: String,
    pub width: u32,
    pub height: u32,
    pub pixels: Vec<u8>, // RGBA format
}

/// Color palette entry
#[derive(Debug, Clone)]
struct PaletteEntry {
    r: u8,
    g: u8,
    b: u8,
    a: u8,
}

/// Parses a single .toml texture file
pub fn parse_texture_file<P: AsRef<Path>>(path: P) -> Result<ParsedTexture, String> {
    let content = fs::read_to_string(path).map_err(|e| format!("Failed to read file: {}", e))?;
    
    // Parse TOML content
    let texture_toml: TextureToml = toml::from_str(&content)
        .map_err(|e| format!("Failed to parse TOML: {}", e))?;
    
    let width = texture_toml.texture.size[0];
    let height = texture_toml.texture.size[1];
    let name = texture_toml.texture.name;
    
    // Build palette from TOML
    let mut palette: HashMap<char, PaletteEntry> = HashMap::new();
    for (key_str, color_str) in texture_toml.palette {
        if let Some(key_char) = key_str.chars().next() {
            if color_str == "transparent" {
                palette.insert(key_char, PaletteEntry { r: 0, g: 0, b: 0, a: 0 });
            } else if color_str.starts_with('#') {
                let hex = &color_str[1..];
                match hex.len() {
                    6 => {
                        // #rrggbb format
                        if let (Ok(r), Ok(g), Ok(b)) = (
                            u8::from_str_radix(&hex[0..2], 16),
                            u8::from_str_radix(&hex[2..4], 16),
                            u8::from_str_radix(&hex[4..6], 16),
                        ) {
                            palette.insert(key_char, PaletteEntry { r, g, b, a: 255 });
                        }
                    }
                    8 => {
                        // #rrggbbaa format
                        if let (Ok(r), Ok(g), Ok(b), Ok(a)) = (
                            u8::from_str_radix(&hex[0..2], 16),
                            u8::from_str_radix(&hex[2..4], 16),
                            u8::from_str_radix(&hex[4..6], 16),
                            u8::from_str_radix(&hex[6..8], 16),
                        ) {
                            palette.insert(key_char, PaletteEntry { r, g, b, a });
                        }
                    }
                    _ => {
                        // Invalid hex color length, skip
                    }
                }
            }
        }
    }
    
    // Parse pixel data from multi-line string
    let pixel_lines: Vec<&str> = texture_toml.pixels.data
        .lines()
        .map(|line| line.trim())
        .filter(|line| !line.is_empty())
        .collect();
    
    // Validate dimensions
    if width == 0 || height == 0 {
        return Err("Invalid texture dimensions".to_string());
    }
    
    if pixel_lines.len() != height as usize {
        return Err(format!("Expected {} pixel rows, found {}", height, pixel_lines.len()));
    }
    
    // Convert pixel characters to RGBA data
    let mut pixels = Vec::with_capacity((width * height * 4) as usize);
    
    for (row_index, line) in pixel_lines.iter().enumerate() {
        let chars: Vec<char> = line.chars().collect();
        if chars.len() != width as usize {
            return Err(format!("Row {} has {} characters, expected {}", row_index, chars.len(), width));
        }
        
        for &ch in &chars {
            let color = palette.get(&ch).unwrap_or(&PaletteEntry { r: 255, g: 0, b: 255, a: 255 }); // Magenta for missing
            pixels.push(color.r);
            pixels.push(color.g);
            pixels.push(color.b);
            pixels.push(color.a);
        }
    }
    
    Ok(ParsedTexture {
        name,
        width,
        height,
        pixels,
    })
}

/// Load all texture files from the textures directory
pub fn load_all_textures() -> Result<HashMap<String, ParsedTexture>, String> {
    let mut textures = HashMap::new();
    
    let textures_dir = Path::new("textures");
    if !textures_dir.exists() {
        return Err("Textures directory not found".to_string());
    }
    
    let entries = fs::read_dir(textures_dir)
        .map_err(|e| format!("Failed to read textures directory: {}", e))?;
    
    for entry in entries {
        let entry = entry.map_err(|e| format!("Failed to read directory entry: {}", e))?;
        let path = entry.path();
        
        if let Some(extension) = path.extension() {
            if extension == "toml" {
                if let Some(file_stem) = path.file_stem() {
                    let texture_name = file_stem.to_string_lossy().to_string();
                    
                    match parse_texture_file(&path) {
                        Ok(texture) => {
                            textures.insert(texture_name.clone(), texture);
                            println!("Loaded texture: {}", texture_name);
                        }
                        Err(e) => {
                            eprintln!("Failed to parse texture {}: {}", texture_name, e);
                        }
                    }
                }
            }
        }
    }
    
    Ok(textures)
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_simple_texture_parsing() {
        let content = r#"
# Test texture
size: 2x2
palette:
  . = #FF0000  # Red
  # = #00FF00  # Green

pixels:
.#
#.
"#;
        
        // We'd need to create a temporary file for this test
        // For now, this demonstrates the expected functionality
    }
}