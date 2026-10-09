import assert from 'node:assert/strict'
import { readFileSync } from 'node:fs'
import { createRequire } from 'node:module'
import { test } from 'node:test'
import { runInNewContext } from 'node:vm'
import ts from 'typescript'
import * as vue from 'vue'
import { defaultData } from '../src/types.ts'
import { translate } from '../src/i18n.ts'

const require = createRequire(import.meta.url)
const vueRequire = createRequire(require.resolve('vue'))
const { compileScript, parse } = vueRequire('@vue/compiler-sfc')
const { descriptor } = parse(readFileSync(new URL('../src/components/ReminderPopup.vue', import.meta.url), 'utf8'))
const script = compileScript(descriptor, { id: 'reminder-popup-test' })
const scriptCode = ts.transpileModule(script.content, {
  compilerOptions: { module: ts.ModuleKind.CommonJS, target: ts.ScriptTarget.ES2022 },
}).outputText
const noop = () => {}
const settle = () => new Promise(setImmediate)
const restEvent = { sessionId: 1, id: '__rest__', title: '休息时间到了', type: 'interval', isRest: true, isTest: false }
const powerEvent = { sessionId: 2, id: 'lock-plan', title: '锁定电脑', type: 'daily', isRest: false, powerAction: 'lock', isTest: false }

test('failed power actions retain the popup and can be retried', async () => {
  const popup = await mountPopup()
  await popup.state.handleTrigger(powerEvent)
  popup.setInvoke('execute_power_action', async () => { throw new Error('permission denied') })
  await popup.state.executePowerAction()
  assert.match(popup.state.powerError.value, /permission denied/)
  assert.equal(popup.state.current.value.sessionId, powerEvent.sessionId)
  popup.setInvoke('execute_power_action', async () => true)
  await popup.state.executePowerAction()
  assert.equal(popup.calls.filter((name) => name === 'execute_power_action').length, 2)
})

test('new settings enable fullscreen reminders by default', () => {
  assert.equal(defaultData().settings.popupFullscreen, true)
})

test('rest elapsed display carries minutes into hours in both languages', async () => {
  const popup = await mountPopup({ active: restEvent })
  assert.equal(popup.state.restElapsed.value, '已休息：0 分 0 秒')
  await popup.advance(3_599_000)
  assert.equal(popup.state.restElapsed.value, '已休息：59 分 59 秒')
  await popup.advance(1_000)
  assert.equal(popup.state.restElapsed.value, '已休息：1 小时 0 分 0 秒')
  await popup.advance(3_661_000)
  assert.equal(popup.state.restElapsed.value, '已休息：2 小时 1 分 1 秒')
  popup.state.settings.value.language = 'en'
  assert.equal(popup.state.restElapsed.value, 'Resting: 2h 1m 1s')
  await popup.state.handleTrigger({ ...restEvent, sessionId: 2 })
  popup.state.settings.value.language = 'en'
  await popup.advance(61_000)
  assert.equal(popup.state.restElapsed.value, 'Resting: 1m 1s')
})

test('mode changes preserve the rest start and automatic power deadline', async () => {
  const popup = await mountPopup()
  const startedAt = popup.now()
  await popup.state.handleTrigger({ ...restEvent, restStartedAtMs: startedAt })
  await popup.advance(20_000)
  await popup.state.handleTrigger({ ...restEvent, sessionId: 2, restStartedAtMs: startedAt })
  assert.equal(popup.state.restElapsedSeconds.value, 20)

  const power = await mountPopup()
  const deadline = power.now() + 60_000
  await power.state.handleTrigger({ ...powerEvent, powerDeadlineMs: deadline })
  await power.advance(40_000)
  await power.state.handleTrigger({ ...powerEvent, sessionId: 3, powerDeadlineMs: deadline })
  assert.equal(power.state.powerCountdown.value, 20)
  await power.advance(20_000)
  assert.equal(power.calls.filter((name) => name === 'execute_power_action').length, 1)
})

test('failed mode preparation cannot restore the closed reminder or its timer', async () => {
  const popup = await mountPopup()
  await popup.state.handleTrigger(restEvent)
  popup.setInvoke('toggle_popup_fullscreen', async () => {
    await popup.emitTo('reminder', 'reminder-closed', restEvent.sessionId)
    await popup.emitTo('reminder', 'reminder-closed', restEvent.sessionId + 1)
    throw new Error('window preparation failed')
  })
  await popup.state.toggleFullscreenMode()
  assert.equal(popup.state.current.value, null)
  assert.equal(popup.intervals.size, 0)
  await popup.state.handleTrigger(restEvent)
  assert.equal(popup.state.current.value, null)
  await popup.state.handleTrigger({ ...restEvent, sessionId: 3 })
  assert.equal(popup.state.current.value.sessionId, 3)
})

test('image changes refresh cached popups without rereading images for settings', async () => {
  const popup = await mountPopup()
  const reads = popup.calls.filter((name) => name === 'read_popup_image').length
  await popup.emitTo('reminder', 'settings-updated', defaultData().settings)
  assert.equal(popup.calls.filter((name) => name === 'read_popup_image').length, reads)
  popup.setInvoke('read_popup_image', async () => 'data:image/jpeg;base64,new')
  await popup.emitTo('reminder', 'popup-image-updated')
  assert.equal(popup.state.popupBackgroundUrl.value, 'data:image/jpeg;base64,new')
  popup.setInvoke('read_popup_image', async () => null)
  await popup.emitTo('reminder', 'popup-image-updated')
  assert.equal(popup.state.popupBackgroundUrl.value, '')
})

test('a delayed old image read cannot restore an image after clearing', async () => {
  const popup = await mountPopup()
  let resolveOld
  popup.setInvoke('read_popup_image', () => new Promise((resolve) => { resolveOld = resolve }))
  const pending = popup.state.refreshPopupBackground()
  popup.setInvoke('read_popup_image', async () => null)
  await popup.state.refreshPopupBackground()
  resolveOld('data:image/png;base64,old')
  await pending
  assert.equal(popup.state.popupBackgroundUrl.value, '')
})

test('a late initial read cannot replace a newer trigger', async () => {
  const popup = await mountPopup({ readActive: async (listeners) => {
    listeners.get('reminder-triggered').callback({ payload: powerEvent })
    return restEvent
  } })
  await settle()
  assert.equal(popup.state.current.value.sessionId, powerEvent.sessionId)
  assert.equal(popup.calls.filter((name) => name === 'show').length, 1)
})

test('closing during the initial read prevents a stale popup from opening', async () => {
  const popup = await mountPopup({ readActive: async (listeners) => {
    listeners.get('reminder-closed').callback({ payload: restEvent.sessionId })
    return restEvent
  } })
  assert.equal(popup.state.current.value, null)
  assert.equal(popup.calls.includes('show'), false)
  assert.equal(popup.intervals.size, 0)
})

test('a cached popup ignores duplicate and older sessions after closing', async () => {
  const popup = await mountPopup()
  await popup.state.handleTrigger(powerEvent)
  await popup.state.dismiss()
  await popup.state.handleTrigger(restEvent)
  await popup.state.handleTrigger(powerEvent)
  assert.equal(popup.state.current.value, null)
  assert.equal(popup.calls.filter((name) => name === 'show').length, 1)
  assert.equal(popup.intervals.size, 0)
  await popup.state.handleTrigger({ ...restEvent, sessionId: 3 })
  assert.equal(popup.calls.filter((name) => name === 'show').length, 2)
})

test('a rejected native show clears the pending notification and timers', async () => {
  const popup = await mountPopup()
  popup.setInvoke('show_reminder', async () => false)
  await popup.state.handleTrigger(powerEvent)
  assert.equal(popup.state.current.value, null)
  assert.equal(popup.intervals.size, 0)
  assert.equal(popup.calls.includes('show'), false)
})

async function mountPopup({ label = 'reminder', active = null, readActive } = {}) {
  const data = defaultData()
  const listeners = new Map()
  const intervals = new Map()
  const calls = []
  const nativeHandlers = new Map()
  const invokeHandlers = new Map()
  const windowListeners = new Map()
  let mounted
  let nextInterval = 1
  let clockNow = Date.now()
  let readData = async () => structuredClone(data)
  let confirmAction = async () => true
  const nativeWindow = {
    label,
    listen: async (name, callback) => { listeners.set(name, { target: label, callback }); return noop },
    onCloseRequested: async () => noop,
  }
  for (const name of ['hide', 'setAlwaysOnTop', 'center', 'show', 'setFocus']) {
    nativeWindow[name] = async () => {
      calls.push(name)
      await nativeHandlers.get(name)?.()
    }
  }
  const modules = {
    vue: { ...vue, onMounted: (callback) => { mounted = callback }, onUnmounted: noop },
    '@tauri-apps/api/app': {
      setTheme: async () => { calls.push('setTheme'); await nativeHandlers.get('setTheme')?.() },
    },
    '@tauri-apps/api/core': {
      invoke: async (command, args) => {
        calls.push(command)
        if (invokeHandlers.has(command)) return invokeHandlers.get(command)(args)
        if (command === 'load_data') return readData()
        if (command === 'read_popup_image') return null
        if (command === 'popup_window_is_transparent') return false
        if (command === 'get_active_reminder') return readActive ? readActive(listeners) : active
        if (command === 'hide_idle_window') return nativeWindow.hide()
        if (command === 'show_reminder') {
          assert.equal(typeof args.sessionId, 'number')
          await nativeWindow.show()
          return true
        }
        if (command === 'execute_power_action') {
          listeners.get('reminder-closed')?.callback({ payload: args.sessionId })
          return true
        }
        if (['dismiss_reminder', 'snooze_reminder'].includes(command)) {
          listeners.get('reminder-closed')?.callback({ payload: args.sessionId })
          return
        }
        throw new Error(`Unexpected command: ${command}`)
      },
    },
    '@tauri-apps/api/event': {
      listen: async (name, callback) => { listeners.set(name, { target: null, callback }); return noop },
    },
    '@tauri-apps/api/window': { getCurrentWindow: () => nativeWindow },
    '@tauri-apps/plugin-dialog': { confirm: () => confirmAction() },
    '@lucide/vue': new Proxy({}, { get: () => ({ render: noop }) }),
    '../assets/logo.svg': { default: 'logo.svg' },
    '../i18n': { translate },
    '../types': { defaultData },
    './PopupBackground.vue': { default: { render: noop } },
    '../error': { logError: noop },
  }
  const exports = {}
  runInNewContext(scriptCode, {
    exports,
    require: (name) => {
      assert.ok(Object.hasOwn(modules, name), `Unexpected module: ${name}`)
      return modules[name]
    },
    window: {
      setInterval: (callback) => { const id = nextInterval++; intervals.set(id, callback); return id },
      clearInterval: (id) => intervals.delete(id),
      addEventListener: (name, callback) => windowListeners.set(name, callback),
      removeEventListener: (name) => windowListeners.delete(name),
    },
    document: {
      addEventListener: noop,
      removeEventListener: noop,
      documentElement: { classList: { add: noop, remove: noop } },
    },
    localStorage: { getItem: () => null, setItem: noop },
    Date: class extends Date { static now() { return clockNow } },
    console,
  })
  const state = exports.default.setup({}, { expose: noop })
  await mounted()
  return {
    state, calls, intervals,
    now: () => clockNow,
    setReadData: (callback) => { readData = callback },
    setNative: (name, callback) => nativeHandlers.set(name, callback),
    setInvoke: (name, callback) => invokeHandlers.set(name, callback),
    setConfirm: (callback) => { confirmAction = callback },
    keydown: async (event) => {
      windowListeners.get('keydown')?.({ repeat: false, preventDefault: noop, ...event })
      await settle()
    },
    emitTo: async (target, name, payload) => {
      const listener = listeners.get(name)
      if (listener && (listener.target === null || listener.target === target)) {
        listener.callback({ payload })
      }
      await settle()
    },
    advance: async (milliseconds) => {
      clockNow += milliseconds
      for (const callback of intervals.values()) callback()
      await settle()
    },
  }
}

test('system reminder events sent only to main never start popup handling', async () => {
  const popup = await mountPopup()
  const reads = popup.calls.filter((name) => name === 'load_data').length
  await popup.emitTo('main', 'reminder-triggered', restEvent)
  assert.equal(popup.state.current.value, null)
  assert.equal(popup.calls.filter((name) => name === 'load_data').length, reads)
  assert.equal(popup.calls.filter((name) => name === 'show').length, 0)
  assert.equal(popup.intervals.size, 0)
})

test('popup and main trigger emissions load the reminder content only once', async () => {
  const popup = await mountPopup()
  const reads = popup.calls.filter((name) => name === 'load_data').length
  await popup.emitTo('reminder', 'reminder-triggered', restEvent)
  await popup.emitTo('main', 'reminder-triggered', restEvent)
  assert.equal(popup.calls.filter((name) => name === 'load_data').length, reads + 1)
  assert.equal(popup.intervals.size, 1)
  assert.equal(popup.state.current.value.id, '__rest__')
})

test('import reset clears the current rest and its elapsed timer', async () => {
  const popup = await mountPopup()
  await popup.state.handleTrigger(restEvent)
  await popup.advance(28_000)
  assert.equal(popup.state.restElapsedSeconds.value, 28)
  await popup.emitTo('reminder', 'reminders-reset')
  assert.equal(popup.state.current.value, null)
  assert.equal(popup.state.triggeredAt.value, null)
  assert.equal(popup.state.restElapsedSeconds.value, 0)
  assert.equal(popup.intervals.size, 0)
  assert.ok(!popup.calls.includes('hide'))
})

test('import reset cancels the pending automatic power countdown', async () => {
  const popup = await mountPopup()
  await popup.state.handleTrigger(powerEvent)
  assert.equal(popup.intervals.size, 1)
  await popup.emitTo('reminder', 'reminders-reset')
  await popup.advance(60_000)
  assert.equal(popup.state.current.value, null)
  assert.equal(popup.state.powerCountdown.value, 60)
  assert.equal(popup.intervals.size, 0)
  assert.ok(!popup.calls.includes('execute_power_action'))
})

test('dismiss, snooze, cancellation and reset prevent pending content from restarting old timers', async () => {
  for (const step of ['load_data']) {
    for (const action of ['dismiss', 'snooze', 'cancelRest', 'resetReminders']) {
      for (const event of action === 'cancelRest' ? [restEvent] : [restEvent, powerEvent]) {
        const popup = await mountPopup()
        let resolveStep
        const pending = new Promise((resolve) => { resolveStep = resolve })
        if (step === 'load_data') popup.setReadData(() => pending)
        else popup.setNative(step, () => pending)
        const trigger = popup.state.handleTrigger(event)
        await settle()
        await popup.state[action](14_400)
        resolveStep(defaultData())
        await trigger
        const scenario = `${step}, ${action}, ${event.id}`
        assert.equal(popup.calls.filter((name) => name === 'show').length, 0, scenario)
        assert.equal(popup.intervals.size, 0, scenario)
        await popup.advance(60_000)
        assert.ok(!popup.calls.includes('execute_power_action'), scenario)
      }
    }
  }
})

test('popup theme updates never override the app-wide native theme', async () => {
  const popup = await mountPopup()
  for (const theme of ['dark', 'light', 'dark', 'system']) {
    await popup.emitTo('reminder', 'settings-updated', { ...defaultData().settings, theme })
    assert.equal(popup.state.settings.value.theme, theme)
  }
  await popup.state.handleTrigger(restEvent)
  assert.ok(!popup.calls.includes('setTheme'))
  assert.equal(popup.calls.filter((name) => name === 'show').length, 1)
})

test('a late dismiss or snooze response cannot hide a newer notification', async () => {
  for (const action of ['dismiss', 'snooze']) {
    const popup = await mountPopup()
    await popup.state.handleTrigger(restEvent)
    let resolveAction
    popup.setInvoke(`${action}_reminder`, () => new Promise((resolve) => { resolveAction = resolve }))
    const closing = popup.state[action](14_400)
    await settle()
    await popup.state.handleTrigger(powerEvent)
    const hides = popup.calls.filter((name) => name === 'hide').length
    resolveAction()
    await closing
    assert.equal(popup.calls.filter((name) => name === 'hide').length, hides, action)
    assert.equal(popup.state.current.value.id, powerEvent.id, action)
    assert.equal(popup.intervals.size, 1, action)
  }
})

test('an old power confirmation cannot execute after import reset', async () => {
  const popup = await mountPopup()
  await popup.state.handleTrigger({ ...powerEvent, id: '__test_power__', isTest: true })
  let resolveConfirmation
  popup.setConfirm(() => new Promise((resolve) => { resolveConfirmation = resolve }))
  const executing = popup.state.executePowerAction()
  await popup.emitTo('reminder', 'reminders-reset')
  resolveConfirmation(true)
  await executing
  assert.ok(!popup.calls.includes('execute_power_action'))
})

test('Escape dismisses the active reminder without a duplicate frontend hide', async () => {
  const popup = await mountPopup()
  await popup.state.handleTrigger(restEvent)
  await popup.keydown({ key: 'Escape' })
  assert.ok(popup.calls.includes('dismiss_reminder'))
  assert.ok(!popup.calls.includes('hide'))
  assert.equal(popup.state.current.value, null)
  assert.equal(popup.intervals.size, 0)
})

test('Escape still hides a popup whose reminder state was already cleared', async () => {
  const popup = await mountPopup()
  await popup.keydown({ key: 'Escape' })
  assert.ok(!popup.calls.includes('dismiss_reminder'))
  assert.equal(popup.calls.at(-1), 'hide')
})

test('popup Escape button invokes the same dismissal as the keyboard', () => {
  assert.match(descriptor.template.content, /<button class="popup-escape-hint"[^>]*@click="dismiss">ESC<\/button>/)
})

test('click dismissal cancels rest, event and power sessions and their timers', async () => {
  for (const event of [restEvent, { ...restEvent, id: '__test_event__', isRest: false }, powerEvent]) {
    const popup = await mountPopup()
    await popup.state.handleTrigger(event)
    await popup.state.dismiss()
    await popup.advance(60_000)
    assert.equal(popup.state.current.value, null)
    assert.equal(popup.intervals.size, 0)
    assert.equal(popup.calls.filter(name => name === 'dismiss_reminder').length, 1)
    assert.ok(!popup.calls.includes('execute_power_action'))
  }
  const idle = await mountPopup()
  await idle.state.dismiss()
  assert.equal(idle.calls.at(-1), 'hide')
})

test('a secondary fullscreen popup never executes the automatic power action', async () => {
  const popup = await mountPopup({ label: 'reminder-monitor-1' })
  await popup.state.handleTrigger(powerEvent)
  await popup.advance(60_000)
  assert.ok(!popup.calls.includes('execute_power_action'))
  assert.equal(popup.intervals.size, 0)
})

test('the cached windowed popup remains a controller after a mode switch', async () => {
  const popup = await mountPopup({ label: 'reminder-windowed', active: powerEvent })
  assert.equal(popup.state.current.value.sessionId, powerEvent.sessionId)
  await popup.advance(60_000)
  assert.equal(popup.calls.filter((name) => name === 'execute_power_action').length, 1)
  assert.equal(popup.state.current.value, null)
  assert.equal(popup.intervals.size, 0)
})

test('switching back to a cached fullscreen popup only accepts the newer session', async () => {
  const popup = await mountPopup({ active: restEvent })
  await popup.emitTo('reminder', 'reminder-closed', restEvent.sessionId)
  await popup.emitTo('reminder', 'reminder-closed', powerEvent.sessionId)
  await popup.state.handleTrigger(restEvent)
  assert.equal(popup.state.current.value, null)
  await popup.state.handleTrigger({ ...restEvent, sessionId: 3 })
  assert.equal(popup.state.current.value.sessionId, 3)
  assert.equal(popup.calls.filter((name) => name === 'show').length, 2)
  assert.equal(popup.intervals.size, 1)
})

test('a backend close event clears matching popup state and timers', async () => {
  const popup = await mountPopup()
  await popup.state.handleTrigger(powerEvent)
  await popup.emitTo('reminder', 'reminder-closed', powerEvent.sessionId)
  assert.equal(popup.state.current.value, null)
  assert.equal(popup.intervals.size, 0)
  assert.ok(!popup.calls.includes('hide'))
})

test('fade-in state is ready before the native popup is shown', async () => {
  const popup = await mountPopup()
  popup.setNative('show', () => assert.equal(popup.state.popupAnimating.value, true))
  await popup.state.handleTrigger(restEvent)
})

test('each reminder replaces the popup root so fade-in animation restarts', async () => {
  const popup = await mountPopup()
  await popup.state.handleTrigger(restEvent)
  const firstKey = popup.state.popupAnimationKey.value
  await popup.state.handleTrigger({ ...restEvent, sessionId: 3 })
  assert.equal(popup.state.popupAnimationKey.value, firstKey + 1)
})

test('a newly created monitor popup restores the active reminder on mount', async () => {
  const popup = await mountPopup({ label: 'reminder-monitor-1', active: restEvent })
  assert.equal(popup.state.current.value.sessionId, restEvent.sessionId)
  assert.equal(popup.intervals.size, 1)
  assert.ok(popup.calls.includes('show'))
})
