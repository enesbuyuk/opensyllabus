import { ArrowRight, CalendarRange, Users } from 'lucide-react'
import { Link } from 'react-router'
import { CourseCover } from '@/pages/CoursesPage'
import { cn } from '@/lib/utils'
import type { CourseListItem } from '@/types/api'

/** Course card for the dashboard: cover thumbnail, title and where the course stands this week. */
export function CourseBanner({ course, showStudents = false, className }: { course: CourseListItem; showStudents?: boolean; className?: string }) {
  const week = course.current_week
  return (
    <Link
      to={`/courses/${course.id}`}
      className={cn(
        'group flex flex-col gap-4 overflow-hidden rounded-xl border bg-card p-3 shadow-xs transition hover:border-primary/40 focus-visible:ring-2 focus-visible:ring-ring/50 focus-visible:outline-none sm:flex-row sm:items-center',
        className,
      )}
    >
      <div className="aspect-[40/21] w-full shrink-0 overflow-hidden rounded-lg sm:w-48">
        <CourseCover course={course} className="h-full" />
      </div>
      <div className="min-w-0 flex-1 px-1 pb-1 sm:px-0 sm:pb-0">
        <div className="text-xs font-semibold tracking-wide text-primary uppercase">
          {week ? `Week ${week} of ${course.duration_weeks}` : `${course.duration_weeks}-week course`}
        </div>
        <div className="mt-1 truncate text-lg font-semibold">{course.title}</div>
        <div className="mt-1 flex flex-wrap items-center gap-x-4 gap-y-1 text-[13px] text-muted-foreground">
          {showStudents && (
            <span className="inline-flex items-center gap-1.5">
              <Users className="size-3.5" /> {course.enrolled_count} {course.enrolled_count === 1 ? 'student' : 'students'}
            </span>
          )}
          <span className="inline-flex items-center gap-1.5">
            <CalendarRange className="size-3.5" /> {course.published_weeks} / {course.total_weeks} weeks published
          </span>
        </div>
      </div>
      <ArrowRight className="mr-2 hidden size-4 shrink-0 text-muted-foreground transition group-hover:translate-x-0.5 group-hover:text-primary sm:block" />
    </Link>
  )
}
