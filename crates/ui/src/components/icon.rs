//! Icon primitive backed by embedded SVG assets.
//!
//! [`IconName`] is a curated set of ~140 common UI icons, each variant
//! mapping to an `icons/<snake_case>.svg` asset shipped inside this crate.
//! The SVGs are sourced from [Lucide](https://lucide.dev) (ISC License - see
//! `assets/icons/LICENSES`) at their canonical 24x24 stroke form. The actual
//! asset resolution is performed by gpui's `svg()` element, which goes
//! through the [`AssetSource`](gpui::AssetSource) registered on the
//! [`Application`]. The showcase wires up [`crate::assets::Assets`] for that
//! purpose.
//!
//! An [`Icon`] carries an [`IconSource`]: either a curated [`IconName`]
//! (Embedded), an image file on disk (External), or an SVG string path
//! resolved via the consumer app's AssetSource (ExternalSvg). This mirrors
//! zed's `ui::IconSource` shape with one intentional divergence - engram's
//! ExternalSvg routes through `svg().path(...)` (the AssetSource) rather
//! than zed's `svg().external_path(...)`, because the primary engram use
//! case for external SVGs is a consumer app bundling its own brand icons
//! into a combined AssetSource. `External` is the escape hatch for a path
//! the app never bundled - a user's own icon, say - and takes the direct
//! fs read for `.svg`.
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

/// Catalogue of every icon shipped with engram-ui. Each variant resolves to
/// `icons/<snake_case>.svg` via [`IconName::path`].
///
/// The set is curated from [Lucide](https://lucide.dev) - the names follow
/// Lucide's vocabulary except for a few engram-side renames where the
/// component layer already speaks differently (e.g. [`Self::Close`] ->
/// Lucide `x`, [`Self::Dash`] -> Lucide `minus`, [`Self::MagnifyingGlass`] ->
/// Lucide `search`, [`Self::Warning`] -> Lucide `triangle-alert`,
/// [`Self::XCircle`] -> Lucide `circle-x`). [`Self::StarFilled`] is a
/// derivative of Lucide's `star` with `fill="currentColor"`.
///
/// The set also carries the status vocabulary a desktop shell needs -
/// battery and Wi-Fi signal ladders, Bluetooth states, Ethernet, and
/// sensor icons - which a general-purpose UI catalogue has no reason to
/// ship.
///
/// Config files name icons the same way the assets do (`"battery_low"`), so
/// serde and strum agree on `snake_case`.
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
}

impl IconName {
    /// Asset path to this icon's SVG, e.g. `IconName::ArrowDown` ->
    /// `icons/arrow_down.svg`. Resolved by gpui's `svg().path(...)`.
    pub fn path(self) -> Arc<str> {
        let stem: &'static str = self.into();
        format!("icons/{stem}.svg").into()
    }
}

/// Where an [`Icon`] sources its pixels from.
///
/// Mirrors zed's `ui::IconSource`. See the module docs for the divergence
/// on `ExternalSvg` rendering.
#[derive(Debug, Clone)]
pub enum IconSource {
    /// A curated SVG from engram-ui's embedded bundle, keyed by
    /// [`IconName`]. Resolved via `svg().path(name.path())` against the
    /// consumer app's registered [`AssetSource`](gpui::AssetSource), which
    /// must include [`crate::assets::Assets`] (directly or wrapped).
    Embedded(IconName),
    /// An image file on disk. A `.svg` is read directly by gpui and stays
    /// tintable; anything else goes through `img(path)`, so callers can use
    /// arbitrary PNG/JPG sources - a user-supplied avatar, a file-type
    /// thumbnail, or a user's own icon named in a config file.
    External(Arc<Path>),
    /// An SVG at the given asset path. Rendered via `svg().path(...)`,
    /// which routes through the consumer's AssetSource. Consumer apps that
    /// ship brand/trademark icons outside engram's curated catalogue
    /// combine their own assets with [`crate::assets::Assets`] under a
    /// single AssetSource and reference them by path here.
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

    /// Construct an icon from an asset path string. The path is routed
    /// through the consumer's [`AssetSource`](gpui::AssetSource), so this
    /// is the preferred hook for apps that bundle their own (e.g. brand)
    /// SVGs alongside engram-ui's curated catalogue. Equivalent to
    /// `Icon::new(IconSource::ExternalSvg(path.into()))`.
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

        // `Exact` is the one size authored in device pixels; everything
        // else is rems so icons track the window's font scale.
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
