use directories::ProjectDirs;
use ini::Ini;
use std::fmt;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::str::FromStr;

/// The protocol a connection speaks. Not persisted; it only distinguishes behavior that
/// differs between protocols, such as whether layouts can be switched while connected.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ProtocolType {
    Via,
    Vial,
    Zmk,
}

#[derive(Debug)]
pub struct ParseSettingsError;

// Persisted by name, not index — indices shift when displays change, names don't.
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum MonitorSelection {
    Primary,
    Named(String),
}

impl fmt::Display for MonitorSelection {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            MonitorSelection::Primary => write!(f, "Primary"),
            MonitorSelection::Named(name) => write!(f, "{name}"),
        }
    }
}

impl FromStr for MonitorSelection {
    type Err = ParseSettingsError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        Ok(if value == "Primary" {
            MonitorSelection::Primary
        } else {
            MonitorSelection::Named(value.to_string())
        })
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

/// How a key with more than one legend (a native Shift pair, or an OS-resolved
/// RAlt result) is displayed. An enum, not two bools: only three of the four
/// combinations are meaningful.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum LegendMode {
    /// Base+Shifted stacked, as always (default).
    Stacked,
    /// Only what a plain tap produces.
    Single,
    /// Single, plus live-swap to the Shift/RAlt result while that modifier
    /// is physically held anywhere on the keyboard.
    SingleLive,
}

impl fmt::Display for LegendMode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}",
            match self {
                LegendMode::Stacked => "Stacked",
                LegendMode::Single => "Single",
                LegendMode::SingleLive => "Single + live preview",
            }
        )
    }
}

impl FromStr for LegendMode {
    type Err = ParseSettingsError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "Stacked" => Ok(LegendMode::Stacked),
            "Single" => Ok(LegendMode::Single),
            "Single + live preview" => Ok(LegendMode::SingleLive),
            _ => Err(ParseSettingsError),
        }
    }
}

/// Bitmask of the layers the overlay is shown for; bit `i` corresponds to layer `i`.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct LayerMask(u32);

impl LayerMask {
    pub const ALL: Self = Self(u32::MAX);

    pub fn bits(self) -> u32 {
        self.0
    }

    /// Whether any layer of the `layers` bitmask is shown.
    pub fn contains_any(self, layers: u32) -> bool {
        self.0 & layers != 0
    }

    pub fn set(&mut self, layers: u32, shown: bool) {
        if shown {
            self.0 |= layers;
        } else {
            self.0 &= !layers;
        }
    }
}

impl fmt::Display for LayerMask {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:#010x}", self.0)
    }
}

impl FromStr for LayerMask {
    type Err = ParseSettingsError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        let value = value.trim();
        u32::from_str_radix(value.strip_prefix("0x").unwrap_or(value), 16)
            .map(Self)
            .map_err(|_| ParseSettingsError)
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct ThemeColor {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

impl ThemeColor {
    pub const fn new(r: u8, g: u8, b: u8, a: u8) -> Self {
        Self { r, g, b, a }
    }
}

impl fmt::Display for ThemeColor {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{},{},{},{}", self.r, self.g, self.b, self.a)
    }
}

impl FromStr for ThemeColor {
    type Err = ParseSettingsError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        let mut parts = value.split(',').map(str::trim);
        let (Some(r), Some(g), Some(b), Some(a)) =
            (parts.next(), parts.next(), parts.next(), parts.next())
        else {
            return Err(ParseSettingsError);
        };

        if parts.next().is_some() {
            return Err(ParseSettingsError);
        }

        Ok(Self {
            r: r.parse().map_err(|_| ParseSettingsError)?,
            g: g.parse().map_err(|_| ParseSettingsError)?,
            b: b.parse().map_err(|_| ParseSettingsError)?,
            a: a.parse().map_err(|_| ParseSettingsError)?,
        })
    }
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct ThemeSettings {
    pub layer_colors: [ThemeColor; 7],
    pub font_color: ThemeColor,
}

impl ThemeSettings {
    /// Layers from here up share `layer_colors`' last entry and a single visibility bit.
    pub const OTHER_LAYERS: u8 = 6;

    pub fn layer_color(&self, layer: u8) -> ThemeColor {
        *self
            .layer_colors
            .get(layer as usize)
            .unwrap_or(&self.layer_colors[Self::OTHER_LAYERS as usize])
    }
}

impl Default for ThemeSettings {
    fn default() -> Self {
        const ALPHA: u8 = 239;
        Self {
            layer_colors: [
                ThemeColor::new(83, 83, 83, ALPHA),
                ThemeColor::new(80, 140, 115, ALPHA),
                ThemeColor::new(100, 115, 150, ALPHA),
                ThemeColor::new(140, 110, 150, ALPHA),
                ThemeColor::new(95, 121, 127, ALPHA),
                ThemeColor::new(147, 137, 110, ALPHA),
                ThemeColor::new(127, 127, 127, ALPHA),
            ],
            font_color: ThemeColor::new(255, 255, 255, 255),
        }
    }
}

#[derive(Clone, PartialEq)]
pub struct Settings {
    pub size: i32,
    pub font_size_multiplier: f32,
    pub auto_fit_before_ellipsis: bool,
    pub position: WindowPosition,
    pub timeout: i64,
    pub activation_delay: u32,
    pub margin: u32,
    pub visible_layers: LayerMask,
    pub theme: ThemeSettings,
    pub legend_mode: LegendMode,
    pub monitor: MonitorSelection,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            size: 60,
            font_size_multiplier: 1.0,
            auto_fit_before_ellipsis: false,
            position: WindowPosition::BottomRight,
            timeout: 2000,
            activation_delay: 0,
            margin: 10,
            visible_layers: LayerMask::ALL,
            theme: ThemeSettings::default(),
            legend_mode: LegendMode::Stacked,
            monitor: MonitorSelection::Primary,
        }
    }
}

impl Settings {
    /// Upper bound for `activation_delay`, shared with the settings UI.
    pub const MAX_ACTIVATION_DELAY_MS: u32 = 3_000;

    pub fn config_file_path() -> Option<PathBuf> {
        Self::project_dirs().map(|dirs| dirs.config_dir().join("settings.ini"))
    }

    fn project_dirs() -> Option<ProjectDirs> {
        ProjectDirs::from("dev", "srwi", "KeyPeek")
    }

    pub fn save(&self) -> std::io::Result<()> {
        let path = Self::config_file_path().ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::NotFound,
                "could not determine the KeyPeek config directory",
            )
        })?;
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        self.save_to_file(&path)
    }

    pub fn load() -> Option<Self> {
        Self::config_file_path()
            .and_then(Self::load_from_file)
            .or_else(|| Self::load_from_file("settings.ini"))
    }

    pub fn save_to_file(&self, path: impl AsRef<Path>) -> std::io::Result<()> {
        let mut conf = Ini::new();
        let mut section = conf.with_section(Some("settings"));
        section.set("size", self.size.to_string());
        section.set(
            "font_size_multiplier",
            self.font_size_multiplier.to_string(),
        );
        section.set(
            "auto_fit_before_ellipsis",
            self.auto_fit_before_ellipsis.to_string(),
        );
        section.set("position", self.position.to_string());
        section.set("timeout", self.timeout.to_string());
        section.set("activation_delay", self.activation_delay.to_string());
        section.set("margin", self.margin.to_string());
        section.set("visible_layers", self.visible_layers.to_string());
        for (index, color) in self.theme.layer_colors.iter().enumerate() {
            section.set(format!("layer_color_{index}"), color.to_string());
        }
        section.set("font_color", self.theme.font_color.to_string());
        section.set("legend_mode", self.legend_mode.to_string());
        section.set("monitor", self.monitor.to_string());
        conf.write_to_file(path)
    }

    pub fn load_from_file(path: impl AsRef<Path>) -> Option<Self> {
        let conf = Ini::load_from_file(path).ok()?;
        let section = conf.section(Some("settings"))?;
        let mut s = Settings::default();
        if let Some(val) = section.get("size") {
            s.size = val.parse().unwrap_or(s.size);
        }
        if let Some(val) = section.get("font_size_multiplier") {
            let parsed = val.parse::<f32>().unwrap_or(s.font_size_multiplier);
            s.font_size_multiplier = parsed.clamp(0.1, 2.0);
        }
        if let Some(val) = section.get("auto_fit_before_ellipsis") {
            s.auto_fit_before_ellipsis = val.parse().unwrap_or(s.auto_fit_before_ellipsis);
        }
        if let Some(val) = section.get("position") {
            if let Ok(parsed) = val.parse() {
                s.position = parsed;
            }
        }
        if let Some(val) = section.get("timeout") {
            let parsed = val.parse::<i64>().unwrap_or(s.timeout);
            s.timeout = if parsed < 0 {
                -1
            } else {
                parsed.clamp(0, 14_999)
            };
        }
        if let Some(val) = section.get("activation_delay") {
            let parsed = val.parse::<u32>().unwrap_or(s.activation_delay);
            s.activation_delay = parsed.min(Self::MAX_ACTIVATION_DELAY_MS);
        }
        if let Some(val) = section.get("margin") {
            s.margin = val.parse().unwrap_or(s.margin);
        }
        if let Some(val) = section.get("visible_layers") {
            s.visible_layers = val.parse().unwrap_or(s.visible_layers);
        }
        for index in 0..s.theme.layer_colors.len() {
            if let Some(val) = section.get(format!("layer_color_{index}")) {
                if let Ok(parsed) = val.parse() {
                    s.theme.layer_colors[index] = parsed;
                }
            }
        }
        if let Some(val) = section.get("font_color") {
            if let Ok(parsed) = val.parse() {
                s.theme.font_color = parsed;
            }
        }
        if let Some(val) = section.get("legend_mode") {
            if let Ok(parsed) = val.parse() {
                s.legend_mode = parsed;
            }
        }
        if let Some(val) = section.get("monitor") {
            if let Ok(parsed) = val.parse() {
                s.monitor = parsed;
            }
        }
        Some(s)
    }
}
