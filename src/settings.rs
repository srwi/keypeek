use ini::Ini;
use std::fmt;
use std::str::FromStr;

#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum ProtocolType {
    #[default]
    Via,
    Vial,
}

impl fmt::Display for ProtocolType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}",
            match self {
                ProtocolType::Via => "via",
                ProtocolType::Vial => "vial",
            }
        )
    }
}

#[derive(Debug)]
pub struct ParseSettingsError;

impl FromStr for ProtocolType {
    type Err = ParseSettingsError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value.to_lowercase().as_str() {
            "via" => Ok(ProtocolType::Via),
            "vial" => Ok(ProtocolType::Vial),
            _ => Err(ParseSettingsError),
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum WindowPosition {
    TopLeft,
    TopRight,
    BottomLeft,
    BottomRight,
    Bottom,
    Top,
}

impl fmt::Display for WindowPosition {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}",
            match self {
                WindowPosition::TopLeft => "Top Left",
                WindowPosition::TopRight => "Top Right",
                WindowPosition::BottomLeft => "Bottom Left",
                WindowPosition::BottomRight => "Bottom Right",
                WindowPosition::Bottom => "Bottom",
                WindowPosition::Top => "Top",
            }
        )
    }
}

impl FromStr for WindowPosition {
    type Err = ParseSettingsError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "Top Left" => Ok(WindowPosition::TopLeft),
            "Top Right" => Ok(WindowPosition::TopRight),
            "Bottom Left" => Ok(WindowPosition::BottomLeft),
            "Bottom Right" => Ok(WindowPosition::BottomRight),
            "Bottom" => Ok(WindowPosition::Bottom),
            "Top" => Ok(WindowPosition::Top),
            _ => Err(ParseSettingsError),
        }
    }
}

#[derive(Clone)]
pub struct Settings {
    pub protocol_type: ProtocolType,
    pub device_identifier: String,
    pub layout_name: String,
    pub size: i32,
    pub position: WindowPosition,
    pub timeout: u64,
    pub margin: u32,
    pub confirmed: bool,
    pub save_settings: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            protocol_type: ProtocolType::default(),
            device_identifier: String::new(),
            layout_name: "LAYOUT".to_string(),
            size: 60,
            position: WindowPosition::BottomRight,
            timeout: 2000,
            margin: 10,
            confirmed: false,
            save_settings: false,
        }
    }
}

impl Settings {
    pub fn save_to_file(&self, path: &str) -> std::io::Result<()> {
        let mut conf = Ini::new();
        let mut section = conf.with_section(Some("settings"));
        section.set("protocol_type", self.protocol_type.to_string());
        section.set("device_identifier", &self.device_identifier);
        section.set("layout_name", &self.layout_name);
        section.set("size", self.size.to_string());
        section.set("position", self.position.to_string());
        section.set("timeout", self.timeout.to_string());
        section.set("margin", self.margin.to_string());
        conf.write_to_file(path)
    }

    pub fn load_from_file(path: &str) -> Option<Self> {
        let conf = Ini::load_from_file(path).ok()?;
        let section = conf.section(Some("settings"))?;
        let mut s = Settings::default();
        if let Some(val) = section.get("protocol_type") {
            if let Ok(parsed) = val.parse() {
                s.protocol_type = parsed;
            }
        }
        if let Some(val) = section.get("device_identifier") {
            s.device_identifier = val.to_string();
        }
        if let Some(val) = section.get("layout_name") {
            s.layout_name = val.to_string();
        }
        if let Some(val) = section.get("size") {
            s.size = val.parse().unwrap_or(s.size);
        }
        if let Some(val) = section.get("position") {
            if let Ok(parsed) = val.parse() {
                s.position = parsed;
            }
        }
        if let Some(val) = section.get("timeout") {
            s.timeout = val.parse().unwrap_or(s.timeout);
        }
        if let Some(val) = section.get("margin") {
            s.margin = val.parse().unwrap_or(s.margin);
        }
        s.confirmed = true;
        Some(s)
    }
}
