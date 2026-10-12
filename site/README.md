# 知歇官网

官网由 Node.js 原生能力生成，不引入任何依赖。视觉采用「暖纸墨感」：纸白暖底、发丝描边卡片、衬线标题，薄荷青与青绿作为点缀；Hero 区的休息弹窗由纯 HTML/CSS 绘制，文案取自软件真实界面，不使用位图截图。中文路径为 `/ZhiXie/`，英文路径为 `/ZhiXie/en/`。样式、脚本与图标全部由本站提供，不加载外部字体或脚本。

主题默认跟随系统，也可在顶栏切换浅色或深色，选择结果保存在浏览器本地。禁用 JavaScript 时页面依旧可读，渐显动效自动关闭。

## 本地预览

在项目根目录运行：

```sh
node site/preview.mjs
```

打开 <http://127.0.0.1:4173/ZhiXie/>。预览命令先查询 GitHub 最新正式 Release，再构建到已被仓库忽略的 `site/dist/`，最后启动一个只监听本机的静态服务器。需要能够访问 `api.github.com`；遇到匿名 API 限流时用环境变量 `GH_TOKEN` 提供只读令牌，不要把令牌写进源码。

改动后 `Ctrl+C` 停止再重新运行命令即可，无需重启桌面软件。

## 验证与正式构建

```sh
node --test site/tests/*.test.mjs
node site/build.mjs
```

版本号、发布日期、文件大小由 GitHub API 的最新正式 Release 提供，不从 `package.json` 回退，也不在访客浏览器里调用 API。仓库还没有正式 Release 时，构建同样成功，下载区显示安装包准备提示与「查看全部版本」入口，首个安装包发布后自动切换为真实下载按钮。

下载链接固定指向 `/releases/latest/download/<附件名>`，**不含版本号**，因此每次发版都不会让旧链接失效。附件名由「版本打包」工作流决定：`zhixie-windows-x64-portable.exe`（双击直接运行的便携 exe）、`zhixie-macos-arm64.dmg`（双击挂载的磁盘镜像）；两者都不是压缩包，下载后直接双击即可。`zhixie-windows-x64-setup.exe`（安装包）与 `zhixie-macos-arm64-portable.zip`（dmg 构建失败时的兜底）会一并上传，但页面优先展示便携 exe 与 dmg，只有对应的免压缩包产物缺失时才回退。

页面保持单屏或接近单屏：Hero（一句话介绍 + 下载入口）→ 功能与特点（六项短条目 + 适用场景）→ 下载（Windows / macOS 两张卡片）。仓库入口仅保留在页脚的小图标中，无可见文字，悬停提示与无障碍名称随页面语言切换。所有区块内容都包在 `.shell` 容器内，左右留白由 `--gutter`、上下留白由 `--block` 统一控制。

API 请求失败（非 404）会让正式构建失败，不上传新产物、不替换线上页面；草稿与预发布不会被当作正式版本。

## 发布

首次上线前，在仓库 **Settings → Pages → Build and deployment → Source** 里选择 **GitHub Actions**。之后把改动提交推送到 `main`，或手动运行 **Pages** 工作流即可重新部署，线上地址为 <https://smileher.github.io/ZhiXie/>。

- `site/**`、`src/assets/logo.svg`、`src-tauri/icons/128x128.png` 变更推送到 `main` 时触发部署。
- **版本打包** 工作流（打 `v*` 标签时运行）构建 Windows x64 安装包与便携包、macOS Apple Silicon 便携包并上传到 Release，成功后 Pages 通过 `workflow_run` 重新构建，从 `main` 读取网站源码并带上最新版本的下载信息。
- 构建失败时保留上一次成功部署的版本，修复后重跑 **Pages** 即可。

## 维护

- 中英文案：`content.mjs`，共享页面结构：`template.mjs`，样式：`style.css`，主题切换与渐显：`app.js`。
- 附件名在 `build.mjs` 的 `assetNames`，必须与「版本打包」工作流上传的附件名一致；改文件名会同时改坏 latest 下载路径，两边要一起改。
- 基础路径与正式域名定义在 `template.mjs`。使用 GitHub 项目 Pages 地址，不配置自定义域名。
- 软件界面或文案调整后，同步 `content.mjs` 里 `mock`（弹窗示意图）与 `features`（六项短条目），使官网展示与真实界面一致。
- 文案强调休息与健康节律，避免强调加班、提高强度或过度追求效率。
- 发布前至少检查 375px、768px、1440px 三个宽度，中英文两种语言，以及深浅两种主题。
