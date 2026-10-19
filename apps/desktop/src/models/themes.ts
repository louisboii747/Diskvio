export const themes = [
  { id: 'system', name: 'System', description: 'Match your computer', scheme: 'light' },
  { id: 'light', name: 'Light', description: 'Clean and bright', scheme: 'light' },
  { id: 'dark', name: 'Dark', description: 'Soft charcoal', scheme: 'dark' },
  { id: 'ocean', name: 'Ocean', description: 'Deep blue', scheme: 'dark' },
  { id: 'forest', name: 'Forest', description: 'Calm green', scheme: 'dark' },
  { id: 'sand', name: 'Sand', description: 'Warm and light', scheme: 'light' },
] as const
export type Theme = (typeof themes)[number]['id']
export function isTheme(value: unknown): value is Theme {
  return themes.some((theme) => theme.id === value)
}
