import assert from 'node:assert/strict';
import { test } from 'node:test';
import { build } from 'esbuild';
import { JSDOM } from 'jsdom';
import { fileURLToPath } from 'node:url';

const bundle = await build({
  stdin: { contents: `
    import * as React from 'react';
    import { createRoot } from 'react-dom/client';
    import { Button } from './src/components/ui/button';
    import { Dialog } from './src/components/ui/dialog';
    import { Tabs } from './src/components/ui/tabs';
    import { Select } from './src/components/ui/select';
    window.fixture = { ...React, createRoot, Button, Dialog, Tabs, Select };
  `, resolveDir: fileURLToPath(new URL('..', import.meta.url)), loader: 'tsx' },
  bundle: true, write: false, format: 'iife', platform: 'browser',
  define: { 'process.env.NODE_ENV': '"development"' }, logLevel: 'silent',
});
async function fixture(t) {
  const dom = new JSDOM('<button id="outside">Outside</button><div id="root"></div>', { runScripts: 'outside-only', pretendToBeVisual: true });
  const w = dom.window;
  // jsdom lacks MessageChannel; React act only needs a next-task callback.
  w.MessageChannel = class {
    port1 = { onmessage: null };
    port2 = { postMessage: () => w.setTimeout(() => this.port1.onmessage?.(), 0) };
  };
  w.IS_REACT_ACT_ENVIRONMENT = true;
  w.HTMLElement.prototype.scrollIntoView = function () {};
  w.eval(bundle.outputFiles[0].text);
  const api = w.fixture, root = api.createRoot(w.document.getElementById('root'));
  t.after(async () => { await api.act(() => root.unmount()); w.close(); });
  const h = api.createElement;
  return { ...api, h, w, get: selector => w.document.querySelector(selector),
    render: async node => api.act(() => root.render(node)),
    key: async (element, key, shiftKey = false) => api.act(() => element.dispatchEvent(new w.KeyboardEvent('keydown', { key, shiftKey, bubbles: true, cancelable: true }))),
    click: async element => api.act(() => element.click()),
  };
}

test('shared buttons preserve native refs, explicit submit and disabled semantics', async t => {
  const ui = await fixture(t), ref = ui.createRef(); let submitted = 0;
  await ui.render(ui.h('form', { onSubmit: event => { event.preventDefault(); submitted++; } },
    ui.h(ui.Button, { ref }, 'Action'), ui.h(ui.Button, { type: 'submit' }, 'Save'), ui.h(ui.Button, { disabled: true, type: 'submit' }, 'Unavailable')));
  assert.equal(ref.current.tagName, 'BUTTON');
  await ui.click(ref.current); assert.equal(submitted, 0);
  await ui.click(ui.get('[type="submit"]')); assert.equal(submitted, 1);
  await ui.click(ui.get(':disabled')); assert.equal(submitted, 1);
});

test('dialog traps both Tab directions, contains focus, restores trigger and removes scrim', async t => {
  const ui = await fixture(t); const outside = ui.get('#outside'); outside.focus();
  const draw = open => ui.h(ui.Dialog, { open, onClose: () => {}, 'aria-label': 'Example' },
    ui.h(ui.Button, { 'data-autofocus': true, id: 'cancel' }, 'Cancel'),
    ui.h('textarea', { id: 'text' }), ui.h(ui.Button, { disabled: true }, 'Disabled'),
    ui.h(ui.Button, { tabIndex: -1 }, 'Excluded'));
  await ui.render(draw(true));
  assert.equal(ui.w.document.activeElement.id, 'cancel');
  assert.equal(outside.inert, true);
  assert.equal(ui.w.document.body.style.overflow, 'hidden');
  await ui.key(ui.get('#cancel'), 'Tab', true); assert.equal(ui.w.document.activeElement.id, 'text');
  await ui.key(ui.get('#text'), 'Tab'); assert.equal(ui.w.document.activeElement.id, 'cancel');
  outside.focus(); assert.equal(ui.w.document.activeElement.id, 'cancel');
  await ui.render(draw(false));
  assert.equal(ui.get('.ui-dialog-scrim'), null);
  assert.notEqual(outside.inert, true);
  assert.equal(ui.w.document.body.style.overflow, '');
  assert.equal(ui.w.document.activeElement, outside);
});

test('empty and busy dialogs retain focus and ignore Escape until available', async t => {
  const ui = await fixture(t); let closed = 0;
  const draw = busy => ui.h(ui.Dialog, { open: true, busy, onClose: () => closed++, 'aria-label': 'Waiting' });
  await ui.render(draw(true));
  assert.equal(ui.w.document.activeElement, ui.get('[role="dialog"]'));
  await ui.key(ui.get('[role="dialog"]'), 'Tab');
  await ui.key(ui.get('[role="dialog"]'), 'Escape'); assert.equal(closed, 0);
  await ui.render(draw(false));
  await ui.key(ui.get('[role="dialog"]'), 'Escape'); assert.equal(closed, 1);
});

test('nested dialogs reserve Escape for the top scope and preserve inert siblings', async t => {
  const ui = await fixture(t); let outer = 0, inner = 0;
  const draw = open => ui.h(ui.Dialog, { open: true, 'aria-label': 'Outer', onClose: () => outer++ },
    ui.h(ui.Button, { id: 'outer-trigger' }, 'Nested'),
    open && ui.h(ui.Dialog, { open, 'aria-label': 'Inner', onClose: () => inner++ }, ui.h(ui.Button, { id: 'inner-cancel' }, 'Cancel')));
  await ui.render(draw(false));
  await ui.render(draw(true));
  await ui.key(ui.get('#inner-cancel'), 'Escape'); assert.equal(inner, 1); assert.equal(outer, 0);
  await ui.render(draw(false));
  assert.equal(ui.get('#outside').inert, true);
  assert.equal(ui.w.document.activeElement.id, 'outer-trigger');
});

test('tabs use a single tab stop, skip disabled tabs, and link the active panel', async t => {
  const ui = await fixture(t);
  function Example() {
    const [value, setValue] = ui.useState('one');
    return ui.h(ui.Tabs, { value, onValueChange: setValue, label: 'Modes', items: [
      { value: 'one', label: 'One' }, { value: 'disabled', label: 'Unavailable', disabled: true }, { value: 'two', label: 'Two' },
    ] }, value);
  }
  await ui.render(ui.h(Example));
  const first = ui.get('[role="tab"]'); first.focus();
  await ui.key(first, 'ArrowRight');
  assert.equal(ui.w.document.activeElement.textContent, 'Two');
  assert.equal(ui.get('[role="tabpanel"]').textContent, 'two');
  assert.equal(ui.get('[role="tabpanel"]').getAttribute('aria-labelledby'), ui.w.document.activeElement.id);
  assert.equal(ui.w.document.querySelectorAll('[role="tab"][tabindex="0"]').length, 1);
  await ui.key(ui.w.document.activeElement, 'Home'); assert.equal(ui.w.document.activeElement, first);
  await ui.key(first, 'End'); assert.equal(ui.w.document.activeElement.textContent, 'Two');
});

test('select distinguishes active from selected, keeps native change, closes portals on blur', async t => {
  const ui = await fixture(t); let changed = '';
  await ui.render(ui.h('div', null, ui.h('span', { id: 'label' }, 'Account type'),
    ui.h(ui.Select, { value: 'one', 'aria-labelledby': 'label', onChange: event => { changed = event.target.value; } },
      ui.h('option', { value: 'one' }, 'One'), ui.h('option', { value: 'skip', disabled: true }, 'Disabled'), ui.h('option', { value: 'two' }, 'Two'))));
  await ui.click(ui.get('[role="combobox"]'));
  assert.equal(ui.get('[role="listbox"]').getAttribute('aria-labelledby'), 'label');
  await ui.key(ui.get('[role="listbox"]'), 'ArrowDown');
  assert.equal(ui.get('[data-highlighted="true"]').textContent, 'Two');
  assert.equal(ui.get('[aria-selected="true"]').textContent.trim(), 'One✓');
  await ui.key(ui.get('[role="listbox"]'), 'Enter'); assert.equal(changed, 'two');
  assert.equal(ui.get('[role="listbox"]'), null);
  await ui.click(ui.get('[role="combobox"]'));
  await ui.act(() => ui.w.dispatchEvent(new ui.w.Event('blur')));
  assert.equal(ui.get('[role="listbox"]'), null);
});

test('dialog permits its select portal and Escape dismisses the listbox first', async t => {
  const ui = await fixture(t); let closed = 0;
  await ui.render(ui.h(ui.Dialog, { open: true, onClose: () => closed++, 'aria-label': 'Choose' },
    ui.h(ui.Select, { 'aria-label': 'Choice', defaultValue: 'one' }, ui.h('option', { value: 'one' }, 'One'))));
  await ui.click(ui.get('[role="combobox"]'));
  assert.equal(ui.w.document.activeElement, ui.get('[role="listbox"]'));
  await ui.key(ui.get('[role="listbox"]'), 'Escape');
  assert.equal(closed, 0); assert.equal(ui.get('[role="listbox"]'), null);
  await ui.key(ui.get('[role="combobox"]'), 'Escape'); assert.equal(closed, 1);
});

test('modal unmount preserves pre-existing inert state and releases nested isolation', async t => {
  const ui = await fixture(t); const outside = ui.get('#outside'); outside.inert = true;
  await ui.render(ui.h(ui.Dialog, { open: true, onClose: () => {}, 'aria-label': 'Outer' },
    ui.h(ui.Dialog, { open: true, onClose: () => {}, 'aria-label': 'Inner' }, ui.h(ui.Button, null, 'Cancel'))));
  assert.equal(ui.w.document.activeElement.closest('[aria-label="Inner"]')?.getAttribute('aria-label'), 'Inner');
  await ui.render(null); assert.equal(outside.inert, true);
  outside.inert = false;
  await ui.render(ui.h(ui.Dialog, { open: true, onClose: () => {}, 'aria-label': 'Again' }, ui.h(ui.Button, null, 'Cancel')));
  await ui.render(null); assert.equal(outside.inert, false);
});
