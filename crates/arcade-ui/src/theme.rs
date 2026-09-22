//! Solarized minimalist color system and glassy sheen styling tokens.

use gpui::{rgb, rgba, Rgba};

/// Palette defining the Solarized minimalist visual system with glassy sheen layers.
#[derive(Clone, Copy, Debug)]
pub struct SolarizedTheme {
    /// Deepest background canvas (Solarized dark base foundation).
    pub bg_canvas: Rgba,
    /// Translucent surface glass for sidebars, tabs, and status bars.
    pub bg_surface_glass: Rgba,
    /// Translucent acrylic surface for floating modals and command palette cards.
    pub bg_card_glass: Rgba,
    /// Glassy input field background.
    pub bg_input_glass: Rgba,
    /// Subtle top-down glassy sheen highlight overlay.
    pub bg_sheen_highlight: Rgba,
    /// Active element glassy highlight.
    pub bg_active_glass: Rgba,
    /// Hover state translucent sheen.
    pub bg_hover_glass: Rgba,

    /// Specular reflection top border (gives the Apple/MAUI glassy edge reflection).
    pub border_specular_top: Rgba,
    /// Standard translucent glass border.
    pub border_glass: Rgba,
    /// Subtle divider border.
    pub border_subtle: Rgba,
    /// Neon cyan focus glow border.
    pub border_focus_glow: Rgba,

    /// Primary text (Solarized base1).
    pub text_primary: Rgba,
    /// Emphasized/bright text (Solarized base3).
    pub text_bright: Rgba,
    /// Secondary text (Solarized base0).
    pub text_secondary: Rgba,
    /// Muted text, line numbers, comments (Solarized base01).
    pub text_muted: Rgba,

    /// Vibrant syntax: Cyan (keywords, storage, declarations).
    pub syntax_cyan: Rgba,
    /// Vibrant syntax: Blue (functions, procedures, methods).
    pub syntax_blue: Rgba,
    /// Vibrant syntax: Green (string literals, characters).
    pub syntax_green: Rgba,
    /// Vibrant syntax: Yellow (types, traits, structs, classes).
    pub syntax_yellow: Rgba,
    /// Vibrant syntax: Orange (numbers, constants, booleans).
    pub syntax_orange: Rgba,
    /// Vibrant syntax: Magenta (special keywords, operators, self).
    pub syntax_magenta: Rgba,
    /// Vibrant syntax: Violet (attributes, macros, annotations).
    pub syntax_violet: Rgba,

    /// Glass badge background.
    pub badge_bg: Rgba,
    /// Glass badge border.
    pub badge_border: Rgba,
}

impl Default for SolarizedTheme {
    fn default() -> Self {
        Self::dark()
    }
}

impl SolarizedTheme {
    /// Creates the Solarized Dark theme with luminous glassy sheen aesthetics.
    pub fn dark() -> Self {
        Self {
            // Deep oceanic teal-black foundation
            bg_canvas: rgb(0x001e26),
            // Translucent glass layers
            bg_surface_glass: rgba(0x002b36e8),
            bg_card_glass: rgba(0x073642f0),
            bg_input_glass: rgba(0x00212bc0),
            bg_sheen_highlight: rgba(0xffffff0f),
            bg_active_glass: rgba(0x2aa1982b),
            bg_hover_glass: rgba(0xffffff14),

            // Glass borders and specular top sheen
            border_specular_top: rgba(0xffffff33),
            border_glass: rgba(0x93a1a129),
            border_subtle: rgba(0x586e7540),
            border_focus_glow: rgb(0x2aa198),

            // Typography
            text_primary: rgb(0x93a1a1),
            text_bright: rgb(0xfdf6e3),
            text_secondary: rgb(0x839496),
            text_muted: rgb(0x586e75),

            // Vibrant syntax accents
            syntax_cyan: rgb(0x2aa198),
            syntax_blue: rgb(0x268bd2),
            syntax_green: rgb(0x859900),
            syntax_yellow: rgb(0xb58900),
            syntax_orange: rgb(0xcb4b16),
            syntax_magenta: rgb(0xd33682),
            syntax_violet: rgb(0x6c71c4),

            // Badges
            badge_bg: rgba(0x002b3699),
            badge_border: rgba(0x93a1a133),
        }
    }

    /// Creates the Solarized Light theme with luminous glassy sheen aesthetics.
    pub fn light() -> Self {
        Self {
            bg_canvas: rgb(0xfdf6e3),
            bg_surface_glass: rgba(0xeee8d5e8),
            bg_card_glass: rgba(0xfdf6e3f8),
            bg_input_glass: rgba(0xeee8d5c0),
            bg_sheen_highlight: rgba(0x00000008),
            bg_active_glass: rgba(0x2aa1982b),
            bg_hover_glass: rgba(0x00000010),

            border_specular_top: rgba(0xffffffcc),
            border_glass: rgba(0x586e7529),
            border_subtle: rgba(0x93a1a140),
            border_focus_glow: rgb(0x2aa198),

            text_primary: rgb(0x657b83),
            text_bright: rgb(0x073642),
            text_secondary: rgb(0x586e75),
            text_muted: rgb(0x93a1a1),

            syntax_cyan: rgb(0x2aa198),
            syntax_blue: rgb(0x268bd2),
            syntax_green: rgb(0x859900),
            syntax_yellow: rgb(0xb58900),
            syntax_orange: rgb(0xcb4b16),
            syntax_magenta: rgb(0xd33682),
            syntax_violet: rgb(0x6c71c4),

            badge_bg: rgba(0xeee8d599),
            badge_border: rgba(0x586e7533),
        }
    }

    /// Provides a subtle top-specular sheen highlight overlay style for panels.
    pub fn sheen_specular_top() -> Rgba {
        rgba(0xffffff38)
    }

    /// High-contrast glass pill background for selected items.
    pub fn active_pill_bg() -> Rgba {
        rgba(0x2aa19838)
    }
}

