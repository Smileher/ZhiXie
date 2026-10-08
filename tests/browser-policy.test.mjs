import assert from 'node:assert/strict'
import { readFileSync } from 'node:fs'
import { test } from 'node:test'
import { runInNewContext } from 'node:vm'

const script = readFileSync(new URL('../src-tauri/src/browser.js', import.meta.url), 'utf8')

function installPolicy(platform = 'Win32') {
  const listeners = new Map()
  let prints = 0
  class Element {
    constructor(editable = false, contentEditable = false) {
      this.editable = editable
      this.isContentEditable = contentEditable
    }
    closest() { return this.editable ? this : null }
  }
  const window = {
    print: () => { prints += 1 },
    addEventListener(type, handler, options) {
      assert.ok(options === true || options.capture, `${type} must intercept before app handlers`)
      if (!listeners.has(type)) listeners.set(type, [])
      listeners.get(type).push(handler)
      if (['wheel', 'gesturestart', 'gesturechange'].includes(type)) assert.equal(options.passive, false)
    },
  }
  runInNewContext(script, { window, Element, navigator: { platform } })
  return {
    window,
    prints: () => prints,
    dispatch(type, properties = {}) {
      const event = {
        key: '', ctrlKey: false, metaKey: false, altKey: false, shiftKey: false,
        target: new Element(), defaultPrevented: false, stopped: false,
        preventDefault() { this.defaultPrevented = true },
        stopImmediatePropagation() { this.stopped = true },
        ...properties,
      }
      for (const handler of listeners.get(type) ?? []) handler(event)
      return event
    },
    input: new Element(true),
    contentEditable: new Element(false, true),
  }
}

test('Windows browser shortcuts are blocked even inside inputs', () => {
  const policy = installPolicy()
  const shortcuts = [
    ...['F1', 'F3', 'F5', 'F6', 'F7', 'F11', 'F12', 'BrowserBack', 'BrowserForward',
      'BrowserRefresh', 'BrowserStop', 'BrowserSearch', 'BrowserFavorites', 'BrowserHome'].map((key) => ({ key })),
    ...['ArrowLeft', 'ArrowRight', 'Home'].map((key) => ({ key, altKey: true })),
    ...['ctrlKey', 'metaKey'].flatMap((modifier) => [
      ...['p', 'r', 's', 'o', 'f', 'g', 'l', 'u', '+', '-', '=', '0', '[', ']']
        .map((key) => ({ key, [modifier]: true })),
      ...['i', 'j', 'c', 'k'].flatMap((key) => [
        { key, [modifier]: true, shiftKey: true },
        { key: key.toUpperCase(), [modifier]: true, altKey: true },
      ]),
    ]),
  ]
  for (const shortcut of shortcuts) {
    const event = policy.dispatch('keydown', { ...shortcut, target: policy.input })
    assert.equal(event.defaultPrevented, true, JSON.stringify(shortcut))
    assert.equal(event.stopped, true, JSON.stringify(shortcut))
  }
})

test('macOS browser commands are blocked while native text editing combinations are preserved', () => {
  const policy = installPolicy('MacIntel')
  const shortcuts = [
    ...['p', 'r', 's', 'o', 'f', 'g', 'l', 'u', '+', '-', '=', '0', '[', ']']
      .map((key) => ({ key, metaKey: true })),
    ...['i', 'j', 'c', 'k'].flatMap((key) => [
      { key, metaKey: true, altKey: true },
      { key: key.toUpperCase(), metaKey: true, shiftKey: true },
    ]),
    { key: 'F12' },
  ]
  for (const shortcut of shortcuts) {
    assert.equal(policy.dispatch('keydown', { ...shortcut, target: policy.input }).defaultPrevented, true)
  }
  for (const target of [policy.input, policy.contentEditable]) {
    for (const key of ['ArrowLeft', 'ArrowRight']) {
      for (const modifier of ['altKey', 'metaKey']) {
        assert.equal(policy.dispatch('keydown', { key, [modifier]: true, target }).defaultPrevented, false)
        assert.equal(policy.dispatch('keydown', { key, [modifier]: true }).defaultPrevented, true)
      }
    }
    for (const key of ['p', 'r', 's', 'o', 'f', 'g', 'l', 'u', 'a', 'e', 'k']) {
      assert.equal(policy.dispatch('keydown', { key, ctrlKey: true, target }).defaultPrevented, false)
    }
  }
})

test('text editing, keyboard navigation and desktop window shortcuts keep working', () => {
  const policy = installPolicy()
  const shortcuts = [
    ...['Escape', 'Tab', 'Enter', ' ', 'ArrowLeft', 'ArrowRight', 'Home', 'End', 'PageUp', 'PageDown', 'p', 'r']
      .map((key) => ({ key })),
    { key: 'F4', altKey: true },
    ...['ctrlKey', 'metaKey'].flatMap((modifier) => ['a', 'c', 'x', 'v', 'z', 'y', 'w', 'q']
      .map((key) => ({ key, [modifier]: true }))),
    { key: 'Backspace', target: policy.input },
    { key: 'Backspace', target: policy.contentEditable },
  ]
  for (const shortcut of shortcuts) {
    const event = policy.dispatch('keydown', shortcut)
    assert.equal(event.defaultPrevented, false, JSON.stringify(shortcut))
    assert.equal(event.stopped, false, JSON.stringify(shortcut))
  }
  assert.equal(policy.dispatch('keydown', { key: 'Backspace' }).defaultPrevented, true)
})

test('context menus, zoom gestures, auxiliary mouse actions, drops and printing are disabled', () => {
  const policy = installPolicy()
  for (const type of ['contextmenu', 'gesturestart', 'gesturechange', 'dragstart', 'dragover', 'drop']) {
    assert.equal(policy.dispatch(type).defaultPrevented, true, type)
  }
  for (const modifier of ['ctrlKey', 'metaKey']) {
    assert.equal(policy.dispatch('wheel', { [modifier]: true }).defaultPrevented, true)
  }
  assert.equal(policy.dispatch('wheel').defaultPrevented, false)
  for (const type of ['mousedown', 'mouseup', 'auxclick']) {
    for (const button of [1, 3, 4]) assert.equal(policy.dispatch(type, { button }).defaultPrevented, true)
    for (const button of [0, 2]) assert.equal(policy.dispatch(type, { button }).defaultPrevented, false)
  }
  policy.window.print()
  assert.equal(policy.prints(), 0)
})

test('editable targets keep pointer and keyboard context menus', () => {
  const policy = installPolicy()
  for (const target of [policy.input, policy.contentEditable]) {
    assert.equal(policy.dispatch('contextmenu', { target }).defaultPrevented, false)
    assert.equal(policy.dispatch('keydown', { target, key: 'ContextMenu' }).defaultPrevented, false)
    assert.equal(policy.dispatch('keydown', { target, key: 'F10', shiftKey: true }).defaultPrevented, false)
  }
})
