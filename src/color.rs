use super::error::PaletteError;

#[derive(Debug, Clone, Copy)]
pub struct Color {
    pub red: u8,
    pub green: u8,
    pub blue: u8,
}

fn lerp(a: u8, b: u8, t: f32) -> u8 {
    (a as f32 * (1. - t) + b as f32 * t) as u8
}

impl Color {
    pub fn rgb(red: u8, green: u8, blue: u8) -> Self {
        Color { red, green, blue }
    }

    pub fn lerp(start: Self, end: Self, t: f32) -> Self {
        Color {
            red: lerp(start.red, end.red, t),
            green: lerp(start.green, end.green, t),
            blue: lerp(start.blue, end.blue, t)
        }
    }
}

#[derive(Clone)]
pub struct Palette {
    keys: Vec<f32>,
    colors: Vec<Color>
}

impl Palette {
    pub fn new<I>(
        colors: impl IntoIterator<Item=Color>,
        keys: Option<I>
    ) -> Result<Palette, PaletteError>
    where I: IntoIterator<Item=f32>, I::IntoIter: Clone {
        let colors_: Vec<Color> = colors.into_iter().collect();

        let keys_: Vec<_> = match keys {
            None => {
                let l = colors_.len() - 1;
                (1..l).map(|i| i as f32 / l as f32).collect()
            }

            Some(keys__) => {
                let keys__ = keys__.into_iter();
                if keys__.clone().any(|k| k <= 0.0 || k >= 1.0) {
                    return Err(PaletteError::OutOfBounds)
                }
                if keys__.clone()
                    .zip(keys__.clone().skip(1))
                    .any(|(k, kn)| k >= kn)
                {
                    return Err(PaletteError::NonMonotonic)
                };
                keys__.collect()
            }
        };

        if keys_.len() + 2 != colors_.len() {
            return Err(PaletteError::IncorrectNumber)
        }

        Ok(Palette { keys: keys_, colors: colors_ })
    }

    pub fn sample(&self, c: f32) -> Option<Color> {
        if c < 0.0 || c > 1.0 { return None };

        let i = match self.keys.iter().rposition(|&k| k < c) {
            None => 0,
            Some(j) => j + 1
        };
        // println!("{}", i);
        let kbefore = self.keys.get(i - 1).unwrap_or(&0.0);
        let kafter = self.keys.get(i).unwrap_or(&1.0);
        let t = (c - kbefore) / (kafter - kbefore);

        Some(Color::lerp(self.colors[i], self.colors[i+1], t))
    }
}
