// 主题选择与滚动渐显。禁用 JavaScript 时页面保持可读、左导航与 FAQ 仍可展开。
(function () {
  var root = document.documentElement
  var key = 'zhixie-theme'

  function stored() {
    try {
      return localStorage.getItem(key)
    } catch (error) {
      return null
    }
  }

  function apply(mode) {
    if (mode === 'light' || mode === 'dark') {
      root.setAttribute('data-theme-mode', mode)
    } else {
      root.removeAttribute('data-theme-mode')
    }
  }

  apply(stored())

  document.querySelectorAll('[data-theme-toggle]').forEach(function (button) {
    button.addEventListener('click', function () {
      var current = root.getAttribute('data-theme-mode')
      var dark = !current
        ? window.matchMedia('(prefers-color-scheme: dark)').matches
        : current === 'dark'
      var next = dark ? 'light' : 'dark'
      apply(next)
      try {
        localStorage.setItem(key, next)
      } catch (error) {
        /* 隐私模式下忽略写入失败 */
      }
    })
  })

  var reveal = document.querySelectorAll('.reveal')
  root.classList.add('js')

  if (!('IntersectionObserver' in window) || window.matchMedia('(prefers-reduced-motion: reduce)').matches) {
    reveal.forEach(function (node) {
      node.classList.add('is-in')
    })
    return
  }

  var observer = new IntersectionObserver(
    function (entries) {
      entries.forEach(function (entry) {
        if (entry.isIntersecting) {
          entry.target.classList.add('is-in')
          observer.unobserve(entry.target)
        }
      })
    },
    { rootMargin: '0px 0px -8% 0px', threshold: 0.08 }
  )

  reveal.forEach(function (node, index) {
    node.style.transitionDelay = Math.min(index % 6, 4) * 60 + 'ms'
    observer.observe(node)
  })
})()
