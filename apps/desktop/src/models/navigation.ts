export type Page = 'overview' | 'partitions' | 'usb' | 'settings'
export const pageTitles: Record<Page, string> = {
  overview: 'Drives',
  partitions: 'Partitions',
  usb: 'USB Writer',
  settings: 'Settings',
}
