import { copyFile, mkdir, writeFile } from 'node:fs/promises'
import { dirname, join, resolve } from 'node:path'
import { fileURLToPath, pathToFileURL } from 'node:url'
import { locales } from './content.mjs'
import { basePath, origin, renderPage, repository } from './template.mjs'

const siteDir = dirname(fileURLToPath(import.meta.url))
export const outputDir = join(siteDir, 'dist')
const latestReleaseApi = 'https://api.github.com/repos/Smileher/ZhiXie/releases/latest'

// 发布附件使用固定英文名（不含版本号），页面才能一直用 latest 路径，链接不随版本失效。
// 名字必须与「版本打包」工作流上传的附件一致。
export const assetNames = {
  windowsSetup: 'zhixie-windows-x64-setup.exe',
  windowsPortable: 'zhixie-windows-x64-portable.zip',
  macPortable: 'zhixie-macos-arm64-portable.zip',
}

// 附件以 latest 路径对外，绕过版本号，长期有效。
export function downloadUrl(name) {
  return `${repository}/releases/latest/download/${name}`
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
    if (!asset) continue
    if (!Number.isSafeInteger(asset.size) || asset.size <= 0) {
      throw new Error(`Invalid release asset size: ${name}`)
    }
    downloads[key] = { url: downloadUrl(name), size: asset.size }
  }
  if (Object.keys(downloads).length === 0) {
    throw new Error(`No known package found in release ${release.tag_name}`)
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
  return parseRelease(await response.json())
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
