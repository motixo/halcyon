//! Color-independent finishes. Separate namespace leaves version-1 layouts readable.
use serde::{Deserialize, Serialize};
pub const NAMESPACE: &str = "io.github.profilaz3.Halcyon.Materials";
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Kind {
    Frosted,
    Clear,
    Smoked,
    Liquid,
}
impl Kind {
    pub const ALL: [Self; 4] = [Self::Frosted, Self::Clear, Self::Smoked, Self::Liquid];
    pub fn title(self) -> &'static str {
        match self {
            Self::Frosted => "Frosted",
            Self::Clear => "Clear glass",
            Self::Smoked => "Smoked glass",
            Self::Liquid => "Liquid",
        }
    }
    pub fn key(self) -> &'static str {
        match self {
            Self::Frosted => "frosted",
            Self::Clear => "clear",
            Self::Smoked => "smoked",
            Self::Liquid => "liquid",
        }
    }
    pub fn opacity(self) -> u8 {
        match self {
            Self::Frosted => 82,
            Self::Clear => 62,
            Self::Smoked => 94,
            Self::Liquid => 76,
        }
    }
    pub fn highlight(self) -> f32 {
        match self {
            Self::Frosted => 0.035,
            Self::Clear => 0.06,
            Self::Smoked => 0.025,
            Self::Liquid => 0.13,
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Finish {
    pub kind: Kind,
    pub opacity: Option<u8>,
    pub tint: u8,
}
impl Finish {
    pub fn new(kind: Kind) -> Self {
        Self {
            kind,
            opacity: None,
            tint: 35,
        }
    }
    pub fn opacity(self) -> u8 {
        self.opacity.unwrap_or(self.kind.opacity()).clamp(45, 100)
    }
    pub fn validate(self) -> Result<(), String> {
        if self.opacity.is_some_and(|v| !(45..=100).contains(&v)) || self.tint > 100 {
            Err("Material opacity must be 45–100 and tint 0–100".into())
        } else {
            Ok(())
        }
    }
    /// Native/app palette remains the color owner; only its accent's surface contribution changes.
    pub fn color(self, mut color: [f32; 4], accent: [f32; 3]) -> [f32; 4] {
        let amount = self.tint.min(100) as f32 / 100. * 0.16;
        for (c, accent) in color[..3].iter_mut().zip(accent) {
            *c = (*c * (1. - amount) + accent * amount).clamp(0., 1.);
        }
        color[3] = self.opacity() as f32 / 100.;
        color
    }
}
/// Versioned independent per-bar record; unknown versions are preserved, ignored by renderer.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BarMaterial {
    pub version: u32,
    pub finish: Finish,
}
impl BarMaterial {
    pub fn validate(self) -> Result<(), String> {
        if self.version != 1 {
            return Err("Unsupported material version".into());
        }
        self.finish.validate()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn four_finishes_and_tint_work_even_on_a_neutral_native_theme() {
        let base = [0.13, 0.13, 0.13, 1.];
        let accent = [0.2, 0.7, 0.8];
        let mut alphas = Vec::new();
        for kind in Kind::ALL {
            let mut f = Finish::new(kind);
            assert!(f.validate().is_ok());
            let tinted = f.color(base, accent);
            f.tint = 0;
            let neutral = f.color(base, accent);
            assert_ne!(neutral, tinted);
            assert_eq!(&neutral[..3], &base[..3]);
            assert!(!alphas.contains(&neutral[3]));
            alphas.push(neutral[3]);
            f.opacity = Some(45);
            assert_eq!(f.color(base, accent)[3], 0.45);
        }
    }
    #[test]
    fn material_schema_rejects_unknown_versions_and_out_of_range_values() {
        let mut record = BarMaterial {
            version: 1,
            finish: Finish::new(Kind::Liquid),
        };
        record.version = 2;
        assert!(record.validate().is_err());
        record.version = 1;
        record.finish.tint = 101;
        assert!(record.validate().is_err());
        record.finish.tint = 35;
        record.finish.opacity = Some(44);
        assert!(record.validate().is_err());
    }
}
