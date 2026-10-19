import { useCallback, useEffect, useState } from 'react'

export type Resource<T> =
  { status: 'loading' } | { status: 'ready'; data: T } | { status: 'error'; message: string }
export function useResource<T>(loader: () => Promise<T>) {
  const [resource, setResource] = useState<Resource<T>>({ status: 'loading' })
  const [revision, setRevision] = useState(0)
  const reload = useCallback(() => {
    setResource({ status: 'loading' })
    setRevision((value) => value + 1)
  }, [])
  useEffect(() => {
    let active = true
    loader().then(
      (data) => {
        if (active) setResource({ status: 'ready', data })
      },
      (error: unknown) => {
        if (active)
          setResource({
            status: 'error',
            message: error instanceof Error ? error.message : String(error),
          })
      },
    )
    return () => {
      active = false
    }
  }, [loader, revision])
  return { resource, reload }
}
