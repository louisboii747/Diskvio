import { useEffect, useState } from 'react'
import { isTheme, themes, type Theme } from '@/models/themes'
function readTheme(): Theme {
  try {
    const value = localStorage.getItem('diskvio-theme')
    return isTheme(value) ? value : 'system'
  } catch {
    return 'system'
  }
}
export function useTheme() {
  const [theme, setTheme] = useState<Theme>(readTheme)
  useEffect(() => {
    const media = matchMedia('(prefers-color-scheme: dark)')
    const apply = () => {
      const resolved = theme === 'system' ? (media.matches ? 'dark' : 'light') : theme
      const dark = themes.find((option) => option.id === resolved)?.scheme === 'dark'
      document.documentElement.dataset.theme = resolved
      document.documentElement.classList.toggle('dark', dark)
      document.documentElement.style.colorScheme = dark ? 'dark' : 'light'
    }
    apply()
    try {
      localStorage.setItem('diskvio-theme', theme)
    } catch {
      /* Keep the choice for this session. */
    }
    if (theme === 'system') media.addEventListener('change', apply)
    return () => media.removeEventListener('change', apply)
  }, [theme])
  return { theme, setTheme }
}
