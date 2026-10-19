import { useEffect, useState } from 'react'
export type Theme = 'light' | 'dark' | 'system'
function readTheme(): Theme {
  try {
    const value = localStorage.getItem('diskvio-theme')
    return value === 'light' || value === 'dark' ? value : 'system'
  } catch {
    return 'system'
  }
}
export function useTheme() {
  const [theme, setTheme] = useState<Theme>(readTheme)
  useEffect(() => {
    const media = matchMedia('(prefers-color-scheme: dark)')
    const apply = () => {
      const dark = theme === 'dark' || (theme === 'system' && media.matches)
      document.documentElement.classList.toggle('dark', dark)
      document.documentElement.style.colorScheme = dark ? 'dark' : 'light'
    }
    apply()
    try {
      localStorage.setItem('diskvio-theme', theme)
    } catch {
      /* Session preference still works. */
    }
    media.addEventListener('change', apply)
    return () => media.removeEventListener('change', apply)
  }, [theme])
  return { theme, setTheme }
}
