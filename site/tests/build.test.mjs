import assert from 'node:assert/strict'
import { test } from 'node:test'
import { assetNames, fetchLatestRelease, parseRelease } from '../build.mjs'
import { locales } from '../content.mjs'
import { basePath, renderPage } from '../template.mjs'

const asset = (key, size) => ({
  name: assetNames[key],
  size,
  browser_download_url: `https://github.com/Smileher/ZhiXie/releases/download/v1.0.0/${assetNames[key]}`,
})

const release = {
  tag_name: 'v1.0.0',
  draft: false,
  prerelease: false,
  published_at: '2026-10-12T00:00:00Z',
  html_url: 'https://github.com/Smileher/ZhiXie/releases/tag/v1.0.0',
  assets: [asset('windowsPortable', 5_242_880), asset('macDisk', 6_291_456), asset('windowsSetup', 6_553_600)],
}

test('parseRelease keeps the version and downloads together', () => {
  const parsed = parseRelease(release)
  assert.equal(parsed.version, '1.0.0')
  assert.equal(parsed.downloads.windowsPortable.size, 5_242_880)
  assert.equal(parsed.downloads.macDisk.size, 6_291_456)
})

test('downloads always use the version-free latest path', () => {
  const parsed = parseRelease(release)
  for (const [key, { url }] of Object.entries(parsed.downloads)) {
    assert.equal(url, `https://github.com/Smileher/ZhiXie/releases/latest/download/${assetNames[key]}`, key)
  }
})

test('parseRelease tolerates optional packages being absent', () => {
  const parsed = parseRelease({ ...release, assets: [asset('windowsPortable', 5_242_880)] })
  assert.equal(parsed.downloads.macDisk, undefined)
  assert.equal(parsed.downloads.windowsSetup, undefined)
  assert.ok(parsed.downloads.windowsPortable)
})

test('parseRelease rejects a release without any known package', () => {
  assert.throws(() => parseRelease({ ...release, assets: [{ name: 'unknown.zip', size: 10 }] }), /No known package/)
})

for (const [label, patch] of [
  ['drafts', { draft: true }],
  ['prereleases', { prerelease: true }],
  ['unversioned tags', { tag_name: 'nightly' }],
  ['missing dates', { published_at: undefined }],
  ['invalid sizes', { assets: [{ ...asset('macDisk', 0) }] }],
]) {
  test(`parseRelease rejects ${label}`, () => {
    assert.throws(() => parseRelease({ ...release, ...patch }), Error)
  })
}

test('fetchLatestRelease treats a missing release as pending', async () => {
  const result = await fetchLatestRelease(async () => ({ ok: false, status: 404 }))
  assert.equal(result, null)
})

test('fetchLatestRelease surfaces API failures', async () => {
  await assert.rejects(fetchLatestRelease(async () => ({ ok: false, status: 500 })), /HTTP 500/)
})

test('pages reference site assets and the canonical URL', () => {
  for (const [language, locale] of Object.entries(locales)) {
    const html = renderPage(language, null)
    assert.match(html, new RegExp(`<html lang="${locale.lang}"`))
    assert.match(html, new RegExp(`<link rel="canonical" href="https://smileher\\.github\\.io${basePath}${locale.path}">`))
    assert.ok(html.includes(`${basePath}assets/style.css`), language)
    assert.ok(html.includes(`${basePath}assets/app.js`), language)
    // 语言切换指向另一份语言版本：/ZhiXie/ 与 /ZhiXie/en/ 互指。
    const otherPath = locale.path === '' ? 'en/' : ''
    assert.ok(html.includes(`href="https://smileher.github.io${basePath}${otherPath}" hreflang=`), language)
  }
})

test('the download section falls back to the repository until a release exists', () => {
  const html = renderPage('zh', null)
  assert.ok(html.includes('https://github.com/Smileher/ZhiXie/releases'))
  assert.ok(!html.includes('latest/download/'))
})

test('a published release puts stable portable links on both platform cards', () => {
  const html = renderPage('en', parseRelease(release))
  assert.ok(html.includes('releases/latest/download/zhixie-windows-x64-portable.exe'))
  assert.ok(html.includes('releases/latest/download/zhixie-macos-arm64.dmg'))
  assert.ok(html.includes('v1.0.0'))
  assert.ok(html.includes('October 12, 2026'))
})

test('each section is wrapped in the shared shell container', () => {
  const html = renderPage('zh', null)
  for (const section of ['<main id="main">', '<footer class="footer">']) {
    assert.ok(html.includes(section), section)
  }
  const bodies = html.split('<section class="').slice(1)
  assert.equal(bodies.length, 3)
  for (const body of bodies) {
    assert.ok(body.includes('<div class="shell'), 'missing shell wrapper')
  }
})

test('locales keep the same shape', () => {
  const [zh, en] = Object.values(locales)
  assert.deepEqual(Object.keys(zh), Object.keys(en))
  assert.equal(zh.features.items.length, en.features.items.length)
  assert.equal(zh.scenarios.items.length, en.scenarios.items.length)
  assert.deepEqual(zh.features.items.map((item) => item.icon), en.features.items.map((item) => item.icon))
  assert.deepEqual(Object.keys(zh.mock), Object.keys(en.mock))
  for (const locale of Object.values(locales)) {
    assert.ok(locale.hero.titleLines.length >= 2, locale.lang)
  }
})
