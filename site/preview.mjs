import { createReadStream } from 'node:fs'
import { stat } from 'node:fs/promises'
import { createServer } from 'node:http'
import { extname, join, resolve, sep } from 'node:path'
import { buildSite, outputDir } from './build.mjs'
import { basePath } from './template.mjs'

await buildSite()

const contentTypes = { '.html': 'text/html; charset=utf-8', '.css': 'text/css; charset=utf-8', '.js': 'text/javascript; charset=utf-8', '.svg': 'image/svg+xml', '.png': 'image/png', '.xml': 'application/xml; charset=utf-8' }
const server = createServer(async (request, response) => {
  try {
    const pathname = decodeURIComponent(new URL(request.url, 'http://127.0.0.1').pathname)
    if (pathname === '/' || pathname === basePath.slice(0, -1)) {
      response.writeHead(302, { Location: basePath }).end()
      return
    }
    if (!pathname.startsWith(basePath)) {
      response.writeHead(404).end('Not found')
      return
    }
    let file = resolve(outputDir, pathname.slice(basePath.length))
    if (file !== outputDir && !file.startsWith(`${outputDir}${sep}`)) {
      response.writeHead(404).end('Not found')
      return
    }
    if ((await stat(file)).isDirectory()) {
      if (!pathname.endsWith('/')) {
        response.writeHead(302, { Location: `${pathname}/` }).end()
        return
      }
      file = join(file, 'index.html')
    }
    await stat(file)
    response.writeHead(200, { 'Content-Type': contentTypes[extname(file)] || 'application/octet-stream', 'Cache-Control': 'no-store' })
    const stream = createReadStream(file)
    stream.on('error', () => response.destroy())
    stream.pipe(response)
  } catch (error) {
    response.writeHead(error.code === 'ENOENT' || error.code === 'ENOTDIR' ? 404 : 400).end('Not found')
  }
})
server.listen(4173, '127.0.0.1', () => console.log(`Preview: http://127.0.0.1:4173${basePath}\nAfter changing the site, stop and restart this command.`))
