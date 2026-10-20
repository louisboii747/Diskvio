import { FlaskConical } from 'lucide-react'
import type { InventorySource } from '@/models/disk'
export function DemoNotice({ source }: { source: InventorySource }) {
  return (
    <div className="demo-notice">
      <FlaskConical size={16} aria-hidden="true" />
      <strong>{source === 'demo' ? 'DEMO DATA' : 'LIVE · READ ONLY'}</strong>
      <span>
        {source === 'demo'
          ? 'These are sample drives. Your files stay untouched.'
          : 'Actual devices. Disk changes are disabled.'}
      </span>
    </div>
  )
}
