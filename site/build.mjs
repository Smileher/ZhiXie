import { copyFile, mkdir, writeFile } from 'node:fs/promises'
import { dirname, join, resolve } from 'node:path'
import { fileURLToPath, pathToFileURL } from 'node:url'
import { locales } from './content.mjs'
import { basePath, origin, renderPage, repository } from './template.mjs'

const siteDir = dirname(fileURLToPath(import.meta.url))
export const outputDir = join(siteDir, 'dist')
const latestReleaseApi = 'https://api.github.com/repos/Smileher/ZhiXie/releases/latest'

// 便携包文件名由「便携包构建」工作流产出的附件决定。
export const assetNames = {
  windows: 'zhixie-windows-x64-portable.zip',
  mac: 'zhixie-macos-arm64-portable.zip',
}

// 读到一条发布结果后，版本号、发布日期与全部下载必须来自同一个 Release。
export function parseRelease(release) {
  const version = /^v(\d+\.\d+\.\d+)$/.exec(release?.tag_name)?.[1]
  if (!version || release.draft !== false || release.prerelease !== false) {
    throw new Error('Expected a published stable release with a vX.Y.Z tag')
  }
  if (typeof release.published_at !== 'string' || !Number.isFinite(Date.parse(release.published_at))) {
    throw new Error('Release publication date is missing or invalid')
  }
  if (release.html_url !== `${repository}/releases/tag/${release.tag_name}` || !Array.isArray(release.assets)) {
    throw new Error('Release URL or assets are invalid')
  }
  const downloads = {}
  for (const [key, name] of Object.entries(assetNames)) {
    const asset = release.assets.find((item) => item.name === name)
    if (!asset || asset.browser_download_url !== `${repository}/releases/download/${release.tag_name}/${name}` || !Number.isSafeInteger(asset.size) || asset.size <= 0) {
      throw new Error(`Missing or invalid release asset: ${name}`)
    }
    downloads[key] = { url: asset.browser_download_url, size: asset.size }
  }
  return { version, publishedAt: release.published_at, url: release.html_url, downloads }
}

// 首个安装包发布前仓库还没有 Release，此时站点照常构建，下载区退化为「查看版本」入口。
export async function fetchLatestRelease(fetchImpl = fetch) {
  const headers = { Accept: 'application/vnd.github+json', 'User-Agent': 'ZhiXie-Pages', 'X-GitHub-Api-Version': '2022-11-28' }
  if (process.env.GH_TOKEN) headers.Authorization = `Bearer ${process.env.GH_TOKEN}`
  const response = await fetchImpl(latestReleaseApi, { headers, signal: AbortSignal.timeout(20000) })
  if (response.status === 404) return null
  if (!response.ok) throw new Error(`GitHub release request failed (HTTP ${response.status})`)
  const release = await response.json()
  return parseRelease(release)
}

export async function buildSite(fetchImpl = fetch) {
  const release = await fetchLatestRelease(fetchImpl)
  await mkdir(join(outputDir, 'assets'), { recursive: true })
  await copyFile(join(siteDir, '..', 'src', 'assets', 'logo.svg'), join(outputDir, 'assets', 'logo.svg'))
  await copyFile(join(siteDir, '..', 'src-tauri', 'icons', '128x128.png'), join(outputDir, 'assets', 'og-image.png'))
  await copyFile(join(siteDir, 'style.css'), join(outputDir, 'assets', 'style.css'))
  await copyFile(join(siteDir, 'app.js'), join(outputDir, 'assets', 'app.js'))
  for (const [language, { path }] of Object.entries(locales)) {
    await mkdir(join(outputDir, path), { recursive: true })
    await writeFile(join(outputDir, path, 'index.html'), renderPage(language, release), 'utf8')
  }
  const urls = Object.values(locales).map(({ path }) => `${origin}${basePath}${path}`)
  await writeFile(
    join(outputDir, 'sitemap.xml'),
    `<?xml version="1.0" encoding="UTF-8"?>\n<urlset xmlns="http://www.sitemaps.org/schemas/sitemap/0.9">${urls.map((url) => `<url><loc>${url}</loc></url>`).join('')}</urlset>\n`,
    'utf8'
  )
  await writeFile(join(outputDir, '.nojekyll'), '', 'utf8')
  console.log(`Built ZhiXie ${release ? `v${release.version}` : '(release pending)'}: ${outputDir}`)
  return release
}

if (process.argv[1] && import.meta.url === pathToFileURL(resolve(process.argv[1])).href) {
  await buildSite()
}
