#!/usr/bin/env node

import { readFileSync, readdirSync } from 'node:fs'
import { join, dirname } from 'node:path'
import { fileURLToPath } from 'node:url'

const __dirname = dirname(fileURLToPath(import.meta.url))
const themesDir = join(__dirname, '..', 'themes')

// ── WCAG contrast requirements ────────────────────────────
const REQUIRED_CONTRAST_PAIRS = [
  { fg: 'editor.foreground', min: 4.5, label: '正文文字 vs 编辑器背景' },
  { fg: 'editorCursor.foreground', min: 4.5, label: '光标 vs 编辑器背景' },
  { fg: 'editor.lineNumber.foreground', min: 3.0, label: '行号 vs 编辑器背景' },
]

// ── Recommended color keys ────────────────────────────────
const RECOMMENDED_COLORS = [
  'editor.background',
  'editor.foreground',
  'editorCursor.foreground',
  'editor.lineNumber.foreground',
  'editor.selectionBackground',
  'editor.lineHighlightBackground',
]

// ── Required top-level fields ─────────────────────────────
const REQUIRED_FIELDS = ['name', 'type', 'colors', 'tokenColors']

// ── Helpers ───────────────────────────────────────────────

function stripJsonc(raw) {
  return raw
    .replace(/^\s*\/\/.*$/gm, '')       // line comments
    .replace(/\/\*[\s\S]*?\*\//g, '')   // block comments
    .replace(/,\s*([}\]])/g, '$1')      // trailing commas
}

function parseThemeFile(filepath) {
  const raw = readFileSync(filepath, 'utf-8')
  const stripped = stripJsonc(raw)
  return JSON.parse(stripped)
}

function hexToRgb(hex) {
  const m = /^#?([a-f\d]{2})([a-f\d]{2})([a-f\d]{2})$/i.exec(hex)
  if (!m) return null
  return { r: parseInt(m[1], 16), g: parseInt(m[2], 16), b: parseInt(m[3], 16) }
}

function relativeLuminance({ r, g, b }) {
  const [rs, gs, bs] = [r, g, b].map(c => {
    const s = c / 255
    return s <= 0.04045 ? s / 12.92 : ((s + 0.055) / 1.055) ** 2.4
  })
  return 0.2126 * rs + 0.7152 * gs + 0.0722 * bs
}

function contrastRatio(hex1, hex2) {
  const rgb1 = hexToRgb(hex1)
  const rgb2 = hexToRgb(hex2)
  if (!rgb1 || !rgb2) return null
  const l1 = relativeLuminance(rgb1)
  const l2 = relativeLuminance(rgb2)
  const lighter = Math.max(l1, l2)
  const darker = Math.min(l1, l2)
  return (lighter + 0.05) / (darker + 0.05)
}

function isValidHexColor(value) {
  return /^#[0-9a-fA-F]{6}([0-9a-fA-F]{2})?$/.test(value)
}

// ── Validators ────────────────────────────────────────────

function validateStructure(theme, filename) {
  const errors = []
  const warnings = []

  for (const field of REQUIRED_FIELDS) {
    if (theme[field] == null) {
      errors.push(`缺少必填字段: ${field}`)
    }
  }

  if (theme.type != null && !['light', 'dark'].includes(theme.type)) {
    errors.push(`type 必须是 "light" 或 "dark"，当前值: "${theme.type}"`)
  }

  if (theme.colors) {
    for (const key of RECOMMENDED_COLORS) {
      if (theme.colors[key] == null) {
        warnings.push(`colors 缺少推荐键: ${key}`)
      }
    }
  }

  if (theme.tokenColors != null && Array.isArray(theme.tokenColors)) {
    for (let i = 0; i < theme.tokenColors.length; i++) {
      const t = theme.tokenColors[i]
      const label = `tokenColors[${i}]` + (t.name ? ` (${t.name})` : '')

      if (!t.name) warnings.push(`${label}: 缺少 name`)
      if (!t.scope) warnings.push(`${label}: 缺少 scope`)
      if (!t.settings) {
        errors.push(`${label}: 缺少 settings`)
      }
    }
  }

  return { errors, warnings }
}

function validateColorFormats(theme) {
  const errors = []

  function check(value, path) {
    if (typeof value !== 'string') {
      errors.push(`${path}: 值不是字符串 (${typeof value})`)
      return
    }
    if (value.startsWith('#') && !isValidHexColor(value)) {
      errors.push(`${path}: "${value}" 不是有效的 hex 颜色（期望 #RRGGBB 或 #RRGGBBAA）`)
    }
  }

  if (theme.colors) {
    for (const [key, value] of Object.entries(theme.colors)) {
      if (typeof value === 'string' && value.startsWith('#')) {
        check(value, `colors.${key}`)
      }
    }
  }

  if (theme.tokenColors && Array.isArray(theme.tokenColors)) {
    for (let i = 0; i < theme.tokenColors.length; i++) {
      const t = theme.tokenColors[i]
      const fg = t.settings?.foreground
      if (fg && typeof fg === 'string' && fg.startsWith('#')) {
        check(fg, `tokenColors[${i}].settings.foreground`)
      }
    }
  }

  return { errors, warnings: [] }
}

function validateContrast(theme, filename) {
  const errors = []
  const warnings = []

  const bg = theme.colors?.['editor.background']
  if (!bg) return { errors, warnings }

  for (const pair of REQUIRED_CONTRAST_PAIRS) {
    const fg = theme.colors?.[pair.fg]
    if (!fg) continue

    const ratio = contrastRatio(fg, bg)
    if (ratio === null) {
      errors.push(`${pair.label}: 无法计算对比度（检查颜色格式）`)
      continue
    }

    if (ratio < pair.min) {
      const deficit = pair.min - ratio
      const result = deficit > 0.5 ? { list: errors, mark: '❌' } : { list: warnings, mark: '⚠️' }
      result.list.push(`${pair.label}: 对比度 ${ratio.toFixed(1)}:1（要求 ≥ ${pair.min}:1）`)
    }
  }

  return { errors, warnings }
}

// ── Main ──────────────────────────────────────────────────

function validateTheme(theme, filename) {
  const all = { errors: [], warnings: [] }

  const merge = (result) => {
    all.errors.push(...result.errors)
    all.warnings.push(...result.warnings)
  }

  merge(validateStructure(theme, filename))
  merge(validateColorFormats(theme))
  merge(validateContrast(theme, filename))

  return all
}

function main() {
  let totalErrors = 0
  let totalWarnings = 0

  const files = readdirSync(themesDir).filter(f => f.endsWith('.json')).sort()

  for (const file of files) {
    const filepath = join(themesDir, file)
    console.log(`\n━━━ ${file} ━━━`)

    let theme
    try {
      theme = parseThemeFile(filepath)
    } catch (e) {
      console.log(`  ❌ JSON 解析失败: ${e.message}`)
      totalErrors++
      continue
    }

    const { errors, warnings } = validateTheme(theme, file)

    if (errors.length === 0 && warnings.length === 0) {
      console.log('  ✅ 全部通过')
    }

    for (const e of errors) {
      console.log(`  ❌ ${e}`)
      totalErrors++
    }
    for (const w of warnings) {
      console.log(`  ⚠️  ${w}`)
      totalWarnings++
    }
  }

  console.log(`\n${'─'.repeat(40)}`)
  const passed = totalErrors === 0
  const icon = passed ? '✅' : '❌'
  console.log(`${icon} ${totalErrors} 个错误, ${totalWarnings} 个警告`)
  console.log()

  process.exit(passed ? 0 : 1)
}

main()
