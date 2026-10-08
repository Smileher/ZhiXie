(() => {
  const prevent = (event) => event.preventDefault()
  const isMac = navigator.platform.startsWith('Mac')
  const browserKeys = new Set([
    'F1', 'F3', 'F5', 'F6', 'F7', 'F11', 'F12', 'ContextMenu',
    'BrowserBack', 'BrowserForward', 'BrowserRefresh', 'BrowserStop',
    'BrowserSearch', 'BrowserFavorites', 'BrowserHome',
  ])
  const commandKeys = new Set(['p', 'r', 's', 'o', 'f', 'g', 'l', 'u', '+', '-', '=', '0', '[', ']'])

  const isEditable = (target) => target instanceof Element
    && (target.closest('input, textarea') !== null || target.isContentEditable)
  window.addEventListener('contextmenu', (event) => {
    if (!isEditable(event.target)) event.preventDefault()
  }, true)
  window.addEventListener('keydown', (event) => {
    const key = event.key.toLowerCase()
    const command = event.metaKey || (!isMac && event.ctrlKey)
    const inspect = command && (event.shiftKey || event.altKey) && ['i', 'j', 'c', 'k'].includes(key)
    const editable = isEditable(event.target)
    // macOS 的 Option + 方向键用于按词移动，Command + 方向键用于行首行尾。
    const navigation = (!editable || !isMac)
      && ((event.altKey && ['ArrowLeft', 'ArrowRight', 'Home'].includes(event.key))
        || (isMac && event.metaKey && ['ArrowLeft', 'ArrowRight'].includes(event.key)))
    if ((browserKeys.has(event.key) && !(event.key === 'ContextMenu' && editable))
      || (event.shiftKey && event.key === 'F10' && !editable)
      || (command && commandKeys.has(key))
      || inspect || navigation
      || (event.key === 'Backspace' && !editable)) {
      event.preventDefault()
      event.stopImmediatePropagation()
    }
  }, true)

  // Ctrl + 滚轮以及 macOS 触控板手势不经过 keydown。
  window.addEventListener('wheel', (event) => {
    if (event.ctrlKey || event.metaKey) event.preventDefault()
  }, { capture: true, passive: false })
  window.addEventListener('gesturestart', prevent, { capture: true, passive: false })
  window.addEventListener('gesturechange', prevent, { capture: true, passive: false })

  for (const type of ['mousedown', 'mouseup', 'auxclick']) {
    window.addEventListener(type, (event) => {
      if ([1, 3, 4].includes(event.button)) event.preventDefault()
    }, true)
  }
  for (const type of ['dragstart', 'dragover', 'drop']) {
    window.addEventListener(type, prevent, true)
  }
  window.print = () => {}
})()
