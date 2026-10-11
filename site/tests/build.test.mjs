import assert from 'node:assert/strict'
import { test } from 'node:test'
import { assetNames, fetchLatestRelease, parseRelease } from '../build.mjs'
import { locales } from '../content.mjs'
import { basePath, renderPage } from '../template.mjs'

const release = {
  tag_name: 'v1.0.0',
  draft: false,
  prerelease: false,
  published_at: '2026-10-12T00:00:00Z',
  html_url: 'https://github.com/Smileher/ZhiXie/releases/tag/v1.0.0',
  assets: Object.entries(assetNames).map(([key, name]) => ({
    name,
    size: key === 'mac' ? 4_718_592 : 5_242_880,
    browser_download_url: `https://github.com/Smileher/ZhiXie/releases/download/v1.0.0/${name}`,
  })),
}

test('parseRelease keeps the version and downloads together', () => {
  const parsed = parseRelease(release)
  assert.equal(parsed.version, '1.0.0')
  assert.equal(parsed.downloads.windows.size, 5_242_880)
  assert.equal(parsed.downloads.mac.url, release.assets[1].browser_download_url)
})

for (const [label, patch] of [
  ['drafts', { draft: true }],
  ['prereleases', { prerelease: true }],
  ['unversioned tags', { tag_name: 'nightly' }],
  ['missing dates', { published_at: undefined }],
  ['renamed assets', { assets: [] }],
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
  assert.ok(!html.includes('zhixie-windows-x64-portable.zip'))
})

test('a published release reaches the download buttons', () => {
  const html = renderPage('en', parseRelease(release))
  assert.ok(html.includes(release.assets[0].browser_download_url))
  assert.ok(html.includes('v1.0.0'))
  assert.ok(html.includes('October 12, 2026'))
})

test('locales keep the same shape', () => {
  const [zh, en] = Object.values(locales)
  assert.deepEqual(Object.keys(zh), Object.keys(en))
  assert.equal(zh.features.items.length, en.features.items.length)
  assert.equal(zh.rhythm.steps.length, en.rhythm.steps.length)
  assert.equal(zh.faq.items.length, en.faq.items.length)
  assert.deepEqual(zh.features.items.map((item) => item.icon), en.features.items.map((item) => item.icon))
})
