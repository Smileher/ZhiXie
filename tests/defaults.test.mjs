import assert from 'node:assert/strict'
import { test } from 'node:test'
import { defaultData } from '../src/types.ts'
import { translate } from '../src/i18n.ts'

test('new configuration enables a 40-minute break and shared popup defaults', () => {
  const { settings } = defaultData()
  assert.equal(settings.restEnabled, true)
  assert.equal(settings.restIntervalMinutes, 40)
  assert.equal(settings.restMessage, '起身走走，喝口水，作业先放一放。')
  assert.equal(settings.theme, 'system')
  assert.equal(settings.accentColor, 'blue')
  assert.equal(settings.popupFullscreen, true)
  assert.equal(settings.popupAlwaysOnTop, true)
  assert.equal(settings.popupFadeEnabled, true)
  assert.equal(settings.popupBackgroundFit, 'stretch')
  assert.equal(settings.popupBackgroundScale, 100)
  assert.equal(settings.popupBackgroundOffsetX, 0)
  assert.equal(settings.popupBackgroundOffsetY, 0)
  assert.equal(settings.autostart, false)
  assert.equal(settings.minimizeToTray, true)
})

test('presets are disabled and Friday shutdown does not overlap the workday lock', () => {
  const { reminders } = defaultData()
  assert.equal(reminders.length, 3)
  assert.ok(reminders.every(reminder => !reminder.enabled))
  assert.deepEqual(reminders.map(({ type, time, weekdays, powerAction }) => ({ type, time, weekdays, powerAction })), [
    { type: 'daily', time: '11:50', weekdays: undefined, powerAction: 'lock' },
    { type: 'weekly', time: '17:30', weekdays: [1, 2, 3, 4], powerAction: 'lock' },
    { type: 'weekly', time: '17:30', weekdays: [5], powerAction: 'shutdown' },
  ])
  const english = defaultData('en')
  assert.equal(english.settings.restMessage, translate('en', 'rest.defaultMessage'))
  assert.equal(english.reminders[2].title, translate('en', 'preset.weekend'))
  reminders[1].weekdays.push(5)
  assert.deepEqual(defaultData().reminders[1].weekdays, [1, 2, 3, 4])
})
