import { useQuery } from '@tanstack/react-query'
import { BookOpen, CheckCircle2, ChevronRight, Lock, Pencil } from 'lucide-react'
import { Link } from 'react-router'
import { coursesApi } from '@/api/endpoints'
import { CourseSelect } from '@/components/course-select'
import { EmptyState, ErrorState, PageHeader, PageSkeleton } from '@/components/page'
import { PublishBadge, WeekStateBadge } from '@/components/status'
import { Button } from '@/components/ui/button'
import { Progress } from '@/components/ui/misc'
import { Tooltip } from '@/components/ui/tooltip'
import { useSelectedCourse } from '@/hooks/queries'
import { useAuth } from '@/lib/auth'
import { useBreadcrumbs } from '@/lib/breadcrumbs'
import { cn, formatPlainDate } from '@/lib/utils'
import type { Module, WeekState, WeekSummary } from '@/types/api'

const STATE_TILE: Record<WeekState, string> = {
  completed: 'bg-success/80 border-success',
  current: 'bg-primary border-primary ring-2 ring-primary/30',
  available: 'bg-muted-foreground/25 border-muted-foreground/30',
  locked: 'bg-transparent border-dashed border-muted-foreground/30',
}

function TimelineStrip({ weeks, admin }: { weeks: WeekSummary[]; admin: boolean }) {
  return (
    <div className="rounded-xl border bg-card p-4">
      <div className="mb-3 flex flex-wrap items-center justify-between gap-3">
        <h2 className="text-sm font-semibold">{weeks.length}-week timeline</h2>
        <div className="flex flex-wrap gap-x-4 gap-y-1 text-xs text-muted-foreground">
          {(['completed', 'current', 'available', 'locked'] as const).map((s) => (
            <span key={s} className="flex items-center gap-1.5">
              <span className={cn('size-2.5 rounded-sm border', STATE_TILE[s])} />
              {admin && s === 'locked' ? 'Draft' : s === 'locked' ? 'Locked' : s[0].toUpperCase() + s.slice(1)}
            </span>
          ))}
        </div>
      </div>
      <ol className="grid grid-cols-12 gap-1 sm:grid-cols-[repeat(24,minmax(0,1fr))] lg:grid-cols-[repeat(48,minmax(0,1fr))]">
        {weeks.map((w) => {
          const tile = <span className={cn('block h-6 rounded-[3px] border transition group-hover:opacity-80', STATE_TILE[w.state])} />
          const clickable = admin || w.state !== 'locked'
          return (
            <li key={w.id}>
              <Tooltip content={`Week ${w.week_number}: ${w.title}${w.state !== 'locked' ? ` · ${w.completion_percent}%` : ''}`}>
                {clickable ? (
                  <Link to={`/weeks/${w.id}`} className="group block rounded-[3px] focus-visible:ring-2 focus-visible:ring-ring/60 focus-visible:outline-none" aria-label={`Week ${w.week_number}`}>
                    {tile}
                  </Link>
                ) : (
                  <span className="block" aria-label={`Week ${w.week_number} (locked)`} tabIndex={0}>
                    {tile}
                  </span>
                )}
              </Tooltip>
            </li>
          )
        })}
      </ol>
    </div>
  )
}

function WeekRow({ w, admin }: { w: WeekSummary; admin: boolean }) {
  const locked = w.state === 'locked' && !admin
  const inner = (
    <>
      <div
        className={cn(
          'flex size-9 shrink-0 items-center justify-center rounded-lg border text-sm font-semibold tabular-nums',
          w.state === 'current' && 'border-primary bg-primary/10 text-primary',
          w.state === 'completed' && 'border-success/40 bg-success/10 text-success',
        )}
      >
        {w.state === 'completed' ? <CheckCircle2 className="size-4" /> : locked ? <Lock className="size-3.5 text-muted-foreground" /> : w.week_number}
      </div>
      <div className="min-w-0 flex-1">
        <div className="flex flex-wrap items-center gap-2">
          <span className={cn('truncate text-sm font-medium', locked && 'text-muted-foreground')}>
            Week {w.week_number} · {w.title}
          </span>
          {admin ? <PublishBadge status={w.status} /> : <WeekStateBadge state={w.state} />}
        </div>
        <div className="mt-0.5 flex flex-wrap gap-x-3 text-xs text-muted-foreground">
          {w.starts_on && <span>Starts {formatPlainDate(w.starts_on)}</span>}
          {admin ? (
            <span>
              {w.published_lesson_count}/{w.lesson_count} lessons published
            </span>
          ) : (
            !locked && (
              <span>
                {w.completed_lesson_count}/{w.published_lesson_count} lessons
              </span>
            )
          )}
          {w.assignment_count > 0 && <span>{w.assignment_count} assignments</span>}
          {w.available_from && <span>Opens {formatPlainDate(w.available_from)}</span>}
        </div>
      </div>
      {!admin && !locked && (
        <div className="hidden w-32 items-center gap-2 sm:flex">
          <Progress value={w.completion_percent} />
          <span className="w-9 text-right text-xs text-muted-foreground tabular-nums">{Math.round(w.completion_percent)}%</span>
        </div>
      )}
      {!locked && <ChevronRight className="size-4 text-muted-foreground" />}
    </>
  )
  const cls = 'flex items-center gap-3 px-4 py-3'
  return (
    <li>
      {locked ? (
        <div className={cn(cls, 'opacity-70')}>{inner}</div>
      ) : (
        <div className="flex items-center">
          <Link to={`/weeks/${w.id}`} className={cn(cls, 'flex-1 transition hover:bg-accent/60 focus-visible:bg-accent focus-visible:outline-none')}>
            {inner}
          </Link>
          {admin && (
            <Button asChild variant="ghost" size="icon-sm" className="mr-3" aria-label={`Edit week ${w.week_number}`}>
              <Link to={`/admin/weeks/${w.id}`}>
                <Pencil />
              </Link>
            </Button>
          )}
        </div>
      )}
    </li>
  )
}

export function CurriculumPage() {
  useBreadcrumbs([{ label: 'Curriculum' }])
  const { isAdmin } = useAuth()
  const { courses, course, select } = useSelectedCourse()
  const q = useQuery({ queryKey: ['curriculum', course?.id], queryFn: () => coursesApi.curriculum(course!.id), enabled: !!course })

  if (courses.isLoading || (course && q.isLoading)) return <PageSkeleton />
  if (!course) return <EmptyState icon={BookOpen} title="No courses available" description="You are not enrolled in a published course yet." />
  if (q.error || !q.data) return <ErrorState error={q.error} onRetry={() => q.refetch()} />

  const { weeks, modules, current_week } = q.data
  const groups: { module: Module | null; weeks: WeekSummary[] }[] = modules.map((m) => ({ module: m, weeks: weeks.filter((w) => w.module_id === m.id) }))
  const loose = weeks.filter((w) => !w.module_id || !modules.some((m) => m.id === w.module_id))
  if (loose.length) groups.push({ module: null, weeks: loose })

  return (
    <div className="space-y-6">
      <PageHeader
        eyebrow={course.title}
        title="Curriculum"
        description={current_week ? `Week ${current_week} of ${course.duration_weeks}.` : `${course.duration_weeks} weeks · ${course.days_per_week} instructional days per week.`}
        actions={
          <>
            <CourseSelect courses={courses.data ?? []} value={course.id} onChange={select} />
            {isAdmin && (
              <Button asChild variant="outline">
                <Link to={`/admin/courses/${course.id}?tab=curriculum`}>
                  <Pencil /> Manage
                </Link>
              </Button>
            )}
          </>
        }
      />
      {weeks.length === 0 ? (
        <EmptyState icon={BookOpen} title="No weeks yet" description={isAdmin ? 'Generate the week structure from Course Management.' : 'The curriculum has not been published yet.'} />
      ) : (
        <>
          <TimelineStrip weeks={weeks} admin={isAdmin} />
          <div className="space-y-6">
            {groups
              .filter((g) => g.weeks.length)
              .map((g, i) => (
                <section key={g.module?.id ?? 'none'} aria-labelledby={`module-${i}`}>
                  <div className="mb-2 flex items-baseline justify-between gap-3">
                    <h2 id={`module-${i}`} className="text-sm font-semibold">
                      {g.module ? g.module.title : 'Other weeks'}
                    </h2>
                    <span className="text-xs text-muted-foreground">
                      Weeks {g.weeks[0].week_number}–{g.weeks[g.weeks.length - 1].week_number}
                    </span>
                  </div>
                  {g.module?.description && <p className="mb-3 text-[13px] text-muted-foreground">{g.module.description}</p>}
                  <ul className="divide-y overflow-hidden rounded-xl border bg-card">
                    {g.weeks.map((w) => (
                      <WeekRow key={w.id} w={w} admin={isAdmin} />
                    ))}
                  </ul>
                </section>
              ))}
          </div>
        </>
      )}
    </div>
  )
}
