import { useQuery } from '@tanstack/react-query'
import { ArrowRight, BookOpen, CalendarRange, CheckCircle2, Clock, Pencil, Target, User, Users } from 'lucide-react'
import { Link, useParams } from 'react-router'
import { coursesApi } from '@/api/endpoints'
import { Markdown } from '@/components/markdown'
import { ErrorState, PageSkeleton } from '@/components/page'
import { PublishBadge, WeekStateBadge } from '@/components/status'
import { Badge } from '@/components/ui/badge'
import { Button } from '@/components/ui/button'
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card'
import { Progress } from '@/components/ui/misc'
import { useAuth } from '@/lib/auth'
import { useBreadcrumbs } from '@/lib/breadcrumbs'
import { formatPlainDate } from '@/lib/utils'
import { CourseCover } from './CoursesPage'

export function CourseOverviewPage() {
  const { id = '' } = useParams()
  const { isAdmin } = useAuth()
  const course = useQuery({ queryKey: ['course', id], queryFn: () => coursesApi.get(id) })
  const curriculum = useQuery({ queryKey: ['curriculum', id], queryFn: () => coursesApi.curriculum(id) })
  useBreadcrumbs([{ label: 'My Courses', to: '/courses' }, { label: course.data?.title ?? 'Course' }])

  if (course.isLoading) return <PageSkeleton />
  if (course.error || !course.data) return <ErrorState error={course.error} onRetry={() => course.refetch()} />
  const c = course.data
  const weeks = curriculum.data?.weeks ?? []
  const modules = curriculum.data?.modules ?? []

  return (
    <div className="space-y-6">
      <div className="overflow-hidden rounded-xl border bg-card">
        <CourseCover course={c} className="h-40 sm:h-52" />
        <div className="space-y-4 p-5 sm:p-6">
          <div className="flex flex-wrap items-center gap-2">
            {isAdmin && <PublishBadge status={c.status} />}
            <Badge variant="outline" className="capitalize">
              {c.difficulty}
            </Badge>
          </div>
          <div className="flex flex-col gap-4 lg:flex-row lg:items-start lg:justify-between">
            <div className="max-w-3xl space-y-2">
              <h1 className="text-2xl font-semibold tracking-tight">{c.title}</h1>
              <div className="flex flex-wrap gap-x-5 gap-y-1 text-[13px] text-muted-foreground">
                <span className="flex items-center gap-1.5">
                  <CalendarRange className="size-4" /> {c.duration_weeks} weeks · {c.days_per_week} instructional days per week
                </span>
                {c.instructor_name && (
                  <span className="flex items-center gap-1.5">
                    <User className="size-4" /> {c.instructor_name}
                  </span>
                )}
                {isAdmin && (
                  <span className="flex items-center gap-1.5">
                    <Users className="size-4" /> {c.enrolled_count} enrolled
                  </span>
                )}
                {(c.start_date || c.end_date) && (
                  <span className="flex items-center gap-1.5">
                    <Clock className="size-4" /> {formatPlainDate(c.start_date)} – {formatPlainDate(c.end_date)}
                  </span>
                )}
              </div>
            </div>
            <div className="flex shrink-0 gap-2">
              {isAdmin && (
                <Button asChild variant="outline">
                  <Link to={`/admin/courses/${c.id}`}>
                    <Pencil /> Edit course
                  </Link>
                </Button>
              )}
              <Button asChild>
                <Link to={`/curriculum?course=${c.id}`}>
                  Open curriculum <ArrowRight />
                </Link>
              </Button>
            </div>
          </div>
          {c.completion_percent != null && (
            <div className="flex max-w-md items-center gap-3">
              <Progress value={c.completion_percent} />
              <span className="text-xs whitespace-nowrap text-muted-foreground tabular-nums">{c.completion_percent}% complete</span>
            </div>
          )}
        </div>
      </div>

      <div className="grid gap-6 lg:grid-cols-3">
        <Card className="lg:col-span-2">
          <CardHeader>
            <CardTitle>About this course</CardTitle>
          </CardHeader>
          <CardContent>{c.description ? <Markdown>{c.description}</Markdown> : <p className="text-sm text-muted-foreground">No description yet.</p>}</CardContent>
        </Card>
        <Card>
          <CardHeader>
            <CardTitle className="flex items-center gap-2">
              <Target className="size-4 text-primary" /> Learning objectives
            </CardTitle>
          </CardHeader>
          <CardContent>
            {c.objectives.length ? (
              <ul className="space-y-2.5">
                {c.objectives.map((o, i) => (
                  <li key={i} className="flex gap-2 text-sm">
                    <CheckCircle2 className="mt-0.5 size-4 shrink-0 text-success" />
                    {o}
                  </li>
                ))}
              </ul>
            ) : (
              <p className="text-sm text-muted-foreground">No objectives listed.</p>
            )}
          </CardContent>
        </Card>
      </div>

      {modules.length > 0 && (
        <Card>
          <CardHeader>
            <CardTitle className="flex items-center gap-2">
              <BookOpen className="size-4 text-primary" /> Curriculum modules
            </CardTitle>
          </CardHeader>
          <CardContent className="grid gap-3 md:grid-cols-2">
            {modules.map((m, i) => {
              const mw = weeks.filter((w) => w.module_id === m.id)
              const range = mw.length ? `Weeks ${mw[0].week_number}–${mw[mw.length - 1].week_number}` : 'No weeks'
              return (
                <div key={m.id} className="rounded-lg border bg-muted/20 p-4">
                  <div className="text-xs text-muted-foreground">
                    Module {i + 1} · {range}
                  </div>
                  <div className="mt-1 font-medium">{m.title}</div>
                  {m.description && <p className="mt-1 text-[13px] text-muted-foreground">{m.description}</p>}
                  {!isAdmin && mw.some((w) => w.state === 'current') && (
                    <div className="mt-2">
                      <WeekStateBadge state="current" />
                    </div>
                  )}
                </div>
              )
            })}
          </CardContent>
        </Card>
      )}
    </div>
  )
}
