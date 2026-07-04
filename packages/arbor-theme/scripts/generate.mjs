#!/usr/bin/env node

import { writeFileSync, mkdirSync } from 'node:fs'
import { join, dirname } from 'node:path'
import { fileURLToPath } from 'node:url'
import { themes } from '../tokens/palette.mjs'

const __dirname = dirname(fileURLToPath(import.meta.url))
const outDir = join(__dirname, '..', 'themes')

// ── HSL → Hex ────────────────────────────────────────────

function hslToHex(h, s, l) {
  const ss = s / 100
  const ll = l / 100
  const a = ss * Math.min(ll, 1 - ll)
  const f = (n) => {
    const k = (n + h / 30) % 12
    const color = ll - a * Math.max(Math.min(k - 3, 9 - k, 1), -1)
    return Math.round(255 * color).toString(16).padStart(2, '0')
  }
  return `#${f(0)}${f(8)}${f(4)}`
}

// ── Surface helpers ──────────────────────────────────────

const surface = (theme, stop) => hslToHex(theme.surfaceHue, theme.surfaceSaturation, stop)

const offset = (theme, delta) => {
  const s = theme.surfaces.editor + delta
  return surface(theme, s)
}

const alpha = (hex, a) => `${hex}${Math.round(a * 255).toString(16).padStart(2, '0')}`

// ── TokenColor builder ───────────────────────────────────

function token(name, scope, settings) {
  return { name, scope, settings }
}

function fg(color) {
  return { foreground: color }
}

function fgItalic(color) {
  return { foreground: color, fontStyle: 'italic' }
}

function fgBold(color) {
  return { foreground: color, fontStyle: 'bold' }
}

function buildTokenColors(s) {
  const tokens = [
    // Comments
    token('Comment',
      ['comment', 'punctuation.definition.comment'],
      fgItalic(s.comment)),
    token('Documentation comment',
      ['comment.block.documentation', 'comment.line.documentation'],
      fgItalic(s.comment)),

    // Strings
    token('String',
      ['string', 'punctuation.definition.string'],
      fg(s.string)),
    token('String Escape / Regexp',
      ['constant.character.escape', 'string.regexp'],
      fg(s.regex)),

    // Template expressions
    token('Template expression',
      ['punctuation.definition.template-expression.begin',
       'punctuation.definition.template-expression.end',
       'punctuation.section.embedded'],
      fg(s.templateExpr)),

    // Numbers & Constants
    token('Numeric',
      ['constant.numeric', 'constant.language', 'constant.character'],
      fg(s.number)),
    token('Boolean / Null',
      ['constant.language.boolean', 'constant.language.null'],
      fg(s.number)),

    // Keywords
    token('Control keywords',
      ['keyword.control', 'keyword.other'],
      fg(s.keyword)),
    token('Storage keywords',
      ['storage.type', 'storage.modifier', 'storage.type.class',
       'storage.type.function', 'storage.type.interface'],
      fg(s.keyword)),

    // Types
    token('Type / Class names',
      ['entity.name.type', 'entity.name.class', 'entity.name.enum',
       'entity.name.interface', 'entity.name.trait',
       'support.type', 'support.type.primitive'],
      fg(s.type_)),
    token('Type alias / Generic',
      ['entity.name.type.alias', 'entity.name.type.parameter'],
      fgItalic(s.type_)),

    // Functions
    token('Function names',
      ['entity.name.function', 'entity.name.function.macro', 'support.function'],
      fg(s.function_)),
    token('Method names',
      ['entity.name.function.member'],
      fg(s.function_)),

    // Variables
    token('Variables',
      ['variable', 'variable.other', 'variable.other.readwrite',
       'variable.parameter', 'variable.language'],
      fg(s.variable)),
    token('Variable property / member',
      ['variable.other.property', 'variable.other.object.property',
       'variable.other.enummember'],
      fg(s.property)),

    // Operators & Punctuation
    token('Operators',
      ['keyword.operator', 'keyword.operator.comparison',
       'keyword.operator.assignment', 'keyword.operator.arithmetic',
       'keyword.operator.logical'],
      fg(s.operator)),
    token('Punctuation',
      ['punctuation', 'punctuation.separator', 'punctuation.terminator',
       'punctuation.section', 'meta.brace', 'meta.delimiter'],
      fg(s.punctuation)),
    token('Punctuation accessor / dot',
      ['punctuation.accessor', 'punctuation.separator.period'],
      fg(s.operator)),

    // Attributes / Annotations
    token('Attributes / Decorators',
      ['entity.name.type.annotation', 'entity.name.type.decorator',
       'punctuation.definition.annotation', 'storage.type.annotation'],
      fg(s.attribute)),

    // Preprocessor / Imports
    token('Preprocessor / Macro',
      ['meta.preprocessor', 'keyword.control.directive', 'keyword.control.import'],
      fg(s.keyword)),
    token('Preprocessor string',
      ['meta.preprocessor string'],
      fg(s.string)),

    // Markup
    token('Markup Heading',
      ['markup.heading', 'markup.heading.markdown', 'entity.name.section'],
      fgBold(s.heading)),
    token('Markup Bold',
      ['markup.bold'],
      fgBold(s.attribute)),
    token('Markup Italic',
      ['markup.italic'],
      fgItalic(s.keyword)),
    token('Markup Quote',
      ['markup.quote'],
      fgItalic(s.comment)),
    token('Markup List',
      ['markup.list', 'punctuation.definition.list.begin'],
      fg(s.heading)),
    token('Markup Link',
      ['markup.underline.link', 'string.other.link', 'meta.link'],
      fg(s.link)),
    token('Markup Raw / Code',
      ['markup.inline.raw', 'markup.raw'],
      fg(s.string)),

    // JSON
    token('JSON keys',
      ['support.type.property-name.json', 'support.type.property-name'],
      fg(s.jsonKey)),

    // HTML/XML/JSX Tags
    token('Tag names',
      ['entity.name.tag'],
      fg(s.tag)),
    token('Tag attribute names',
      ['entity.other.attribute-name'],
      fg(s.keyword)),
    token('Tag delimiters',
      ['punctuation.definition.tag'],
      fg(s.tagPunct)),

    // CSS
    token('CSS property name',
      ['support.type.property-name.css'],
      fg(s.cssProp)),
    token('CSS property value',
      ['support.constant.property-value.css'],
      fg(s.cssVal)),
    token('CSS unit',
      ['keyword.other.unit'],
      fg(s.cssUnit)),
    token('CSS selectors',
      ['entity.other.attribute-name.class', 'entity.other.attribute-name.id'],
      fg(s.cssSelector)),
    token('CSS pseudo selectors',
      ['entity.other.attribute-name.pseudo-class',
       'entity.other.attribute-name.pseudo-element'],
      fg(s.attribute)),

    // Invalid / Deprecated
    token('Invalid',
      ['invalid', 'invalid.illegal'],
      { foreground: s.invalid, fontStyle: 'underline' }),
    token('Deprecated',
      ['invalid.deprecated'],
      { foreground: s.attribute, fontStyle: 'strikethrough' }),
  ]

  return tokens
}

// ── VSCode colour builder ────────────────────────────────

function buildColors(t) {
  const S = t.surfaces
  const isDark = t.type === 'dark'

  const bg = (stop) => surface(t, stop)
  const fg = t.foreground

  return {
    // ── Editor ────────────────────────────────────────
    'editor.background': bg(S.editor),
    'editor.foreground': fg,
    'editor.lineHighlightBackground': bg(S.lineHighlight),
    'editor.lineHighlightBorder': '#00000000',
    'editor.selectionBackground': bg(S.selection),
    'editor.selectionHighlightBackground': alpha(bg(S.selection), isDark ? 0.38 : 0.38),
    'editor.inactiveSelectionBackground': alpha(bg(S.selection), isDark ? 0.50 : 0.50),
    'editorCursor.foreground': t.cursor || t.accent,
    'editor.findMatchBackground': alpha(t.accent, 0.50),
    'editor.findMatchHighlightBackground': alpha(t.accent, 0.44),
    'editor.findRangeHighlightBackground': alpha(bg(S.selection), 0.25),
    'editor.rangeHighlightBackground': alpha(bg(S.selection), 0.25),
    'editor.wordHighlightBackground': alpha(bg(S.selection), 0.25),
    'editor.wordHighlightStrongBackground': alpha(bg(S.selection), 0.38),
    'editor.lineNumber.foreground': isDark ? '#808090' : '#9a8878',
    'editor.lineNumber.activeForeground': isDark ? '#a8a8b0' : '#6a6050',
    'editorGutter.background': bg(S.editor),
    'editorGutter.addedBackground': alpha(t.added, 0.38),
    'editorGutter.modifiedBackground': alpha(t.modified, 0.38),
    'editorGutter.deletedBackground': alpha(t.deleted, 0.38),
    'editorRuler.foreground': bg(S.ruler),
    'editorIndentGuide.background': bg(S.hover),
    'editorIndentGuide.activeBackground': bg(S.border),
    'editorWhitespace.foreground': bg(S.whitespace),
    'editorBracketMatch.background': bg(S.selection),
    'editorBracketMatch.border': surface(t, isDark ? 26 : 72),
    'editorOverviewRuler.border': bg(S.widget),
    'editorOverviewRuler.findMatchForeground': alpha(t.accent, 0.38),
    'editorOverviewRuler.selectionHighlightForeground': alpha(t.accent, 0.25),
    'editorOverviewRuler.wordHighlightForeground': alpha(t.info, 0.25),
    'editorError.foreground': t.error,
    'editorWarning.foreground': t.warning,
    'editorInfo.foreground': t.info,
    'editorHint.foreground': t.hint,
    'editorLightBulb.foreground': t.accent,
    'editorLink.activeForeground': t.info,
    'editorWidget.background': bg(S.widget),
    'editorWidget.border': bg(S.border),
    'editorHoverWidget.background': bg(S.hover),
    'editorHoverWidget.border': surface(t, isDark ? S.border + 4 : S.border - 4),
    'editorSuggestWidget.background': bg(S.widget),
    'editorSuggestWidget.border': bg(S.border),
    'editorSuggestWidget.selectedBackground': bg(S.hover),
    'editorSuggestWidget.highlightForeground': t.accent,
    'editorCodeLens.foreground': t.dimmed,
    'editorGroupHeader.tabsBackground': bg(S.widget),
    'editorGroupHeader.tabsBorder': bg(S.chrome),
    'editorPane.background': bg(S.editor),

    // ── Workbench UI ──────────────────────────────────
    'focusBorder': t.accentMuted,
    'contrastBorder': bg(S.chrome),
    'contrastActiveBorder': t.accent,

    // Activity Bar
    'activityBar.background': bg(S.chrome),
    'activityBar.foreground': t.muted,
    'activityBar.inactiveForeground': t.dimmed,
    'activityBar.activeBorder': t.accent,
    'activityBar.activeBackground': bg(S.widget),
    'activityBar.border': bg(S.widget),
    'activityBarBadge.background': t.accent,
    'activityBarBadge.foreground': bg(S.chrome),

    // Side Bar
    'sideBar.background': bg(S.sidebar),
    'sideBar.foreground': isDark ? '#b8b8c0' : '#4a4238',
    'sideBar.border': bg(S.chrome),
    'sideBarTitle.foreground': isDark ? '#d0d0d8' : '#38342e',
    'sideBarSectionHeader.background': bg(S.sidebar),
    'sideBarSectionHeader.foreground': isDark ? '#a8a8b0' : '#5a5242',
    'sideBarSectionHeader.border': bg(S.hover),

    // Status Bar
    'statusBar.background': bg(S.chrome),
    'statusBar.foreground': isDark ? '#a8a8b0' : '#5a5242',
    'statusBar.border': bg(S.widget),
    'statusBar.debuggingBackground': t.error,
    'statusBar.debuggingForeground': fg,
    'statusBar.noFolderBackground': bg(S.chrome),
    'statusBar.noFolderForeground': isDark ? '#a8a8b0' : '#5a5242',
    'statusBarItem.remoteBackground': bg(S.hover),
    'statusBarItem.remoteForeground': isDark ? '#d0d0d8' : '#4a4238',
    'statusBarItem.activeBackground': bg(S.hover),
    'statusBarItem.hoverBackground': bg(S.hover),
    'statusBarItem.prominentBackground': bg(S.border),
    'statusBarItem.prominentHoverBackground': surface(t, isDark ? S.border + 4 : S.border - 4),

    // Title Bar
    'titleBar.activeBackground': bg(S.chrome),
    'titleBar.activeForeground': isDark ? '#d0d0d8' : '#38342e',
    'titleBar.inactiveBackground': bg(S.chrome),
    'titleBar.inactiveForeground': t.dimmed,
    'titleBar.border': bg(S.widget),

    // Panel
    'panel.background': bg(S.panel),
    'panel.border': bg(S.hover),
    'panelTitle.activeForeground': isDark ? '#d0d0d8' : '#38342e',
    'panelTitle.inactiveForeground': t.dimmed,
    'panelInput.border': bg(S.border),

    // Tabs
    'tab.activeBackground': bg(S.editor),
    'tab.activeForeground': fg,
    'tab.activeBorderTop': t.accent,
    'tab.inactiveBackground': bg(S.sidebar),
    'tab.inactiveForeground': t.dimmed,
    'tab.border': bg(S.chrome),
    'tab.hoverBackground': bg(S.hover),
    'tab.hoverForeground': isDark ? '#d0d0d8' : '#4a4238',
    'tab.unfocusedActiveForeground': isDark ? '#a8a8b0' : '#6a6050',
    'tab.unfocusedInactiveForeground': t.dimmed,

    // Breadcrumb
    'breadcrumb.background': bg(S.sidebar),
    'breadcrumb.foreground': t.muted,
    'breadcrumb.focusForeground': isDark ? '#d0d0d8' : '#4a4238',
    'breadcrumb.activeSelectionForeground': t.accent,

    // Input / Dropdown
    'input.background': bg(S.input),
    'input.foreground': fg,
    'input.border': bg(S.border),
    'input.placeholderForeground': t.dimmed,
    'inputOption.activeBackground': bg(S.border),
    'inputOption.activeBorder': t.accent,
    'inputOption.activeForeground': fg,
    'inputValidation.errorBackground': alpha(t.error, 0.25),
    'inputValidation.errorBorder': t.error,
    'inputValidation.warningBackground': alpha(t.warning, 0.25),
    'inputValidation.warningBorder': t.warning,
    'inputValidation.infoBackground': alpha(t.info, 0.25),
    'inputValidation.infoBorder': t.info,
    'dropdown.background': bg(S.widget),
    'dropdown.foreground': fg,
    'dropdown.border': bg(S.border),

    // List / Tree
    'list.activeSelectionBackground': bg(S.hover),
    'list.activeSelectionForeground': fg,
    'list.inactiveSelectionBackground': bg(S.hover),
    'list.inactiveSelectionForeground': isDark ? '#b8b8c0' : '#4a4238',
    'list.hoverBackground': bg(S.selection),
    'list.hoverForeground': fg,
    'list.focusBackground': bg(S.selection),
    'list.focusForeground': fg,
    'list.highlightForeground': t.accent,
    'list.errorForeground': t.error,
    'list.warningForeground': t.warning,
    'tree.indentGuidesStroke': bg(S.border),

    // Badge
    'badge.background': t.accent,
    'badge.foreground': bg(S.chrome),

    // Scrollbar
    'scrollbar.shadow': isDark ? '#00000030' : '#00000010',
    'scrollbarSlider.background': alpha(bg(S.border), 0.38),
    'scrollbarSlider.hoverBackground': alpha(bg(S.border), 0.50),
    'scrollbarSlider.activeBackground': alpha(bg(S.border), 0.63),

    // Notifications
    'notifications.background': bg(S.widget),
    'notifications.foreground': fg,
    'notifications.border': bg(S.border),
    'notificationCenter.border': bg(S.border),
    'notificationToast.border': bg(S.border),
    'notificationCenterHeader.background': bg(S.panel),

    // Quick Input
    'quickInput.background': bg(S.widget),
    'quickInput.foreground': fg,

    // Terminal
    'terminal.background': bg(S.input),
    'terminal.foreground': fg,
    'terminal.ansiBlack': bg(S.widget),
    'terminal.ansiRed': isDark ? '#f08890' : '#9a4a4a',
    'terminal.ansiGreen': isDark ? '#b0d488' : '#4a6b5a',
    'terminal.ansiYellow': isDark ? '#f0c878' : '#9a7a4a',
    'terminal.ansiBlue': isDark ? '#68c8d8' : '#4a6b7a',
    'terminal.ansiMagenta': isDark ? '#d0a8e8' : '#6b5580',
    'terminal.ansiCyan': isDark ? '#68c8d8' : '#4a6b7a',
    'terminal.ansiWhite': fg,
    'terminal.ansiBrightBlack': t.dimmed,
    'terminal.ansiBrightRed': isDark ? '#f8a0a8' : '#b05a5a',
    'terminal.ansiBrightGreen': isDark ? '#c8e8a0' : '#5a7a68',
    'terminal.ansiBrightYellow': isDark ? '#f8d898' : '#b08a5a',
    'terminal.ansiBrightBlue': isDark ? '#80d8e8' : '#5a7a8a',
    'terminal.ansiBrightMagenta': isDark ? '#e0bcf0' : '#7a6a90',
    'terminal.ansiBrightCyan': isDark ? '#80d8e8' : '#5a7a8a',
    'terminal.ansiBrightWhite': isDark ? '#f8f4f0' : '#282420',
    'terminal.selectionBackground': bg(S.selection),

    // Debug
    'debugToolBar.background': bg(S.widget),
    'debugExceptionWidget.background': bg(S.widget),
    'debugExceptionWidget.border': t.error,

    // Diff
    'diffEditor.insertedTextBackground': alpha(t.added, 0.19),
    'diffEditor.removedTextBackground': alpha(t.deleted, 0.19),
    'diffEditor.insertedLineBackground': alpha(t.added, 0.13),
    'diffEditor.removedLineBackground': alpha(t.deleted, 0.13),
    'diffEditor.border': bg(S.hover),

    // Peek View
    'peekView.border': t.accent,
    'peekViewEditor.background': bg(S.input),
    'peekViewEditor.matchHighlightBackground': alpha(t.accent, 0.50),
    'peekViewResult.background': bg(S.widget),
    'peekViewResult.matchHighlightBackground': alpha(t.accent, 0.50),
    'peekViewResult.selectionBackground': bg(S.hover),
    'peekViewTitle.background': bg(S.panel),
    'peekViewTitleDescription.background': bg(S.hover),

    // Merge Conflicts
    'merge.currentHeaderBackground': surface(t, isDark ? S.selection + 4 : S.selection - 4),
    'merge.currentContentBackground': alpha(surface(t, isDark ? S.selection + 4 : S.selection - 4), 0.13),
    'merge.incomingHeaderBackground': surface(t, isDark ? S.selection + 2 : S.selection - 2),
    'merge.incomingContentBackground': alpha(surface(t, isDark ? S.selection + 2 : S.selection - 2), 0.13),
    'merge.commonHeaderBackground': bg(S.widget),
    'merge.commonContentBackground': alpha(bg(S.selection), 0.25),

    // Git Decoration
    'gitDecoration.addedResourceForeground': t.added,
    'gitDecoration.modifiedResourceForeground': t.modified,
    'gitDecoration.deletedResourceForeground': t.deleted,
    'gitDecoration.untrackedResourceForeground': t.untracked,
    'gitDecoration.ignoredResourceForeground': t.dimmed,
    'gitDecoration.conflictingResourceForeground': isDark ? '#e0b860' : '#8b6b4a',
    'gitDecoration.submoduleResourceForeground': isDark ? '#d0a8e8' : '#6b5580',

    // Minimap
    'minimap.background': bg(S.sidebar),
    'minimap.selectionHighlight': alpha(t.accent, 0.38),
    'minimap.findMatchHighlight': alpha(t.accent, 0.50),
    'minimap.errorHighlight': alpha(t.error, 0.50),
    'minimap.warningHighlight': alpha(t.warning, 0.50),

    // Button
    'button.background': t.accent,
    'button.foreground': bg(S.chrome),
    'button.hoverBackground': surface(t, isDark ? S.selection + 2 : S.selection - 2),
    'button.secondaryBackground': bg(S.selection),
    'button.secondaryForeground': isDark ? '#d0d0d8' : '#4a4238',
    'button.secondaryHoverBackground': bg(S.border),

    // Progress Bar
    'progressBar.background': t.accent,

    // Misc
    'textLink.foreground': t.info,
    'textLink.activeForeground': t.info,
    'textBlockQuote.background': bg(S.widget),
    'textBlockQuote.border': t.accent,
    'textCodeBlock.background': bg(S.input),
    'textSeparator.foreground': bg(S.hover),
    'descriptionForeground': t.muted,
    'widget.shadow': isDark ? '#00000040' : '#00000010',

    // Welcome / Walkthrough
    'welcomePage.background': bg(S.widget),
    'walkThrough.embeddedEditorBackground': bg(S.widget),
  }
}

// ── JSONC formatter ──────────────────────────────────────

const SECTION_ORDER = [
  ['Editor', ['editor.', 'editorGutter.', 'editorRuler.', 'editorIndentGuide.',
    'editorWhitespace.', 'editorBracketMatch.', 'editorOverviewRuler.',
    'editorError.', 'editorWarning.', 'editorInfo.', 'editorHint.',
    'editorLightBulb.', 'editorLink.', 'editorWidget.', 'editorHoverWidget.',
    'editorSuggestWidget.', 'editorCodeLens.', 'editorGroupHeader.', 'editorPane.',
    'editorCursor.', 'editorLineNumber.',
    'editor.findMatch', 'editor.selection', 'editor.range',
    'editor.wordHighlight', 'editor.inactiveSelection']],
  ['Workbench UI', ['focusBorder', 'contrastBorder', 'contrastActiveBorder']],
  ['Activity Bar', ['activityBar.', 'activityBarBadge.']],
  ['Side Bar', ['sideBar.', 'sideBarTitle.', 'sideBarSectionHeader.']],
  ['Status Bar', ['statusBar.', 'statusBarItem.']],
  ['Title Bar', ['titleBar.']],
  ['Panel', ['panel.', 'panelTitle.', 'panelInput.']],
  ['Tabs', ['tab.', 'tab.unfocused']],
  ['Breadcrumb', ['breadcrumb.']],
  ['Input / Dropdown', ['input.', 'inputOption.', 'inputValidation.', 'dropdown.']],
  ['List / Tree', ['list.', 'tree.']],
  ['Badge', ['badge.']],
  ['Scrollbar', ['scrollbar.', 'scrollbarSlider.']],
  ['Notifications', ['notifications.', 'notificationCenter.', 'notificationToast.', 'notificationCenterHeader.']],
  ['Quick Input', ['quickInput.']],
  ['Terminal', ['terminal.']],
  ['Debug', ['debug']],
  ['Diff', ['diffEditor.']],
  ['Peek View', ['peekView.']],
  ['Merge Conflicts', ['merge.']],
  ['Git Decoration', ['gitDecoration.']],
  ['Minimap', ['minimap.']],
  ['Button', ['button.']],
  ['Progress Bar', ['progressBar.']],
  ['Misc', ['textLink.', 'textBlockQuote.', 'textCodeBlock.', 'textSeparator.', 'descriptionForeground', 'widget.shadow']],
  ['Welcome / Walkthrough', ['welcomePage.', 'walkThrough.']],
]

function sectionFor(key) {
  for (const [title, prefixes] of SECTION_ORDER) {
    for (const pf of prefixes) {
      if (key.startsWith(pf)) return title
    }
  }
  return 'Misc'
}

function formatJsonc(obj, tokenColors) {
  const lines = []
  lines.push('{')

  // Top-level fields
  lines.push(`  "name": ${JSON.stringify(obj.name)},`)
  lines.push(`  "type": ${JSON.stringify(obj.type)},`)
  lines.push(`  "semanticHighlighting": true,`)
  lines.push('')

  // ── Colors ────────────────────────────────────────────
  lines.push('  "colors": {')

  const colors = obj.colors
  // Build flat entry list: { key, section }
  const entries = Object.keys(colors).sort().map(k => ({
    key: k,
    section: sectionFor(k),
  }))

  // Sort by section order, then by key
  const sectionIndex = new Map()
  SECTION_ORDER.forEach(([title], i) => sectionIndex.set(title, i))
  entries.sort((a, b) => {
    const si = (sectionIndex.get(a.section) ?? 99) - (sectionIndex.get(b.section) ?? 99)
    if (si !== 0) return si
    return a.key.localeCompare(b.key)
  })

  let lastSection = ''
  for (let i = 0; i < entries.length; i++) {
    const { key, section } = entries[i]
    const isLast = i === entries.length - 1
    const comma = isLast ? '' : ','

    if (section !== lastSection) {
      if (lastSection !== '') lines.push('')
      lines.push(`    // ── ${section} ${'─'.repeat(Math.max(1, 52 - section.length))}`)
      lastSection = section
    }

    lines.push(`    ${JSON.stringify(key)}: ${JSON.stringify(colors[key])}${comma}`)
  }

  lines.push('  },')
  lines.push('')

  // ── Token Colors ──────────────────────────────────────
  lines.push('  "tokenColors": [')

  const tokenGroups = [
    ['Comments', ['Comment']],
    ['Strings (green)', ['String', 'Template expression']],
    ['Numbers & Constants (amber)', ['Numeric', 'Boolean']],
    ['Keywords (violet)', ['Control', 'Storage']],
    ['Types (cyan)', ['Type']],
    ['Functions (blue)', ['Function', 'Method']],
    ['Variables (warm white)', ['Variables', 'Variable property']],
    ['Operators & Punctuation', ['Operators', 'Punctuation', 'Punctuation accessor']],
    ['Attributes / Annotations (gold)', ['Attributes']],
    ['Meta / Preprocessor', ['Preprocessor']],
    ['Markup', ['Markup']],
    ['JSON / YAML', ['JSON']],
    ['Entity / Tags (HTML/XML/JSX)', ['Tag names', 'Tag attribute', 'Tag delimiters']],
    ['CSS / SCSS', ['CSS']],
    ['Invalid / Errors', ['Invalid', 'Deprecated']],
  ]

  // Build flat token list with group info
  const tokenEntries = tokenColors.map(t => {
    for (const [groupTitle, namePrefixes] of tokenGroups) {
      if (namePrefixes.some(p => t.name.startsWith(p))) {
        return { token: t, group: groupTitle }
      }
    }
    return { token: t, group: 'Misc' }
  })

  // Sort by group order
  const tokenGroupIndex = new Map()
  tokenGroups.forEach(([title], i) => tokenGroupIndex.set(title, i))
  tokenEntries.sort((a, b) => {
    const gi = (tokenGroupIndex.get(a.group) ?? 99) - (tokenGroupIndex.get(b.group) ?? 99)
    if (gi !== 0) return gi
    return a.token.name.localeCompare(b.token.name)
  })

  let lastTokenGroup = ''
  for (let i = 0; i < tokenEntries.length; i++) {
    const { token: t, group } = tokenEntries[i]
    const isLast = i === tokenEntries.length - 1
    const comma = isLast ? '' : ','

    if (group !== lastTokenGroup) {
      if (lastTokenGroup !== '') lines.push('')
      lines.push(`    // ── ${group} ${'─'.repeat(Math.max(1, 48 - group.length))}`)
      lastTokenGroup = group
    }

    lines.push('    {')
    lines.push(`      "name": ${JSON.stringify(t.name)},`)
    lines.push(`      "scope": ${JSON.stringify(t.scope)},`)
    lines.push(`      "settings": ${JSON.stringify(t.settings)}`)
    lines.push(`    }${comma}`)
  }

  lines.push('  ]')
  lines.push('}')
  lines.push('')

  return lines.join('\n')
}

// ── Main ──────────────────────────────────────────────────

function generate() {
  mkdirSync(outDir, { recursive: true })

  for (const [id, palette] of Object.entries(themes)) {
    const colors = buildColors(palette)
    const tokenColors = buildTokenColors(palette.syntax)
    const filename = `${id}.json`

    const theme = {
      name: palette.name,
      type: palette.type,
      semanticHighlighting: true,
      colors,
    }

    const output = formatJsonc(theme, tokenColors)
    writeFileSync(join(outDir, filename), output, 'utf-8')
    console.log(`  ✅  ${filename}  (${palette.name})`)
  }
}

console.log('\n🎨  Generating Arbor themes…\n')
generate()
console.log('\n📦  Done. Run `pnpm validate` to check contrast ratios.\n')
