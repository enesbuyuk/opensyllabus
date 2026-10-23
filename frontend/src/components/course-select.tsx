import { NativeSelect } from './ui/input'
import type { CourseListItem } from '@/types/api'

export function CourseSelect({ courses, value, onChange, allowAll, className }: { courses: CourseListItem[]; value?: string; onChange: (id: string) => void; allowAll?: boolean; className?: string }) {
  if (courses.length <= 1 && !allowAll) return null
  return (
    <NativeSelect value={value ?? ''} onChange={(e) => onChange(e.target.value)} className={className ?? 'w-full sm:w-64'} aria-label="Course">
      {allowAll && <option value="">All courses</option>}
      {courses.map((c) => (
        <option key={c.id} value={c.id}>
          {c.title}
        </option>
      ))}
    </NativeSelect>
  )
}
