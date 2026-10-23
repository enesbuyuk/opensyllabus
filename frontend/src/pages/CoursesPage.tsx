import { CalendarRange, GraduationCap, Plus, Users } from 'lucide-react'
import { Link } from 'react-router'
import { fileUrl } from '@/api/client'
import { EmptyState, ErrorState, PageHeader, PageSkeleton } from '@/components/page'
import { PublishBadge } from '@/components/status'
import { Badge } from '@/components/ui/badge'
import { Button } from '@/components/ui/button'
import { Progress } from '@/components/ui/misc'
import { useCourses } from '@/hooks/queries'
import { useAuth } from '@/lib/auth'
import { useBreadcrumbs } from '@/lib/breadcrumbs'
import { formatPlainDate } from '@/lib/utils'
import type { CourseListItem } from '@/types/api'

export function CourseCover({ course, className = 'h-32' }: { course: CourseListItem; className?: string }) {
  return course.cover_file_id ? (
    <img src={fileUrl(course.cover_file_id)} alt="" className={`${className} w-full object-cover`} />
  ) : (
    <div className={`${className} relative w-full overflow-hidden bg-gradient-to-br from-primary/25 via-primary/5 to-transparent`}>
      <div className="absolute inset-0 opacity-40 [background-image:radial-gradient(var(--border)_1px,transparent_1px)] [background-size:14px_14px]" />
      <GraduationCap className="absolute right-4 bottom-3 size-8 text-primary/60" />
    </div>
  )
}

export function CoursesPage() {
  useBreadcrumbs([{ label: 'My Courses' }])
  const { isAdmin } = useAuth()
  const q = useCourses()
  if (q.isLoading) return <PageSkeleton rows={1} />
  if (q.error) return <ErrorState error={q.error} onRetry={() => q.refetch()} />
  const courses = q.data ?? []
  return (
    <>
      <PageHeader
        title={isAdmin ? 'Courses' : 'My Courses'}
        description={isAdmin ? 'All courses on the platform.' : 'Courses you are enrolled in.'}
        actions={
          isAdmin && (
            <Button asChild>
              <Link to="/admin/courses">
                <Plus /> Manage courses
              </Link>
            </Button>
          )
        }
      />
      {courses.length === 0 ? (
        <EmptyState icon={GraduationCap} title="No courses yet" description={isAdmin ? 'Create your first course in Course Management.' : 'You are not enrolled in any published course yet.'} />
      ) : (
        <div className="grid gap-5 sm:grid-cols-2 xl:grid-cols-3">
          {courses.map((c) => (
            <Link key={c.id} to={`/courses/${c.id}`} className="group overflow-hidden rounded-xl border bg-card shadow-xs transition hover:border-primary/40 hover:shadow-md focus-visible:ring-2 focus-visible:ring-ring/50 focus-visible:outline-none">
              <CourseCover course={c} />
              <div className="space-y-3 p-4">
                <div className="flex items-center gap-2">
                  {isAdmin && <PublishBadge status={c.status} />}
                  <Badge variant="outline" className="capitalize">
                    {c.difficulty}
                  </Badge>
                </div>
                <div>
                  <h2 className="font-semibold tracking-tight group-hover:text-primary">{c.title}</h2>
                  <p className="mt-1 line-clamp-2 text-[13px] text-muted-foreground">{c.description}</p>
                </div>
                <div className="flex flex-wrap gap-x-4 gap-y-1 text-xs text-muted-foreground">
                  <span className="flex items-center gap-1">
                    <CalendarRange className="size-3.5" /> {c.duration_weeks} weeks · {c.days_per_week} days/week
                  </span>
                  {isAdmin && (
                    <span className="flex items-center gap-1">
                      <Users className="size-3.5" /> {c.enrolled_count} students
                    </span>
                  )}
                  {c.start_date && <span>Starts {formatPlainDate(c.start_date)}</span>}
                </div>
                {c.completion_percent != null && (
                  <div className="flex items-center gap-3 pt-1">
                    <Progress value={c.completion_percent} />
                    <span className="text-xs text-muted-foreground tabular-nums">{c.completion_percent}%</span>
                  </div>
                )}
              </div>
            </Link>
          ))}
        </div>
      )}
    </>
  )
}
