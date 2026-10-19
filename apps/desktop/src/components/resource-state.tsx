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
          ? 'Loading sample drives'
          : kind === 'empty'
            ? 'No sample drives to show'
            : "We couldn't load the sample drives"}
      </h2>
      <p>
        {kind === 'error'
          ? 'Try loading them again.'
          : kind === 'empty'
            ? 'Reload the samples to try again.'
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
