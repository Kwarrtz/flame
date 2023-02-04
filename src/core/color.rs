#[derive(Debug, Clone, Copy)]
pub struct Color {
    pub red: u8,
    pub green: u8,
    pub blue: u8,
}

impl Color {
    pub fn rgb(red: u8, green: u8, blue: u8) -> Self {
        Color { red, green, blue }
    }
}

#[derive(Clone)]
pub struct Palette {
    colors: [Color; 256]
}

impl Palette {
    pub fn new(colors: [Color; 256]) -> Palette {
        Palette { colors }
    }

    pub fn sample(&self, i: f32) -> Color {
        if i >= 0. && i < 1. {
            self.colors[(i * 256.) as usize]
        } else {
            panic!("Palette sample index must be between 0 and 1")
        }
    }
}
