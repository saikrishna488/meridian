import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import { test } from 'node:test';

// Test the shipped theme values, including aliases, against WCAG contrast ratios.
const source = await readFile(new URL('../src/lib/tokens.css', import.meta.url), 'utf8');
function declarations(block) {
  return Object.fromEntries([...block.matchAll(/--([\w-]+):\s*([^;]+);/g)].map(([, name, value]) => [name, value.trim()]));
}
const light = declarations(source.match(/:root\s*\{([^}]+)\}/)[1]);
const dark = { ...light, ...declarations(source.match(/:root\[data-theme="dark"\]\s*\{([^}]+)\}/)[1]) };
function color(palette, name) {
  const value = palette[name];
  const alias = value.match(/^var\(--([\w-]+)\)$/);
  if (alias) return color(palette, alias[1]);
  assert.match(value, /^#[\da-f]{6}$/i, `Expected solid color for ${name}`);
  return [1, 3, 5].map(i => parseInt(value.slice(i, i + 2), 16) / 255);
}
function luminance(rgb) {
  return rgb.map(c => c <= .04045 ? c / 12.92 : ((c + .055) / 1.055) ** 2.4).reduce((sum, c, i) => sum + c * [.2126, .7152, .0722][i], 0);
}
function contrast(a, b) {
  const values = [luminance(a), luminance(b)].sort((a, b) => b - a);
  return (values[0] + .05) / (values[1] + .05);
}
for (const [name, palette] of Object.entries({ light, dark })) {
  test(`${name} theme keeps text, actions and focus distinguishable`, () => {
    for (const bg of ['canvas', 'surface', 'surface-raised', 'surface-subtle']) {
      for (const fg of ['ink', 'ink-muted', 'action', 'danger', 'success']) {
        assert.ok(contrast(color(palette, fg), color(palette, bg)) >= 4.5, `${name}: ${fg} on ${bg}`);
      }
      assert.ok(contrast(color(palette, 'focus-ring'), color(palette, bg)) >= 3, `${name}: focus on ${bg}`);
    }
    assert.ok(contrast(color(palette, 'on-primary'), color(palette, 'control-track')) >= 3, `${name}: switch thumb on track`);
    for (const [fg, bg] of [['selection-ink', 'selection'], ['on-danger', 'button-danger'], ['on-danger', 'button-danger-hover'], ['on-primary', 'button-primary'], ['on-primary', 'button-primary-hover']]) {
      assert.ok(contrast(color(palette, fg), color(palette, bg)) >= 4.5, `${name}: ${fg} on ${bg}`);
    }
  });
}
