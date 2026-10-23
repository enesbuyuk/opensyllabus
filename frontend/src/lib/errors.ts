import type { FieldValues, Path, UseFormSetError } from 'react-hook-form'
import { toast } from 'sonner'
import { ApiError } from '@/api/client'

export function errorMessage(e: unknown) {
  if (e instanceof ApiError) return e.message
  if (e instanceof Error) return e.message
  return 'Something went wrong'
}

/** Shows an API error as a toast and maps field errors into the form when possible. */
export function handleFormError<T extends FieldValues>(e: unknown, setError?: UseFormSetError<T>) {
  if (e instanceof ApiError && Object.keys(e.fields).length && setError) {
    for (const [field, message] of Object.entries(e.fields)) setError(field as Path<T>, { message })
    toast.error(e.message)
    return
  }
  toast.error(errorMessage(e))
}
