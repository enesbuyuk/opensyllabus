// Applies the saved theme before first paint (external file: the CSP forbids inline scripts).
;(function () {
  try {
    var t = localStorage.getItem('opensyllabus-theme') || 'dark'
    var dark = t === 'dark' || (t === 'system' && window.matchMedia('(prefers-color-scheme: dark)').matches)
    document.documentElement.classList.toggle('dark', dark)
  } catch (e) {}
})()
