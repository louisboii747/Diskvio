import { AlertCircle, HardDrive, LoaderCircle } from 'lucide-react'
import { Button } from '@/components/ui/button'
export function ResourceState({
  kind,
  message,
  retry,
}: {
  kind: 'loading' | 'empty' | 'error'
  message?: string
  retry?: () => void
}) {
  const Icon = kind === 'error' ? AlertCircle : kind === 'loading' ? LoaderCircle : HardDrive
  return (
    <div className="resource-state" role={kind === 'error' ? 'alert' : 'status'}>
      <Icon size={28} />
      <h2>
        {kind === 'loading'
          ? 'Loading demonstration devices'
          : kind === 'empty'
            ? 'No demonstration devices'
            : 'Could not load data'}
      </h2>
      <p>
        {message ??
          (kind === 'empty'
            ? 'Refresh to reload the local sample devices.'
            : 'Reading local fixtures. No hardware scan is performed.')}
      </p>
      {retry && (
        <Button variant="outline" onClick={retry}>
          Try again
        </Button>
      )}
    </div>
  )
}
