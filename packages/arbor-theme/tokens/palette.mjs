// ── Arbor Theme Palette ──────────────────────────────────
// All visual design decisions in one place.
// Run `pnpm generate` to rebuild themes/*.json from these tokens.

// ── Surface colours ──────────────────────────────────────
// Each theme defines a base hue + saturation (HSL space)
// and luminance stops for different surface roles.
// Dark themes: low L = deeper (chrome is darkest).
// Light themes: high L = brighter (editor is lightest).

// ── Dark variants ────────────────────────────────────────

export const nocturne = {
  name: 'Arbor Nocturne',
  type: 'dark',

  // Surface HSL base — blue-grey, like One Dark
  surfaceHue: 230,
  surfaceSaturation: 10,

  // Luminance stops (0–100). Lower = deeper / further back.
  surfaces: {
    chrome: 10,       // status bar, title bar, activity bar
    panel: 12,         // bottom panel
    sidebar: 14,       // side bar
    widget: 14,        // dropdowns, suggest widget, notifications
    input: 14,         // input fields
    editor: 18,        // main editor background (~ #282c34)
    lineHighlight: 21, // current line
    hover: 18,         // list/tab hover
    selection: 26,     // selection background
    border: 26,        // borders, indent guides
    ruler: 22,         // editor ruler
    whitespace: 22,    // whitespace dots
  },

  // ── Semantic UI colours ─────────────────────────────
  foreground: '#e8e4dc',
  accent: '#d4b884',
  accentMuted: '#d4b88480',
  error: '#f08890',
  warning: '#f0c878',
  info: '#68c8d8',
  hint: '#787880',
  muted: '#909098',
  dimmed: '#606068',

  added: '#b0d488',
  modified: '#f0c878',
  deleted: '#f08890',
  untracked: '#68c8d8',

  // ── Syntax token colours ────────────────────────────
  syntax: {
    comment: '#707888',
    string: '#b8e098',
    number: '#f0b860',
    keyword: '#e0b0f8',
    type_: '#60d8e8',
    function_: '#88c8f8',
    variable: '#e8e4dc',
    property: '#c8c0b8',
    operator: '#a8b0b8',
    punctuation: '#889098',
    attribute: '#f0d068',
    heading: '#f0b860',
    link: '#60d8e8',
    invalid: '#f87078',
    tag: '#60d8e8',
    tagPunct: '#707888',
    cssProp: '#e0b0f8',
    cssVal: '#b8e098',
    cssUnit: '#f0b860',
    cssSelector: '#88c8f8',
    jsonKey: '#e0b0f8',
    regex: '#f0b860',
    templateExpr: '#e8a850',
  },
}

// ── Deeper dark variant ──────────────────────────────────

export const nocturneDeep = {
  ...nocturne,
  name: 'Arbor Nocturne Deep',
  surfaces: {
    chrome: 6,
    panel: 8,
    sidebar: 10,
    widget: 10,
    input: 10,
    editor: 13,
    lineHighlight: 16,
    hover: 13,
    selection: 19,
    border: 19,
    ruler: 16,
    whitespace: 16,
  },
}

// ── Light variant ────────────────────────────────────────

export const vellum = {
  name: 'Arbor Vellum',
  type: 'light',

  // Paper-toned warm beige
  surfaceHue: 40,
  surfaceSaturation: 30,

  // Light theme: higher L = brighter / closer.
  // Editor is lightest, chrome is slightly darker.
  surfaces: {
    chrome: 88,       // status bar, title bar, activity bar
    panel: 91,         // bottom panel
    sidebar: 94,       // side bar
    widget: 94,        // dropdowns, suggest widget
    input: 96,         // input fields
    editor: 98,        // main editor background (paper)
    lineHighlight: 95, // current line
    hover: 92,         // list/tab hover
    selection: 89,     // selection background
    border: 85,        // borders
    ruler: 92,         // editor ruler
    whitespace: 86,    // whitespace dots
  },

  // ── Semantic UI colours ─────────────────────────────
  foreground: '#3a3832',
  accent: '#8b6b4a',
  accentMuted: '#8b6b4a80',
  error: '#9a4a4a',
  warning: '#9a7a4a',
  info: '#4a6b7a',
  hint: '#b0a898',
  muted: '#7a7060',
  dimmed: '#a09888',

  added: '#5a7a68',
  modified: '#9a8a50',
  deleted: '#9a5a50',
  untracked: '#4a6b7a',

  cursor: '#5a3a20',

  // ── Syntax token colours ────────────────────────────
  syntax: {
    comment: '#9a9080',
    string: '#2d8040',
    number: '#c06020',
    keyword: '#8848c0',
    type_: '#1880a0',
    function_: '#2860c0',
    variable: '#3a3832',
    property: '#584838',
    operator: '#706860',
    punctuation: '#888078',
    attribute: '#c08020',
    heading: '#b05018',
    link: '#1880a0',
    invalid: '#c83030',
    tag: '#1880a0',
    tagPunct: '#b0a898',
    cssProp: '#8848c0',
    cssVal: '#2d8040',
    cssUnit: '#c06020',
    cssSelector: '#2860c0',
    jsonKey: '#8848c0',
    regex: '#c06020',
    templateExpr: '#b05018',
  },
}

// ── Registry ─────────────────────────────────────────────
// Add new variants here; `pnpm generate` picks them up.

export const themes = { nocturne, nocturneDeep, vellum }
