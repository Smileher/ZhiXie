import { locales } from './content.mjs'

export const basePath = '/ZhiXie/'
export const origin = 'https://smileher.github.io'
export const repository = 'https://github.com/Smileher/ZhiXie'

export function escapeHtml(value) {
  return String(value).replace(/[&<>"']/g, (character) => ({ '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;', "'": '&#39;' })[character])
}

const paths = {
  download: '<path d="M12 3v12m-5-5 5 5 5-5M5 16v5h14v-5"/>',
  coffee: '<path d="M4 9h13v6a5 5 0 0 1-5 5H9a5 5 0 0 1-5-5Zm13 0h2a3 3 0 1 1 0 6h-2M7 3v2m4-2v2m4-2v2M3 22h16"/>',
  calendar: '<rect x="3" y="5" width="18" height="16" rx="3"/><path d="M16 3v4M8 3v4M3 11h18m-13 5h2m4 0h2"/>',
  palette: '<path d="M12 3a9 9 0 1 0 0 18h1a2 2 0 0 0 1-4 1.5 1.5 0 0 1 1-3h3a3 3 0 0 0 3-3 9 9 0 0 0-9-8Z"/><path d="M7 10h.01M10 6.5h.01M15 7h.01M6.5 15h.01"/>',
  sparkles: '<path d="M12 3l1.8 4.7L18.5 9.5 13.8 11.3 12 16l-1.8-4.7L5.5 9.5l4.7-1.8Z"/><path d="M18 16l.9 2.3L21 19.2l-2.1.8L18 22.3l-.9-2.3L15 19.2l2.1-.9Z"/>',
  tray: '<rect x="3" y="4" width="18" height="16" rx="3"/><path d="M3 12h7l1.5 2h1l1.5-2h7M12 8v4m-2-2 2 2 2-2"/>',
  shield: '<path d="M12 3l7 3v6c0 4-3 7-7 9-4-2-7-5-7-9V6Z"/><path d="m9 12 2 2 4-4"/>',
  github: '<path d="M9 19c-4 1.2-4-2.1-6-2.6m12 5v-3.6a3 3 0 0 0-.9-2.4c3-.3 5.4-1.5 5.4-6.6A5 5 0 0 0 18.4 6a4.6 4.6 0 0 0-.1-3.4s-1.3-.4-4.2 1.6a10.5 10.5 0 0 0-5.6 0C5.6 2.2 4.3 2.6 4.3 2.6A4.6 4.6 0 0 0 4.2 6 5 5 0 0 0 3 8.8c0 5 2.4 6.3 5.4 6.6a3 3 0 0 0-.9 2.3V22"/>',
  sun: '<circle cx="12" cy="12" r="4"/><path d="M12 2v2m0 16v2M2 12h2m16 0h2M5 5l1.5 1.5m11 11L19 19M5 19l1.5-1.5m11-11L19 5"/>',
  moon: '<path d="M20 14A9 9 0 0 1 10 3a9 9 0 1 0 10 11Z"/>',
  windows: '<path d="m3 5 8-1v7H3Zm10-1 8-1v8h-8ZM3 13h8v7l-8-1Zm10 0h8v8l-8-1Z"/>',
  apple: '<path d="M15 3c0 2-1.5 3-3 3 0-2 1-3 3-3Zm-3 5c-2-2-7-1-7 4 0 4 3 9 5 9 1 0 1-1 2-1s2 1 3 1c2 0 4-3 4-5-3-1-4-5-1-7-2-2-4-2-6-1Z"/>',
  check: '<path d="m5 12 4 4L19 6"/>',
  chevron: '<path d="m9 6 6 6-6 6"/>',
  send: '<path d="M4 12l16-8-6 16-2.5-6.5Z"/>',
}

function icon(name, className = '', size = 24) {
  return `<svg class="icon ${className}" width="${size}" height="${size}" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.6" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true" focusable="false">${paths[name]}</svg>`
}

// 每张下载卡片按优先级挑一个可用附件：安装包优先，其次便携包，都没有就退回版本列表页。
const downloadTargets = [
  { key: 'windows', icon: 'windows', platform: 'windows', candidates: ['windowsSetup', 'windowsPortable'] },
  { key: 'mac', icon: 'apple', platform: 'macos', candidates: ['macPortable'] },
]

function formatSize(bytes, locale) {
  return new Intl.NumberFormat(locale, { minimumFractionDigits: 1, maximumFractionDigits: 1 }).format(bytes / 1024 / 1024)
}

function formatDate(iso, locale) {
  return new Intl.DateTimeFormat(locale, { dateStyle: 'long', timeZone: 'UTC' }).format(new Date(iso))
}

function renderDownload(t, release, e) {
  const cards = downloadTargets
    .map(({ key, icon: brand, platform, candidates }) => {
      const picked = release ? candidates.find((name) => release.downloads[name]) : undefined
      const asset = picked ? release.downloads[picked] : undefined
      const meta = []
      if (release) meta.push(`v${e(release.version)}`)
      if (asset) meta.push(`${e(t.download.sizeLabel)} ${formatSize(asset.size, t.lang)} MiB`)
      if (release) meta.push(`${e(t.download.released)} ${e(formatDate(release.publishedAt, t.lang))}`)
      return `<article class="pkg reveal">
        <div class="pkg-head">
          <span class="pkg-mark">${icon(brand, '', 24)}</span>
          <div class="pkg-id">
            <h3 class="pkg-name">${e(t.download[platform])}</h3>
            <p class="pkg-meta">${meta.join(' · ')}</p>
          </div>
        </div>
        <p class="pkg-note">${e(t.download[`${platform}Note`])}</p>
        <a class="btn ${asset ? 'btn-solid' : 'btn-quiet'}" href="${e(asset ? asset.url : `${repository}/releases`)}" rel="noopener">${icon('download', '', 18)}<span>${e(t.download.downloadLabel)}</span></a>
      </article>`
    })
    .join('')

  const waiting = `<article class="pkg pkg-wait reveal">
    <div class="pkg-head">
      <span class="pkg-mark">${icon('send', '', 24)}</span>
      <div class="pkg-id">
        <h3 class="pkg-name">${e(t.download.waiting)}</h3>
      </div>
    </div>
    <p class="pkg-note">${e(t.download.waitingBody)}</p>
    <div class="pkg-actions">
      <a class="btn btn-solid" href="${repository}/releases" rel="noopener">${icon('download', '', 18)}<span>${e(t.download.browseReleases)}</span></a>
      <a class="btn btn-quiet" href="${repository}" rel="noopener">${icon('github', '', 18)}<span>${e(t.download.starRepo)}</span></a>
    </div>
  </article>`

  return `<section class="section" id="download">
    <div class="shell">
      <div class="section-head reveal">
        <h2>${e(t.download.title)}</h2>
        <p class="lede">${e(t.download.subtitle)}</p>
      </div>
      <div class="pkg-grid">${release ? cards : `${cards}${waiting}`}</div>
    </div>
  </section>`
}

// 弹窗只用纯 HTML/CSS 画，文案取自软件默认歇息文案。
function renderMock(t, e) {
  return `<figure class="mock reveal">
    <div class="mock-bar"><span class="mock-dot"></span><span class="mock-app">${e(t.mock.appName)}</span></div>
    <div class="mock-body">
      <p class="mock-outlet">${e(t.mock.outlet)}</p>
      <div class="mock-breath" aria-hidden="true"><span></span><span></span><span></span></div>
      <p class="mock-message">${e(t.mock.message)}</p>
      <div class="mock-actions"><span class="mock-btn">${e(t.mock.snooze)}</span><span class="mock-btn mock-btn-solid">${e(t.mock.finish)}</span></div>
    </div>
  </figure>`
}

export function renderPage(language, release) {
  const t = locales[language]
  const e = escapeHtml
  const other = language === 'zh' ? ['en', locales.en] : ['zh', locales.zh]
  const url = `${origin}${basePath}${t.path}`
  const primary = release
    ? release.downloads.windowsSetup?.url ?? release.downloads.windowsPortable?.url ?? release.downloads.macPortable?.url ?? `${repository}/releases`
    : `${repository}/releases`

  const navLinks = ['features', 'download']
    .map((key) => `<a class="nav-link" href="#${key}">${e(t.nav[key])}</a>`)
    .join('')
  const chips = t.intro.chips.map((chip) => `<li>${icon('check', 'chip-icon', 15)}<span>${e(chip)}</span></li>`).join('')
  const features = t.features.items
    .map(
      (item) => `<article class="card reveal">
        <span class="card-icon">${icon(item.icon, '', 20)}</span>
        <div class="card-text"><h3>${e(item.title)}</h3><p>${e(item.hint)}</p></div>
      </article>`,
    )
    .join('')
  const scenarios = t.scenarios.items.map((item) => `<li>${e(item)}</li>`).join('')

  return `<!doctype html>
<html lang="${t.lang}" data-theme-mode="auto">
<head>
  <meta charset="UTF-8">
  <meta name="viewport" content="width=device-width, initial-scale=1">
  <meta name="color-scheme" content="dark light">
  <meta name="theme-color" content="#0F1614" media="(prefers-color-scheme: dark)">
  <meta name="theme-color" content="#FAF5EC" media="(prefers-color-scheme: light)">
  <title>${e(t.title)}</title>
  <meta name="description" content="${e(t.description)}">
  <meta name="keywords" content="${e(t.keywords)}">
  <link rel="canonical" href="${url}">
  <link rel="alternate" hreflang="zh-CN" href="${origin}${basePath}">
  <link rel="alternate" hreflang="en" href="${origin}${basePath}en/">
  <link rel="alternate" hreflang="x-default" href="${origin}${basePath}">
  <meta property="og:type" content="website">
  <meta property="og:site_name" content="知歇 ZhiXie">
  <meta property="og:title" content="${e(t.title)}">
  <meta property="og:description" content="${e(t.description)}">
  <meta property="og:url" content="${url}">
  <meta property="og:image" content="${origin}${basePath}assets/og-image.png">
  <meta name="twitter:card" content="summary">
  <meta name="twitter:title" content="${e(t.title)}">
  <meta name="twitter:description" content="${e(t.description)}">
  <meta name="twitter:image" content="${origin}${basePath}assets/og-image.png">
  <link rel="icon" type="image/svg+xml" href="${basePath}assets/logo.svg">
  <link rel="stylesheet" href="${basePath}assets/style.css">
  <script defer src="${basePath}assets/app.js"></script>
</head>
<body>
  <a class="skip-link" href="#main">${e(t.nav.skip)}</a>

  <header class="topbar">
    <div class="shell topbar-inner">
      <a class="brand" href="${url}">
        <img class="brand-mark" src="${basePath}assets/logo.svg" alt="" width="30" height="30">
        <span class="brand-text"><strong>知歇</strong><em>ZhiXie</em></span>
      </a>
      <nav class="nav" aria-label="${e(t.nav.aria)}">
        ${navLinks}
        <a class="nav-link" href="${repository}" rel="noopener">${icon('github', '', 17)}<span>${e(t.nav.source)}</span></a>
      </nav>
      <div class="topbar-tools">
        <a class="lang" href="${origin}${basePath}${other[1].path}" hreflang="${other[1].lang}">${e(other[0] === 'zh' ? '简体中文' : 'English')}</a>
        <button class="theme-btn" type="button" data-theme-toggle aria-label="${e(t.nav.theme)}">
          <span class="theme-icon theme-icon-sun">${icon('sun', '', 18)}</span>
          <span class="theme-icon theme-icon-moon">${icon('moon', '', 18)}</span>
        </button>
      </div>
    </div>
  </header>

  <main id="main">
    <section class="hero" id="top">
      <div class="shell hero-inner">
        <div class="hero-copy">
          <p class="kicker">${e(t.hero.kicker)}</p>
          <h1>${t.hero.titleLines.map((line) => e(line)).join('<br>')}</h1>
          <p class="lede">${e(t.hero.subtitle)}</p>
          <div class="hero-actions">
            <a class="btn btn-solid btn-lg" href="${e(primary)}" rel="noopener">${icon('download', '', 19)}<span>${e(t.hero.primary)}</span></a>
            <a class="btn btn-line btn-lg" href="${repository}" rel="noopener">${icon('github', '', 19)}<span>${e(t.hero.secondary)}</span></a>
          </div>
        </div>
        ${renderMock(t, e)}
      </div>
    </section>

    <section class="section" id="features">
      <div class="shell">
        <div class="section-head reveal">
          <h2>${e(t.intro.title)}</h2>
          <p class="lede">${e(t.intro.subtitle)}</p>
          <ul class="chips">${chips}</ul>
        </div>
        <div class="cards">${features}</div>
        <div class="scenarios reveal">
          <h3>${e(t.scenarios.title)}</h3>
          <ul>${scenarios}</ul>
        </div>
      </div>
    </section>

    ${renderDownload(t, release, e)}
  </main>

  <footer class="footer">
    <div class="shell footer-inner">
      <p class="footer-tagline">${e(t.footer.tagline)}</p>
      <div class="footer-links">
        <a href="${repository}" rel="noopener">${icon('github', '', 16)}<span>${e(t.footer.repository)}</span></a>
        <a href="${repository}/releases" rel="noopener">${icon('download', '', 16)}<span>${e(t.footer.releases)}</span></a>
      </div>
    </div>
    <p class="footer-copy">© ${new Date().getUTCFullYear()} ${e(t.footer.copyright)}</p>
  </footer>
</body>
</html>
`
}
