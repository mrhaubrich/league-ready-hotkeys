use thiserror::Error;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShortcutKey {
    F1,
    F2,
    F3,
    F4,
    F5,
    F6,
    F7,
    F8,
    F9,
    F10,
    F11,
    F12,
}

impl ShortcutKey {
    pub const fn virtual_key(self) -> u32 {
        match self {
            Self::F1 => 0x70,
            Self::F2 => 0x71,
            Self::F3 => 0x72,
            Self::F4 => 0x73,
            Self::F5 => 0x74,
            Self::F6 => 0x75,
            Self::F7 => 0x76,
            Self::F8 => 0x77,
            Self::F9 => 0x78,
            Self::F10 => 0x79,
            Self::F11 => 0x7A,
            Self::F12 => 0x7B,
        }
    }
    pub fn parse(value: &str) -> Result<Self, ShortcutError> {
        match value.trim().to_ascii_uppercase().as_str() {
            "F1" => Ok(Self::F1),
            "F2" => Ok(Self::F2),
            "F3" => Ok(Self::F3),
            "F4" => Ok(Self::F4),
            "F5" => Ok(Self::F5),
            "F6" => Ok(Self::F6),
            "F7" => Ok(Self::F7),
            "F8" => Ok(Self::F8),
            "F9" => Ok(Self::F9),
            "F10" => Ok(Self::F10),
            "F11" => Ok(Self::F11),
            "F12" => Ok(Self::F12),
            _ => Err(ShortcutError::Unsupported(value.to_owned())),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ShortcutConfig {
    pub accept: ShortcutKey,
    pub decline: ShortcutKey,
}

impl Default for ShortcutConfig {
    fn default() -> Self {
        Self {
            accept: ShortcutKey::F1,
            decline: ShortcutKey::F2,
        }
    }
}

impl ShortcutConfig {
    pub fn parse(accept: &str, decline: &str) -> Result<Self, ShortcutError> {
        let config = Self {
            accept: ShortcutKey::parse(accept)?,
            decline: ShortcutKey::parse(decline)?,
        };
        if config.accept == config.decline {
            return Err(ShortcutError::Duplicate);
        }
        Ok(config)
    }
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum ShortcutError {
    #[error("unsupported shortcut: {0}")]
    Unsupported(String),
    #[error("accept and decline shortcuts must differ")]
    Duplicate,
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn defaults_are_f1_f2() {
        assert_eq!(
            ShortcutConfig::default(),
            ShortcutConfig::parse("F1", "F2").unwrap()
        );
    }
    #[test]
    fn rejects_duplicates() {
        assert_eq!(
            ShortcutConfig::parse("F3", "f3"),
            Err(ShortcutError::Duplicate)
        );
    }
    #[test]
    fn rejects_unknown_keys() {
        assert!(matches!(
            ShortcutKey::parse("A"),
            Err(ShortcutError::Unsupported(_))
        ));
    }
}
