import assert from 'node:assert/strict'
import { readFileSync } from 'node:fs'
import { createRequire } from 'node:module'
import { randomUUID } from 'node:crypto'
import { test } from 'node:test'
import { runInNewContext } from 'node:vm'
import ts from 'typescript'
import * as vue from 'vue'
import { renderToString } from 'vue/server-renderer'
import { defaultData, defaultReminders } from '../src/types.ts'
import { translate } from '../src/i18n.ts'

const require = createRequire(import.meta.url)
const vueRequire = createRequire(require.resolve('vue'))
const { compileScript, compileTemplate, parse } = vueRequire('@vue/compiler-sfc')
// Run the component's actual handlers and template with only native bridges stubbed.
const { descriptor } = parse(readFileSync(new URL('../src/App.vue', import.meta.url), 'utf8'))
const script = compileScript(descriptor, { id: 'rest-timer-test' })
const template = compileTemplate({
  id: 'rest-timer-test',
  source: descriptor.template.content,
  filename: 'App.vue',
  compilerOptions: { bindingMetadata: script.bindings },
})
assert.deepEqual(template.errors, [])

function compile(source) {
  return ts.transpileModule(source, {
    compilerOptions: { module: ts.ModuleKind.CommonJS, target: ts.ScriptTarget.ES2022 },
  }).outputText
}

const scriptCode = compile(script.content)
const templateCode = compile(template.code)
const noop = () => {}
const resting = { isResting: true, nextTriggerAt: null }

test('persistence failures remain visible until a successful retry or recovery event', async () => {
  const app = await mountApp()
  await app.emit('persistence-failed', 'disk full')
  assert.equal(app.state.persistenceError.value, 'disk full')
  app.setInvoke('load_data', async () => { throw new Error('still full') })
  await app.state.retryPersistence()
  assert.match(app.state.persistenceError.value, /still full/)
  await app.emit('persistence-failed', null)
  assert.equal(app.state.persistenceError.value, '')
})

test('persistence retry synchronizes the draft before the next queued setting', async () => {
  const app = await mountApp({ nativeErrors: { persistenceError: 'disk full' } })
  assert.equal(app.state.persistenceError.value, 'disk full')
  let resolveLoad
  app.setInvoke('load_data', () => new Promise((resolve) => { resolveLoad = resolve }))
  const retry = app.state.retryPersistence()
  await new Promise(setImmediate)
  const edit = app.state.updateSetting('theme', 'light')
  const loaded = defaultData()
  loaded.settings.restMessage = 'recovered text'
  resolveLoad(loaded)
  await Promise.all([retry, edit])
  assert.equal(app.state.persistenceError.value, '')
  assert.equal(app.state.restMessageDraft.value, 'recovered text')
  assert.equal(app.state.data.value.settings.theme, 'light')
})

test('scheduler failures survive reload and cannot be cleared by a persistence retry', async () => {
  const app = await mountApp({ nativeErrors: { schedulerError: 'state lock poisoned' } })
  assert.equal(app.state.schedulerError.value, 'state lock poisoned')
  const reads = app.calls.filter((command) => command === 'load_data').length
  await app.emit('persistence-failed', 'disk full')
  await app.emit('scheduler-failed', 'scheduler stopped')
  await app.state.retryPersistence()
  assert.equal(app.calls.filter((command) => command === 'load_data').length, reads)
  assert.equal(app.state.schedulerError.value, 'scheduler stopped')
  await app.emit('persistence-failed', null)
  assert.equal(app.state.schedulerError.value, 'scheduler stopped')
  const html = await app.render()
  assert.ok(html.includes(translate('zh-CN', 'status.schedulerFailed', { error: 'scheduler stopped' })))
})

test('a late native status read cannot hide a newer persistence failure', async () => {
  const app = await mountApp()
  let resolveErrors
  app.setInvoke('get_native_errors', () => new Promise((resolve) => { resolveErrors = resolve }))
  const read = app.state.refreshNativeErrors()
  await app.emit('persistence-failed', 'new failure')
  resolveErrors({ persistenceError: null, schedulerError: null })
  await read
  assert.equal(app.state.persistenceError.value, 'new failure')
})

async function mountApp({ enabled = true, status = resting, nativeErrors = { autostartError: null, notificationError: null }, appearanceDocument } = {}) {
  const settingsData = defaultData()
  settingsData.settings.restEnabled = enabled
  settingsData.settings.restIntervalMinutes = 1
  const listeners = new Map()
  const windowListeners = new Map()
  const calls = []
  let mounted
  let focused
  let themeChanged
  let hideCount = 0
  let readStatus = async () => status
  let saveData = async (value) => structuredClone(value)
  let confirmReset = async () => false
  const invokeHandlers = new Map()
  const nativeHandlers = new Map()
  const errors = []
  function emitTo(target, name, payload) {
    const listener = listeners.get(name)
    if (listener && (listener.target === null || listener.target === target)) {
      return listener.callback({ payload })
    }
  }
  const modules = {
    vue: { ...vue, onMounted: (callback) => { mounted = callback }, onUnmounted: noop },
    '@tauri-apps/api/app': { getVersion: async () => '0.9.0', setTheme: async (theme) => nativeHandlers.get('setTheme')?.(theme) },
    '@tauri-apps/api/core': {
      invoke: async (command, args) => {
        calls.push(command)
        if (invokeHandlers.has(command)) return invokeHandlers.get(command)(args)
        if (command === 'load_data') return structuredClone(settingsData)
        if (command === 'get_native_errors') return nativeErrors
        if (command === 'get_autostart_status') {
          if (nativeErrors.autostartError) throw nativeErrors.autostartError
          return { enabled: false, conflict: false, reason: null }
        }
        if (command === 'set_autostart') return { enabled: args.enabled, conflict: false, reason: null }
        if (command === 'save_data') return saveData(JSON.parse(JSON.stringify(args.data)), args)
        if (command === 'read_popup_image') return null
        if (command === 'is_popup_image_default') return false
        if (command === 'reset_popup_image') return 'data:image/png;base64,default'
        if (command === 'clear_popup_image') return
        if (command === 'hide_idle_window') { hideCount += 1; return }
        if (command === 'get_rest_timer_status') {
          assert.ok(listeners.has('rest-timer-updated'), 'subscribe before reading initial status')
          return readStatus()
        }
        throw new Error(`Unexpected command: ${command}`)
      },
    },
    '@tauri-apps/api/event': {
      listen: async (name, callback) => { listeners.set(name, { target: null, callback }); return noop },
    },
    '@tauri-apps/api/window': {
      getCurrentWindow: () => ({
        hide: async () => { hideCount += 1 },
        onFocusChanged: async (callback) => { focused = callback; return noop },
        listen: async (name, callback) => { listeners.set(name, { target: 'main', callback }); return noop },
      }),
    },
    '@tauri-apps/plugin-dialog': {
      ask: () => confirmReset(), confirm: () => confirmReset(),
      open: (options) => nativeHandlers.get('open')?.(options), save: noop,
    },
    '@tauri-apps/plugin-notification': { sendNotification: noop, onAction: async () => ({ unregister: async () => {} }), isPermissionGranted: async () => true, requestPermission: async () => 'granted' },
    '@lucide/vue': new Proxy({}, { get: () => ({ render: noop }) }),
    './components/ReminderPopup.vue': { default: { render: noop } },
    './components/AppSidebar.vue': { default: { render: noop } },
    './components/EventsView.vue': { default: { render: noop } },
    './components/RestView.vue': { default: { render: noop } },
    './components/SettingsView.vue': { default: { render: noop } },
    './components/AboutView.vue': { default: { render: noop } },
    './components/AgentView.vue': { default: { render: noop } },
    './assets/logo.svg': { default: 'logo.svg' },
    './i18n': { translate },
    './types': { defaultData, defaultReminders },
    './error': { logError: (context, error) => { errors.push({ context, error }) } },
  }
  function load(code) {
    const exports = {}
    runInNewContext(code, {
      exports,
      require: (name) => {
        assert.ok(Object.hasOwn(modules, name), `Unexpected module: ${name}`)
        return modules[name]
      },
      window: {
        innerWidth: 780, innerHeight: 540,
        matchMedia: () => ({ matches: false, addEventListener: (_name, callback) => { themeChanged = callback }, removeEventListener: noop }),
        location: { hash: '' },
        setInterval: () => 1,
        clearInterval: noop,
        setTimeout: () => 1,
        clearTimeout: noop,
        addEventListener: (name, callback) => { windowListeners.set(name, callback) },
        removeEventListener: (name) => { windowListeners.delete(name) },
      },
      localStorage: { getItem: () => null, setItem: noop },
      document: appearanceDocument ?? { visibilityState: 'visible', hasFocus: () => true },
      console,
      crypto: { randomUUID },
    })
    return exports
  }
  const component = load(scriptCode).default
  const state = component.setup({}, { expose: noop })
  const render = load(templateCode).render
  await mounted()
  return {
    state, calls, errors,
    setReadStatus: (callback) => { readStatus = callback },
    setSaveData: (callback) => { saveData = callback },
    setConfirm: (callback) => { confirmReset = callback },
    setInvoke: (command, callback) => { invokeHandlers.set(command, callback) },
    setNative: (name, callback) => { nativeHandlers.set(name, callback) },
    emit: (name, payload) => emitTo('main', name, payload),
    emitTo,
    focus: (isFocused) => focused({ payload: isFocused }),
    systemThemeChange: (matches) => themeChanged({ matches }),
    keydown: (event) => windowListeners.get('keydown')?.(event),
    getHideCount: () => hideCount,
    render: () => renderToString(vue.createSSRApp({
      setup: () => state, render, components: { BellRing: { render: noop } },
    })),
  }
}

test('Escape hides the main window to the tray', async () => {
  const app = await mountApp()
  app.keydown({ key: 'Enter', repeat: false })
  app.keydown({ key: 'Escape', repeat: true })
  assert.equal(app.getHideCount(), 0)
  app.keydown({ key: 'Escape', repeat: false })
  await new Promise(setImmediate)
  assert.equal(app.getHideCount(), 1)
})

test('serial saves keep newer edits and external popup mode changes', async () => {
  const app = await mountApp()
  const writes = []
  app.setSaveData((data, args) => new Promise((resolve) => writes.push({ data, args, resolve })))
  const first = app.state.updateSetting('popupOverlayOpacity', 10)
  await new Promise(setImmediate)
  const second = app.state.updateSetting('popupOverlayOpacity', 90)
  assert.equal(writes.length, 1)
  await app.emit('popup-fullscreen-updated', false)
  assert.equal(app.state.data.value.settings.popupOverlayOpacity, 10)
  writes[0].resolve(writes[0].data)
  await new Promise(setImmediate)
  assert.equal(writes.length, 2)
  assert.equal(writes[1].data.settings.popupOverlayOpacity, 90)
  assert.equal(writes[1].data.settings.popupFullscreen, false)
  assert.equal(writes[1].args.popupFullscreen, null)
  writes[1].resolve(writes[1].data)
  await Promise.all([first, second])
  assert.equal(app.state.data.value.settings.popupOverlayOpacity, 90)
  assert.equal(app.state.data.value.settings.popupFullscreen, false)
})

test('explicit main-window mode changes are included in the save', async () => {
  const app = await mountApp()
  let savedArgs
  app.setSaveData(async (data, args) => { savedArgs = args; return data })
  await app.state.updateSetting('popupFullscreen', false)
  assert.equal(savedArgs.popupFullscreen, false)
})

test('failed setting rolls back before the next queued field or same-field edit', async () => {
  for (const nextKey of ['theme', 'popupOverlayOpacity']) {
    const app = await mountApp()
    const original = app.state.data.value.settings.popupOverlayOpacity
    let rejectFirst
    let count = 0
    app.setSaveData(async (data) => {
      if (++count === 1) return new Promise((_, reject) => { rejectFirst = reject })
      assert.equal(data.settings.popupOverlayOpacity, nextKey === 'theme' ? original : 90)
      return data
    })
    const first = app.state.updateSetting('popupOverlayOpacity', 10)
    await new Promise(setImmediate)
    const second = app.state.updateSetting(nextKey, nextKey === 'theme' ? 'light' : 90)
    rejectFirst(new Error('disk full'))
    assert.equal(await first, false)
    assert.equal(await second, true)
    assert.equal(app.state.data.value.settings[nextKey], nextKey === 'theme' ? 'light' : 90)
  }
})

test('normalized rest messages are returned to the editable draft', async () => {
  const app = await mountApp()
  app.setSaveData(async (data) => ({ ...data, settings: { ...data.settings, restMessage: 'normalized' } }))
  await app.state.saveRestMessage('')
  assert.equal(app.state.restMessageDraft.value, 'normalized')
})

test('background image import failures preserve the existing preview', async () => {
  const app = await mountApp()
  const previous = 'data:image/png;base64,existing'
  app.state.popupBackgroundPreview.value = previous
  app.setNative('open', async () => 'selected.png')
  app.setInvoke('import_popup_image', async () => { throw new Error('Image must not exceed 10 MiB') })
  await app.state.pickPopupImage()
  assert.equal(app.state.popupBackgroundPreview.value, previous)
  assert.match(app.state.actionMessage.value, /10 MiB/)
})

test('import and export cancellations are silent and never pass a path from the frontend', async () => {
  const app = await mountApp()
  const before = JSON.stringify(app.state.data.value)
  app.setInvoke('import_data', async (args) => { assert.equal(args, undefined); return null })
  app.setInvoke('export_data', async (args) => { assert.equal(args, undefined); return false })
  await app.state.importData()
  await app.state.exportData()
  assert.equal(JSON.stringify(app.state.data.value), before)
  assert.equal(app.state.actionMessage.value, '')
  app.setInvoke('import_data', async () => { throw new Error('invalid json') })
  await app.state.importData()
  assert.equal(JSON.stringify(app.state.data.value), before)
  assert.equal(app.state.actionMessage.value, translate('zh-CN', 'status.importFailed'))
  app.setInvoke('export_data', async () => { throw new Error('disk full') })
  await app.state.exportData()
  assert.equal(app.state.actionMessage.value, translate('zh-CN', 'status.exportFailed'))
})

test('successful import synchronizes reminders and the draft before export', async () => {
  const app = await mountApp()
  const imported = defaultData()
  imported.settings.restMessage = 'imported text'
  imported.reminders = []
  app.setInvoke('import_data', async () => imported)
  app.setInvoke('export_data', async () => true)
  await app.state.importData()
  assert.equal(app.state.data.value.reminders.length, 0)
  assert.equal(app.state.restMessageDraft.value, 'imported text')
  assert.equal(app.state.actionMessage.value, translate('zh-CN', 'status.imported'))
  await app.state.exportData()
  assert.equal(app.state.actionMessage.value, translate('zh-CN', 'status.exported'))
})

test('reminder CRUD validates, preserves disabled state and rolls back failures', async () => {
  const app = await mountApp()
  app.state.data.value.reminders = []
  app.state.patchForm({ title: '', type: 'daily', time: '09:00' })
  await app.state.saveReminder()
  assert.equal(app.state.data.value.reminders.length, 0)
  app.state.patchForm({ title: 'event', type: 'weekly', weekdays: [] })
  await app.state.saveReminder()
  assert.equal(app.state.data.value.reminders.length, 0)
  app.state.patchForm({ type: 'daily' })
  await app.state.saveReminder()
  assert.equal(app.state.data.value.reminders.length, 1)
  const id = app.state.data.value.reminders[0].id
  await app.state.toggleReminder(app.state.data.value.reminders[0])
  assert.equal(app.state.data.value.reminders[0].enabled, false)
  app.state.editReminder(app.state.data.value.reminders[0])
  app.state.patchForm({ title: 'edited' })
  await app.state.saveReminder()
  assert.equal(app.state.data.value.reminders[0].id, id)
  assert.equal(app.state.data.value.reminders[0].title, 'edited')
  assert.equal(app.state.data.value.reminders[0].enabled, false)
  app.setSaveData(async () => { throw new Error('disk full') })
  await app.state.toggleReminder(app.state.data.value.reminders[0])
  assert.equal(app.state.data.value.reminders[0].enabled, false)
  app.state.patchForm({ title: 'failed edit' })
  await app.state.saveReminder()
  assert.equal(app.state.data.value.reminders[0].title, 'edited')
  app.setConfirm(async () => false)
  await app.state.removeReminder(id)
  assert.equal(app.state.data.value.reminders.length, 1)
  app.setConfirm(async () => true)
  await app.state.removeReminder(id)
  assert.equal(app.state.data.value.reminders.length, 1)
  app.setSaveData(async (data) => data)
  await app.state.removeReminder(id)
  assert.equal(app.state.data.value.reminders.length, 0)
})

test('a failed reminder mutation cannot replace a later successful setting', async () => {
  const app = await mountApp()
  app.state.data.value.reminders = []
  app.state.patchForm({ title: 'event', type: 'daily', time: '09:00' })
  let rejectFirst
  let count = 0
  app.setSaveData(async (data) => ++count === 1 ? new Promise((_, reject) => { rejectFirst = reject }) : data)
  const first = app.state.saveReminder()
  await new Promise(setImmediate)
  const setting = app.state.updateSetting('theme', 'light')
  rejectFirst(new Error('disk full'))
  await Promise.all([first, setting])
  assert.equal(app.state.data.value.reminders.length, 0)
  assert.equal(app.state.data.value.settings.theme, 'light')
})

test('failed fullscreen and language edits preserve a newer native mode event', async () => {
  for (const languageEdit of [false, true]) {
    const app = await mountApp()
    app.state.data.value.settings.popupFullscreen = false
    let reject
    app.setSaveData(() => new Promise((_, fail) => { reject = fail }))
    const change = languageEdit ? app.state.updateLanguage('en') : app.state.updateSetting('popupFullscreen', true)
    await new Promise(setImmediate)
    await app.emit('popup-fullscreen-updated', true)
    reject(new Error('disk full'))
    await change
    assert.equal(app.state.data.value.settings.popupFullscreen, true)
    assert.equal(app.state.data.value.settings.language, 'zh-CN')
  }
})

test('default-image detection failures preserve loaded and imported image previews', async () => {
  const app = await mountApp()
  app.setInvoke('is_popup_image_default', async () => { throw new Error('detection unavailable') })
  app.setInvoke('read_popup_image', async () => 'data:image/png;base64,existing')
  await app.state.loadPopupImagePreview()
  assert.equal(app.state.popupBackgroundPreview.value, 'data:image/png;base64,existing')
  assert.equal(app.state.popupImageIsDefault.value, false)
  app.setNative('open', async () => 'custom.png')
  app.setInvoke('import_popup_image', async () => 'data:image/png;base64,imported')
  await app.state.pickPopupImage()
  assert.equal(app.state.popupBackgroundPreview.value, 'data:image/png;base64,imported')
  assert.equal(app.state.actionMessage.value, translate('zh-CN', 'status.imageSaved'))
})

test('reset keeps English defaults and reminders while restoring the default image', async () => {
  const app = await mountApp()
  app.state.data.value.settings = defaultData().settings
  app.state.data.value.settings.language = 'en'
  app.state.data.value.settings.restMessage = translate('en', 'rest.defaultMessage')
  app.state.data.value.reminders = [{ id: 'keep', title: 'keep reminder' }]
  app.state.popupBackgroundPreview.value = 'data:image/png;base64,image'
  assert.equal(app.state.canResetSettings.value, true)
  app.setConfirm(async () => true)
  await app.state.resetSettings()
  assert.equal(app.state.data.value.settings.language, 'en')
  assert.equal(app.state.data.value.settings.restMessage, translate('en', 'rest.defaultMessage'))
  assert.equal(app.state.data.value.reminders[0].id, 'keep')
  assert.equal(app.state.popupBackgroundPreview.value, 'data:image/png;base64,default')
  assert.equal(app.state.popupImageIsDefault.value, true)
  assert.equal(app.state.canResetSettings.value, false)
})

test('failed reset restores settings and autostart without overriding the native theme', async () => {
  const app = await mountApp()
  app.state.data.value.settings.autostart = true
  app.state.data.value.settings.theme = 'light'
  app.state.popupBackgroundPreview.value = 'data:image/png;base64,image'
  const before = JSON.stringify(app.state.data.value.settings)
  const nativeCalls = []
  app.setNative('disable', async () => nativeCalls.push('disable'))
  app.setNative('enable', async () => nativeCalls.push('enable'))
  app.setNative('setTheme', async (theme) => nativeCalls.push(theme))
  app.setSaveData(async () => { throw new Error('disk full') })
  app.setConfirm(async () => true)
  await app.state.resetSettings()
  assert.equal(JSON.stringify(app.state.data.value.settings), before)
  assert.equal(app.calls.includes('set_autostart'), false)
  assert.ok(!nativeCalls.includes(null) && !nativeCalls.includes('light'))
  assert.equal(app.calls.includes('reset_popup_image'), false)
  assert.equal(app.state.popupBackgroundPreview.value, 'data:image/png;base64,image')
  assert.equal(app.state.actionMessage.value, translate('zh-CN', 'status.saveFailed'))
})

test('image restoration failure keeps the saved defaults and reports the actual image state', async () => {
  const app = await mountApp()
  app.state.data.value.settings.theme = 'light'
  app.state.popupBackgroundPreview.value = 'data:image/png;base64,image'
  app.setInvoke('reset_popup_image', async () => { throw new Error('permission denied') })
  app.setInvoke('read_popup_image', async () => 'data:image/png;base64,image')
  app.setConfirm(async () => true)
  await app.state.resetSettings()
  assert.equal(app.state.data.value.settings.theme, 'system')
  assert.equal(app.state.popupBackgroundPreview.value, 'data:image/png;base64,image')
  assert.equal(app.state.actionMessage.value, translate('zh-CN', 'status.imageFailed'))
})

test('language changes translate untouched presets and preserve custom content', async () => {
  const app = await mountApp()
  app.state.data.value.reminders = defaultData().reminders
  app.state.data.value.reminders[1].title = '自定义下班提醒'
  await app.state.updateLanguage('en')
  assert.equal(app.state.data.value.reminders[0].title, translate('en', 'preset.lunch'))
  assert.equal(app.state.data.value.reminders[1].title, '自定义下班提醒')
  assert.equal(app.state.data.value.reminders[2].title, translate('en', 'preset.weekend'))
})

test('native startup failures display actual autostart status and retain background notification errors', async () => {
  const app = await mountApp({ nativeErrors: { autostartError: 'registration denied', notificationError: 'notifications disabled' } })
  assert.equal(app.state.data.value.settings.autostart, false)
  assert.equal(app.state.autostartError.value, translate('zh-CN', 'status.autostartFailed', { error: 'registration denied' }))
  assert.equal(app.state.notificationError.value, translate('zh-CN', 'status.notificationFailed', { error: 'notifications disabled' }))
})

test('startup is queried on launch without enabling a registration', async () => {
  const app = await mountApp()
  assert.equal(app.state.data.value.settings.autostart, false)
  assert.ok(app.calls.includes('get_autostart_status'))
  assert.equal(app.calls.includes('set_autostart'), false)
})

test('external startup disable updates the switch without re-enabling it', async () => {
  const app = await mountApp()
  app.state.data.value.settings.autostart = true
  app.focus(true)
  await new Promise(setImmediate)
  assert.equal(app.state.data.value.settings.autostart, false)
  assert.equal(app.calls.includes('set_autostart'), false)
})

test('startup conflict retains the disabled switch and displays a localized explanation', async () => {
  const app = await mountApp()
  app.setInvoke('set_autostart', async () => { throw 'conflict' })
  app.setInvoke('get_autostart_status', async () => ({ enabled: false, conflict: true, reason: null }))
  await app.state.updateAutostart(true)
  assert.equal(app.state.data.value.settings.autostart, false)
  assert.equal(app.state.autostartNotice.value, translate('zh-CN', 'settings.autostartConflict'))
  assert.equal(app.state.autostartError.value, translate('zh-CN', 'settings.autostartConflict'))
})

test('a startup query failure preserves the observed switch and reports the error', async () => {
  const app = await mountApp()
  app.state.data.value.settings.autostart = true
  app.setInvoke('get_autostart_status', async () => { throw 'query denied' })
  await app.state.refreshAutostart()
  assert.equal(app.state.data.value.settings.autostart, true)
  assert.equal(app.state.autostartError.value, translate('zh-CN', 'status.autostartFailed', { error: 'query denied' }))
})

test('only an explicit startup toggle changes native registration and does not run a separate settings save', async () => {
  const app = await mountApp()
  const before = app.calls.length
  await app.state.updateAutostart(true)
  assert.equal(app.state.data.value.settings.autostart, true)
  assert.deepEqual(app.calls.slice(before), ['set_autostart'])
  await app.state.updateAutostart(false)
  assert.equal(app.state.data.value.settings.autostart, false)
})

test('reset saves the system theme and preserves autostart without changing native startup', async () => {
  const app = await mountApp()
  const nativeCalls = []
  app.setNative('enable', async () => nativeCalls.push('enable'))
  app.setNative('disable', async () => nativeCalls.push('disable'))
  app.setNative('setTheme', async (theme) => nativeCalls.push(theme))
  app.setConfirm(async () => true)
  app.state.data.value.settings.autostart = true
  await app.state.resetSettings()
  assert.deepEqual(nativeCalls, [])
  assert.equal(app.state.data.value.settings.autostart, true)
  assert.equal(app.state.data.value.settings.theme, 'system')
})

test('repeated theme changes persist without writing the competing app theme', async () => {
  const app = await mountApp()
  const savedThemes = []
  const nativeThemes = []
  app.setNative('setTheme', async (theme) => nativeThemes.push(theme))
  app.setSaveData(async (data) => {
    savedThemes.push(data.settings.theme)
    return structuredClone(data)
  })
  for (const theme of ['dark', 'light', 'dark', 'system', 'light', 'dark']) {
    assert.equal(await app.state.updateSetting('theme', theme), true)
    assert.equal(app.state.data.value.settings.theme, theme)
  }
  assert.deepEqual(savedThemes, ['dark', 'light', 'dark', 'system', 'light', 'dark'])
  assert.deepEqual(nativeThemes, [])
})

test('appearance snapshots preserve save rollback and fall back if the transition API throws', async () => {
  const classes = new Set()
  const animations = []
  const appearanceDocument = {
    querySelector: () => null,
    documentElement: {
      classList: { add: value => classes.add(value), remove: value => classes.delete(value) },
      animate: (frames, options) => animations.push({ frames, options }),
    },
    startViewTransition: callback => {
      const ready = Promise.resolve().then(callback)
      return { ready, finished: ready }
    },
  }
  const app = await mountApp({ appearanceDocument })
  const event = { currentTarget: { getBoundingClientRect: () => ({ left: 100, top: 200, width: 40, height: 30 }) }, detail: 0 }
  assert.equal(await app.state.updateSetting('theme', 'dark', event), true)
  assert.equal(animations[0].frames.clipPath[0], 'circle(0px at 120px 215px)')
  assert.equal(animations[0].options.pseudoElement, '::view-transition-new(root)')
  assert.equal(classes.size, 0)
  assert.equal(await app.state.updateSetting('accentColor', 'rose', event), true)
  assert.deepEqual(Array.from(animations[1].frames.opacity), [0, 1])
  assert.equal(animations[1].options.duration, 480)
  await app.state.updateSetting('theme', 'system')
  const saveCount = app.calls.filter(command => command === 'save_data').length
  app.systemThemeChange(true)
  await new Promise(setImmediate)
  assert.equal(app.state.displayedTheme.value, 'dark')
  assert.equal(app.state.data.value.settings.theme, 'system')
  assert.equal(animations.at(-1).frames.clipPath[0], 'circle(0px at 390px 270px)')
  assert.equal(app.calls.filter(command => command === 'save_data').length, saveCount)
  await app.state.updateSetting('theme', 'dark')
  const animationCount = animations.length
  app.systemThemeChange(false)
  await new Promise(setImmediate)
  assert.equal(app.state.displayedTheme.value, 'dark')
  assert.equal(animations.length, animationCount)
  app.setSaveData(async () => { throw new Error('disk full') })
  assert.equal(await app.state.updateSetting('theme', 'light', event), false)
  assert.equal(app.state.data.value.settings.theme, 'dark')
  assert.equal(classes.size, 0)
  app.setSaveData(async data => data)
  appearanceDocument.startViewTransition = () => { throw new Error('snapshot unavailable') }
  assert.equal(await app.state.updateSetting('theme', 'light', event), true)
  assert.equal(app.state.data.value.settings.theme, 'light')
  assert.equal(classes.size, 0)
})

test('failed theme save restores the previous preference without changing the app theme', async () => {
  const app = await mountApp()
  app.state.data.value.settings.theme = 'dark'
  const nativeThemes = []
  app.setNative('setTheme', async (theme) => nativeThemes.push(theme))
  app.setSaveData(async () => { throw new Error('disk full') })
  assert.equal(await app.state.updateSetting('theme', 'light'), false)
  assert.equal(app.state.data.value.settings.theme, 'dark')
  assert.deepEqual(nativeThemes, [])
})

test('test break notifications use backend rest state even with reminders disabled', async () => {
  const app = await mountApp({ enabled: false, status: { isResting: false, nextTriggerAt: null } })
  app.setReadStatus(async () => resting)
  await app.emit('reminder-triggered', { id: '__test_rest__', isRest: true, isTest: true })
  assert.equal(app.state.restIsActive.value, true)
  assert.equal(app.state.nextRestTrigger.value, null)
  assert.equal(app.state.restProgress.value, 0)
  // 文案已下沉到 RestView，这里断言状态与文案来源保持一致，避免依赖界面文本。
  assert.equal(app.state.restStatusText.value, '正在休息中')
})

test('a slow status read cannot restore the countdown after a resting event', async () => {
  const app = await mountApp()
  let resolveStatus
  app.setReadStatus(() => new Promise((resolve) => { resolveStatus = resolve }))
  const refresh = app.state.refreshTimers()
  await app.emit('rest-timer-updated', resting)
  resolveStatus({ isResting: false, nextTriggerAt: '2026-09-23T16:00:00+08:00' })
  await refresh
  assert.equal(app.state.restIsActive.value, true)
  assert.equal(app.state.nextRestTrigger.value, null)
})

test('temporary status failures keep the active break and do not restore a countdown', async () => {
  const app = await mountApp()
  app.setReadStatus(async () => { throw new Error('bridge unavailable') })
  await app.state.refreshTimers()
  assert.equal(app.state.restIsActive.value, true)
  assert.equal(app.state.nextRestTrigger.value, null)
})

test('snooze and completion events apply their backend deadlines without stale reads replacing them', async () => {
  const app = await mountApp()
  const snoozedUntil = new Date(Date.now() + 4 * 60 * 60 * 1000).toISOString()
  let resolveStatus
  app.setReadStatus(() => new Promise((resolve) => { resolveStatus = resolve }))
  const refresh = app.state.refreshTimers()
  await app.emit('rest-timer-updated', { isResting: false, nextTriggerAt: snoozedUntil })
  resolveStatus(resting)
  await refresh
  assert.equal(app.state.restIsActive.value, false)
  assert.equal(app.state.nextRestTrigger.value, snoozedUntil)
  assert.equal(app.state.data.value.settings.restIntervalMinutes, 1)
  await app.emit('rest-timer-updated', resting)
  assert.equal(app.state.restIsActive.value, true)
  const completedUntil = new Date(Date.now() + 60 * 1000).toISOString()
  await app.emit('rest-timer-updated', { isResting: false, nextTriggerAt: completedUntil })
  assert.equal(app.state.restIsActive.value, false)
  assert.equal(app.state.nextRestTrigger.value, completedUntil)
})

test('newer refreshes take precedence over older reads', async () => {
  const app = await mountApp()
  let resolveOld
  app.setReadStatus(() => new Promise((resolve) => { resolveOld = resolve }))
  const oldRefresh = app.state.refreshTimers()
  app.setReadStatus(async () => resting)
  await app.state.refreshTimers()
  resolveOld({ isResting: false, nextTriggerAt: '2026-09-23T16:00:00+08:00' })
  await oldRefresh
  assert.equal(app.state.restIsActive.value, true)
  assert.equal(app.state.nextRestTrigger.value, null)
})

test('window focus changes preserve the active backend break', async () => {
  const app = await mountApp()
  for (const focused of [false, true]) {
    const before = app.calls.length
    app.focus(focused)
    await new Promise(setImmediate)
    assert.deepEqual(app.calls.slice(before), ['get_rest_timer_status', 'get_native_errors', 'get_autostart_status'])
    assert.equal(app.state.restIsActive.value, true)
    assert.equal(app.state.nextRestTrigger.value, null)
  }
})

test('a trigger follows backend countdown state for system notifications', async () => {
  const app = await mountApp()
  const nextTriggerAt = new Date(Date.now() + 60 * 1000).toISOString()
  app.setReadStatus(async () => ({ isResting: false, nextTriggerAt }))
  await app.emit('reminder-triggered', { id: '__rest__', isRest: true, isTest: false })
  assert.equal(app.state.restIsActive.value, false)
  assert.equal(app.state.nextRestTrigger.value, nextTriggerAt)
})

test('the main window ignores popup-targeted events and handles each scheduled reminder once', async () => {
  const nextTriggerAt = new Date(Date.now() + 60 * 1000).toISOString()
  const app = await mountApp({ status: { isResting: false, nextTriggerAt } })
  const callCount = app.calls.length
  const event = { id: '__rest__', isRest: true, isTest: false }
  app.setReadStatus(async () => resting)
  await app.emitTo('reminder', 'reminder-triggered', event)
  await app.emitTo('reminder', 'rest-timer-updated', resting)
  assert.equal(app.calls.length, callCount)
  assert.equal(app.state.restIsActive.value, false)
  assert.equal(app.state.nextRestTrigger.value, nextTriggerAt)

  await app.emitTo('main', 'reminder-triggered', event)
  assert.equal(app.calls.slice(callCount).filter((command) => command === 'load_data').length, 1)
  assert.equal(app.state.restIsActive.value, true)
  assert.equal(app.state.nextRestTrigger.value, null)
})
