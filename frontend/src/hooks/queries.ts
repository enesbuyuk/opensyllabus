import { useQuery } from '@tanstack/react-query'
import * as React from 'react'
import { useSearchParams } from 'react-router'
import { coursesApi, miscApi } from '@/api/endpoints'

export function useCourses() {
  return useQuery({ queryKey: ['courses'], queryFn: coursesApi.list })
}

export function usePlatformSettings() {
  return useQuery({ queryKey: ['settings'], queryFn: miscApi.settings, staleTime: 10 * 60_000 })
}

const COURSE_KEY = 'opensyllabus-course'

/**
 * Currently selected course: `?course=` in the URL, else the last used one, else the first visible course.
 */
export function useSelectedCourse() {
  const courses = useCourses()
  const [params, setParams] = useSearchParams()
  const fromUrl = params.get('course')
  let stored: string | null = null
  try {
    stored = localStorage.getItem(COURSE_KEY)
  } catch {
    /* ignore */
  }
  const list = courses.data ?? []
  const course = list.find((c) => c.id === fromUrl) ?? list.find((c) => c.id === stored) ?? list.find((c) => c.status === 'published') ?? list[0]

  const select = React.useCallback(
    (id: string) => {
      try {
        localStorage.setItem(COURSE_KEY, id)
      } catch {
        /* ignore */
      }
      setParams(
        (p) => {
          const next = new URLSearchParams(p)
          next.set('course', id)
          return next
        },
        { replace: true },
      )
    },
    [setParams],
  )
  return { courses, course, select }
}

export function useDebounced<T>(value: T, ms = 250) {
  const [v, setV] = React.useState(value)
  React.useEffect(() => {
    const t = setTimeout(() => setV(value), ms)
    return () => clearTimeout(t)
  }, [value, ms])
  return v
}
