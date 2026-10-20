import { AlertCircle, HardDrive, LoaderCircle } from 'lucide-react'
import { Button } from '@/components/ui/button'
import type { InventorySource } from '@/models/disk'
export function ResourceState({
  kind,
  message,
  retry,
  source = 'demo',
}: {
  kind: 'loading' | 'empty' | 'error'
  message?: string
  retry?: () => void
  source?: InventorySource
}) {
  const Icon = kind === 'error' ? AlertCircle : kind === 'loading' ? LoaderCircle : HardDrive
  return (
    <div className="resource-state" role={kind === 'error' ? 'alert' : 'status'}>
      <Icon size={28} />
      <h2>
        {kind === 'loading'
          ? source === 'demo'
            ? 'Loading sample drives'
            : 'Discovering drives'
          : kind === 'empty'
            ? source === 'demo'
              ? 'No sample drives to show'
              : 'No physical drives found'
            : source === 'demo'
              ? "We couldn't load the sample drives"
              : 'Live discovery failed'}
      </h2>
      <p>
        {kind === 'error'
          ? 'Try loading them again.'
          : kind === 'empty'
            ? source === 'demo'
              ? 'Reload the samples to try again.'
              : 'Connect a drive and refresh.'
            : 'Your drives stay untouched.'}
      </p>
      {message && <p className="error-detail">{message}</p>}
      {retry && (
        <Button variant="outline" onClick={retry}>
          Try again
        </Button>
      )}
    </div>
  )
}
