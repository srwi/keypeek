use directories::ProjectDirs;
use ini::Ini;
use std::fmt;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::str::FromStr;
use std::sync::RwLock;

#[derive(Debug)]
pub struct ParseSettingsError;

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

#[derive(Clone, PartialEq, Debug)]
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
        }
    }
}

impl Settings {
    /// Upper bound for `activation_delay`, shared with the settings UI.
    pub const MAX_ACTIVATION_DELAY_MS: u32 = 3_000;

    pub fn config_file_path() -> Option<PathBuf> {
        FileSettingsStore::default_config_path()
    }

    pub fn save(&self) -> std::io::Result<()> {
        FileSettingsStore::default()
            .save(self)
            .map_err(io::Error::other)
    }

    pub fn load() -> Option<Self> {
        FileSettingsStore::default().load_file()
    }

    pub fn to_ini(&self) -> Ini {
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
        conf
    }

    pub fn to_ini_string(&self) -> String {
        let conf = self.to_ini();
        let mut buf = Vec::new();
        let _ = conf.write_to(&mut buf);
        String::from_utf8(buf).unwrap_or_default()
    }

    pub fn from_ini(conf: &Ini) -> Option<Self> {
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
        Some(s)
    }

    pub fn from_ini_str(content: &str) -> Option<Self> {
        let conf = Ini::load_from_str(content).ok()?;
        Self::from_ini(&conf)
    }

    pub fn save_to_file(&self, path: impl AsRef<Path>) -> std::io::Result<()> {
        let conf = self.to_ini();
        conf.write_to_file(path)
    }

    pub fn load_from_file(path: impl AsRef<Path>) -> Option<Self> {
        let conf = Ini::load_from_file(path).ok()?;
        Self::from_ini(&conf)
    }
}

/// Persistent storage port for application settings.
pub trait SettingsStore: Send + Sync {
    fn load(&self) -> Settings;
    fn save(&self, settings: &Settings) -> Result<(), String>;
}

/// In-memory settings store, useful for tests, fallbacks, and environments without disk access.
#[derive(Debug)]
pub struct MemorySettingsStore {
    settings: RwLock<Settings>,
}

impl MemorySettingsStore {
    pub fn new(settings: Settings) -> Self {
        Self {
            settings: RwLock::new(settings),
        }
    }
}

impl Default for MemorySettingsStore {
    fn default() -> Self {
        Self::new(Settings::default())
    }
}

impl SettingsStore for MemorySettingsStore {
    fn load(&self) -> Settings {
        self.settings.read().map(|s| s.clone()).unwrap_or_default()
    }

    fn save(&self, settings: &Settings) -> Result<(), String> {
        match self.settings.write() {
            Ok(mut guard) => {
                *guard = settings.clone();
                Ok(())
            }
            Err(e) => Err(format!("MemorySettingsStore lock poisoned: {e}")),
        }
    }
}

/// File-backed settings store using INI format on the local filesystem.
#[derive(Clone, Debug, Default)]
pub struct FileSettingsStore {
    custom_path: Option<PathBuf>,
}

impl FileSettingsStore {
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self {
            custom_path: Some(path.into()),
        }
    }

    pub fn default_config_path() -> Option<PathBuf> {
        ProjectDirs::from("dev", "srwi", "KeyPeek")
            .map(|dirs| dirs.config_dir().join("settings.ini"))
    }

    pub fn resolve_path(&self) -> Option<PathBuf> {
        self.custom_path.clone().or_else(Self::default_config_path)
    }

    pub fn load_file(&self) -> Option<Settings> {
        if let Some(path) = self.resolve_path() {
            if let Some(settings) = Settings::load_from_file(&path) {
                return Some(settings);
            }
        }
        Settings::load_from_file("settings.ini")
    }
}

impl SettingsStore for FileSettingsStore {
    fn load(&self) -> Settings {
        self.load_file().unwrap_or_default()
    }

    fn save(&self, settings: &Settings) -> Result<(), String> {
        let path = self
            .resolve_path()
            .ok_or_else(|| "could not determine the KeyPeek config directory".to_string())?;

        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|e| {
                format!(
                    "failed to create config directory {}: {e}",
                    parent.display()
                )
            })?;
        }

        settings
            .save_to_file(&path)
            .map_err(|e| format!("failed to save settings to {}: {e}", path.display()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_settings_ini_round_trip() {
        let mut original = Settings::default();
        original.size = 75;
        original.font_size_multiplier = 1.3;
        original.auto_fit_before_ellipsis = true;
        original.position = WindowPosition::TopLeft;
        original.timeout = 3500;
        original.activation_delay = 200;
        original.margin = 25;
        original.visible_layers = LayerMask(0x0000000f);
        original.legend_mode = LegendMode::SingleLive;
        original.theme.font_color = ThemeColor::new(10, 20, 30, 40);
        original.theme.layer_colors[0] = ThemeColor::new(50, 60, 70, 80);

        let ini_text = original.to_ini_string();
        let loaded = Settings::from_ini_str(&ini_text).expect("should parse ini");

        assert_eq!(original, loaded);
    }

    #[test]
    fn test_memory_settings_store() {
        let store = MemorySettingsStore::default();
        let initial = store.load();
        assert_eq!(initial, Settings::default());

        let mut updated = initial.clone();
        updated.timeout = 5000;
        updated.position = WindowPosition::Bottom;

        store.save(&updated).expect("save should succeed");
        assert_eq!(store.load(), updated);
    }

    #[test]
    fn test_file_settings_store_custom_path() {
        let temp_dir = std::env::temp_dir().join(format!("keypeek_test_{}", std::process::id()));
        let file_path = temp_dir.join("test_settings.ini");

        let store = FileSettingsStore::new(&file_path);
        let mut settings = Settings::default();
        settings.size = 80;
        settings.position = WindowPosition::TopRight;

        store
            .save(&settings)
            .expect("saving to temp file should succeed");
        assert!(file_path.exists());

        let loaded = store.load();
        assert_eq!(loaded, settings);

        let _ = fs::remove_file(&file_path);
        let _ = fs::remove_dir(&temp_dir);
    }
}
