import assert from 'node:assert/strict'
import { readFileSync } from 'node:fs'
import { createRequire } from 'node:module'
import { test } from 'node:test'
import { runInNewContext } from 'node:vm'
import ts from 'typescript'
import * as vue from 'vue'
import { translate } from '../src/i18n.ts'

const require = createRequire(import.meta.url)
const vueRequire = createRequire(require.resolve('vue'))
const { parse, compileScript } = vueRequire('@vue/compiler-sfc')
const { descriptor } = parse(readFileSync(new URL('../src/components/RestView.vue', import.meta.url), 'utf8'))
const script = compileScript(descriptor, { id: 'rest-view-test' })
const code = ts.transpileModule(script.content, { compilerOptions: { module: ts.ModuleKind.CommonJS } }).outputText

test('active reminder edits survive countdown and saved-data refreshes until committed', async () => {
  const exports = {}
  const modules = { vue, '@lucide/vue': {}, '../i18n': { translate } }
  runInNewContext(code, { exports, require: name => modules[name] })
  const props = vue.reactive({ language: 'zh-CN', enabled: true, intervalMinutes: 40, message: 'Saved', status: '10:00' })
  const emitted = []
  const state = exports.default.setup(props, { expose() {}, emit: (...args) => emitted.push(args) })
  state.editingInterval.value = true
  state.editingMessage.value = true
  state.intervalDraft.value = '120'
  state.messageDraft.value = 'New reminder'
  props.status = '09:59'
  props.intervalMinutes = 45
  props.message = 'Older synchronized message'
  await vue.nextTick()
  assert.equal(state.intervalDraft.value, '120')
  assert.equal(state.messageDraft.value, 'New reminder')
  state.commitInterval()
  state.commitMessage()
  assert.deepEqual(emitted, [['update:interval', '120'], ['update:message', 'New reminder'], ['messageCommitted', 'New reminder']])
  state.editingInterval.value = false
  state.editingMessage.value = false
  props.intervalMinutes = 120
  props.message = 'Normalized reminder'
  await vue.nextTick()
  assert.equal(state.intervalDraft.value, '120')
  assert.equal(state.messageDraft.value, 'Normalized reminder')
  for (const invalid of ['', '0', '1441', '2.5', 'invalid']) {
    const count = emitted.length
    state.intervalDraft.value = invalid
    state.commitInterval()
    assert.equal(emitted.length, count)
    assert.equal(state.intervalDraft.value, '120')
  }
})
