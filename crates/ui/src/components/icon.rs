//! Icon primitive backed by embedded SVG assets.
//!
//! [`IconName`] is a curated set of ~140 icons, each mapping to an
//! `icons/<snake_case>.svg` shipped in this crate. The SVGs come from
//! [Lucide](https://lucide.dev) (ISC License - see `assets/icons/LICENSES`).
//! Resolution goes through the [`AssetSource`](gpui::AssetSource) registered
//! on the [`Application`], which must include [`crate::assets::Assets`].
//!
//! An [`Icon`] carries an [`IconSource`]: a curated [`IconName`] (Embedded),
//! an asset path routed through the AssetSource (ExternalSvg, for apps
//! bundling their own brand icons), or a file read straight off disk
//! (External).
//!
//! Need an icon that isn't in the catalogue? Either drop the SVG into the
//! consuming app's own asset source and use [`Icon::from_path`], or open a
//! PR adding the variant.

use std::path::Path;
use std::sync::Arc;

use crate::theme::{ActiveTheme, Color};
use gpui::{App, IntoElement, Length, RenderOnce, SharedString, Window, img, prelude::*, px, svg};
use serde::{Deserialize, Serialize};
use strum::{EnumIter, EnumString, IntoStaticStr};

pub use crate::theme::IconSize;

/// Catalogue of every icon shipped with this crate.
#[derive(
    Debug,
    PartialEq,
    Eq,
    Copy,
    Clone,
    Hash,
    IntoStaticStr,
    EnumIter,
    EnumString,
    Serialize,
    Deserialize,
)]
#[strum(serialize_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum IconName {
    ArrowDown,
    ArrowDownLeft,
    ArrowDownRight,
    ArrowLeft,
    ArrowRight,
    ArrowRightLeft,
    ArrowUp,
    ArrowUpLeft,
    ArrowUpRight,
    AtSign,
    Bell,
    BellOff,
    BellRing,
    Bookmark,
    Bug,
    Calendar,
    Camera,
    Chat,
    Check,
    CheckCircle,
    CheckDouble,
    ChevronDown,
    ChevronLeft,
    ChevronRight,
    ChevronUp,
    ChevronsLeftRight,
    ChevronsUpDown,
    Circle,
    CircleAlert,
    CircleHelp,
    Clipboard,
    Clock,
    Close,
    Cloud,
    Code,
    Copy,
    Cpu,
    Dash,
    Database,
    Diamond,
    Download,
    Ellipsis,
    EllipsisVertical,
    Eraser,
    ExternalLink,
    Eye,
    EyeOff,
    FastForward,
    File,
    FileCode,
    FileDiff,
    FileImage,
    FileLock,
    FilePlus,
    FileText,
    Filter,
    Flag,
    Flame,
    Folder,
    FolderOpen,
    FolderPlus,
    FolderSearch,
    Gamepad,
    GitBranch,
    GitCommit,
    GitMerge,
    GitPullRequest,
    Globe,
    Hash,
    Headphones,
    Heart,
    Hexagon,
    History,
    Home,
    Image,
    Info,
    Keyboard,
    Languages,
    Layers,
    Layout,
    Link,
    List,
    ListFilter,
    ListOrdered,
    ListTodo,
    ListTree,
    LoaderCircle,
    Lock,
    MagnifyingGlass,
    Mail,
    Maximize,
    Menu,
    Mic,
    MicOff,
    Minimize,
    Moon,
    Mouse,
    Music,
    Paperclip,
    Pause,
    Pencil,
    Phone,
    Pin,
    PinOff,
    Play,
    Plus,
    Power,
    Quote,
    Refresh,
    RotateCcw,
    RotateCw,
    Save,
    Scissors,
    Send,
    Server,
    Settings,
    Share,
    Sidebar,
    SkipBack,
    SkipForward,
    Sliders,
    Sparkles,
    Square,
    Star,
    StarFilled,
    Stop,
    Sun,
    SunDim,
    SunMedium,
    Table,
    Terminal,
    ThumbsDown,
    ThumbsUp,
    Timer,
    Trash,
    Triangle,
    Unlock,
    Upload,
    User,
    UserCheck,
    UserGroup,
    UserPlus,
    UserRound,
    Volume,
    VolumeLow,
    VolumeMedium,
    VolumeOff,
    Warning,
    XCircle,
    ArrowDownUp,
    Battery,
    BatteryCharging,
    BatteryFull,
    BatteryLow,
    BatteryMedium,
    BatteryWarning,
    Bluetooth,
    BluetoothConnected,
    BluetoothOff,
    BookOpen,
    ChartPie,
    Ethernet,
    Gauge,
    HardDrive,
    Inbox,
    Map,
    MemoryStick,
    Network,
    Palette,
    ScreenShare,
    SquareTerminal,
    Thermometer,
    Webcam,
    Wifi,
    WifiHigh,
    WifiLow,
    WifiOff,
    WifiZero,
    Zap,
    Alacritty,
    Bitwarden,
    Chrome,
    Discord,
    Dropbox,
    Duckduckgo,
    Firefox,
    Github,
    Google,
    Kde,
    Neovim,
    Nextcloud,
    Nixos,
    OnePassword,
    Reddit,
    Rust,
    Slack,
    Spotify,
    Syncthing,
    Tailscale,
    Telegram,
    Thunderbird,
    VisualStudioCode,
    Wezterm,
    Wikipedia,
    Youtube,
    Zed,
}

impl IconName {
    /// Asset path to this icon's SVG, e.g. `IconName::ArrowDown` ->
    /// `icons/arrow_down.svg`.
    pub fn path(self) -> Arc<str> {
        let stem: &'static str = self.into();
        format!("icons/{stem}.svg").into()
    }
}

/// Where an [`Icon`] sources its pixels from.
#[derive(Debug, Clone)]
pub enum IconSource {
    /// A curated SVG from this crate's embedded bundle, keyed by [`IconName`].
    Embedded(IconName),
    /// An image file on disk.
    External(Arc<Path>),
    /// An SVG at the given asset path.
    ExternalSvg(SharedString),
}

impl From<IconName> for IconSource {
    fn from(name: IconName) -> Self {
        Self::Embedded(name)
    }
}

fn is_svg(path: &Path) -> bool {
    path.extension()
        .is_some_and(|ext| ext.eq_ignore_ascii_case("svg"))
}

/// An icon resolved from an [`IconSource`].
#[derive(IntoElement)]
#[must_use = "Icon does nothing unless rendered"]
pub struct Icon {
    source: IconSource,
    size: IconSize,
    color: Color,
}

impl Icon {
    /// Build an icon from anything that can become an [`IconSource`] - an
    /// [`IconName`] (the common path, via `From<IconName> for IconSource`),
    /// or an explicit `IconSource::External`/`ExternalSvg`.
    pub fn new(source: impl Into<IconSource>) -> Self {
        Self {
            source: source.into(),
            size: IconSize::default(),
            color: Color::default(),
        }
    }

    /// Construct an icon from an asset path string.
    pub fn from_path(path: impl Into<SharedString>) -> Self {
        Self::new(IconSource::ExternalSvg(path.into()))
    }

    pub fn size(mut self, size: IconSize) -> Self {
        self.size = size;
        self
    }

    pub fn color(mut self, color: Color) -> Self {
        self.color = color;
        self
    }
}

impl RenderOnce for Icon {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        // Icons resolve against the `icon*` tokens, not the text tokens, so
        // `Color::Default` lands on `theme.colors.icon`.
        let colors = cx.theme().colors();
        let hsla = match self.color {
            Color::Default => colors.icon,
            Color::Muted => colors.icon_muted,
            Color::Disabled => colors.icon_disabled,
            Color::Accent => colors.icon_accent,
            other => other.hsla(colors),
        };

        // `Exact` is the one size authored in device pixels; everything else
        // is rems so icons track the window's font scale.
        let size: Length = match self.size {
            IconSize::Exact(p) => px(p).into(),
            other => other.rems().into(),
        };

        match self.source {
            IconSource::Embedded(name) => svg()
                .size(size)
                .flex_none()
                .path(name.path())
                .text_color(hsla)
                .into_any_element(),
            IconSource::ExternalSvg(path) => svg()
                .size(size)
                .flex_none()
                .path(path)
                .text_color(hsla)
                .into_any_element(),
            // An SVG on disk can't go through `img()` - that decodes raster
            // formats - so it takes gpui's direct-read path instead.
            IconSource::External(path) if is_svg(&path) => svg()
                .size(size)
                .flex_none()
                .external_path(path.to_string_lossy().into_owned())
                .text_color(hsla)
                .into_any_element(),
            IconSource::External(path) => img(path)
                .size(size)
                .flex_none()
                .text_color(hsla)
                .into_any_element(),
        }
    }
}
