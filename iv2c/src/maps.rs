pub enum CharMap {
    Chars1,
    Chars2,
    Chars3,
    Solid,
    Dotted,
    Gradient,
    BlackWhite,
    BwDotted,
    Braille,
    Custom(Vec<char>),
}

impl CharMap {
    pub fn chars(self) -> Vec<char> {
        match self {
            CharMap::Chars1 => CHARS1.chars().collect(),
            CharMap::Chars2 => CHARS2.chars().collect(),
            CharMap::Chars3 => CHARS3.chars().collect(),
            CharMap::Solid => SOLID.chars().collect(),
            CharMap::Dotted => DOTTED.chars().collect(),
            CharMap::Gradient => GRADIENT.chars().collect(),
            CharMap::BlackWhite => BLACKWHITE.chars().collect(),
            CharMap::BwDotted => BW_DOTTED.chars().collect(),
            CharMap::Braille => BRAILLE.chars().collect(),
            CharMap::Custom(chars) => chars,
        }
    }

    pub fn custom(chars: &str) -> Self {
        Self::Custom(chars.chars().collect())
    }
}

// maps from https://github.com/maxcurzi/tplay/blob/main/src/pipeline/char_maps.rs

// ASCII-127 Only
const CHARS1: &str = r##" .:-=+*#%@"##; // 10 chars
const CHARS2: &str = r##" .'`^",:;Il!i~+_-?][}{1)(|/tfjrxnuvczXYUJCLQ0OZmwqpdbkhao*#MW&8%B@$"##; // 67 chars
const CHARS3: &str = r##" `.-':_,^=;><+!rc*/z?sLTv)J7(|Fi{C}fI31tlu[neoZ5Yxjya]2ESwqkP6h9d4VpOGbUAKXHm8RD#$Bg0MNWQ%&@"##; // 92 chars

// ASCII-255
const SOLID: &str = r#"█"#; // 1 Solid block
const DOTTED: &str = r#"⣿"#; // 1 dotted block
const GRADIENT: &str = r#" ░▒▓█"#; // 5 chars
const BLACKWHITE: &str = r#" █"#; // 2 chars
const BW_DOTTED: &str = r#" ⣿"#; // 2 dotted block
const BRAILLE: &str = r#" ··⣀⣀⣤⣤⣤⣀⡀⢀⠠⠔⠒⠑⠊⠉⠁"#; // 16 chars (braille-based)
