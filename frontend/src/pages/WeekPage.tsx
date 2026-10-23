import { useQuery } from '@tanstack/react-query'
import { ArrowLeft, ArrowRight, CheckCircle2, Circle, Clock, Lock, NotebookPen, Pencil, Target } from 'lucide-react'
import { Link, useParams } from 'react-router'
import { curriculumApi } from '@/api/endpoints'
import { AssignmentRows } from '@/components/assignment-list'
import { Markdown } from '@/components/markdown'
import { ErrorState, PageSkeleton, Section } from '@/components/page'
import { ResourceList } from '@/components/resource-list'
import { PublishBadge, WeekStateBadge } from '@/components/status'
import { Badge } from '@/components/ui/badge'
import { Button } from '@/components/ui/button'
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card'
import { Progress } from '@/components/ui/misc'
import { useAuth } from '@/lib/auth'
import { useBreadcrumbs } from '@/lib/breadcrumbs'
import { cn, formatMinutes, formatPlainDate } from '@/lib/utils'

export function WeekPage() {
  const { id = '' } = useParams()
  const { isAdmin } = useAuth()
  const q = useQuery({ queryKey: ['week', id], queryFn: () => curriculumApi.week(id) })
  useBreadcrumbs([{ label: 'Curriculum', to: '/curriculum' }, { label: q.data ? `Week ${q.data.week_number}` : 'Week' }])

  if (q.isLoading) return <PageSkeleton />
  if (q.error || !q.data) return <ErrorState error={q.error} onRetry={() => q.refetch()} />
  const w = q.data
  const days = Array.from(new Set([1, 2, 3, ...w.lessons.map((l) => l.day_number)])).sort((a, b) => a - b)
  const totalMinutes = w.estimated_minutes || w.lessons.reduce((s, l) => s + l.estimated_minutes, 0)

  return (
    <div className="space-y-8">
      {isAdmin && w.status === 'draft' && (
        <div className="flex items-center justify-between gap-3 rounded-lg border border-warning/30 bg-warning/10 px-4 py-2.5 text-[13px]">
          <span>This week is a draft and hidden from students.</span>
          <Button asChild size="sm" variant="outline">
            <Link to={`/admin/weeks/${w.id}`}>Edit & publish</Link>
          </Button>
        </div>
      )}
      <div className="flex flex-col gap-4 lg:flex-row lg:items-end lg:justify-between">
        <div className="space-y-2">
          <div className="flex flex-wrap items-center gap-2 text-xs text-muted-foreground">
            {w.module && <span>{w.module.title}</span>}
            {isAdmin ? <PublishBadge status={w.status} /> : <WeekStateBadge state={w.state} />}
          </div>
          <h1 className="text-2xl font-semibold tracking-tight">
            <span className="text-muted-foreground">Week {w.week_number} · </span>
            {w.title}
          </h1>
          <div className="flex flex-wrap gap-x-4 gap-y-1 text-[13px] text-muted-foreground">
            {w.starts_on && <span>Starts {formatPlainDate(w.starts_on)}</span>}
            <span className="flex items-center gap-1">
              <Clock className="size-3.5" /> Estimated {formatMinutes(totalMinutes)}
            </span>
            <span>{w.lessons.length} lessons</span>
          </div>
        </div>
        <div className="flex items-center gap-2">
          {!isAdmin && (
            <div className="flex w-48 items-center gap-2">
              <Progress value={w.completion_percent} />
              <span className="text-xs text-muted-foreground tabular-nums">{w.completion_percent}%</span>
            </div>
          )}
          {isAdmin && (
            <Button asChild variant="outline">
              <Link to={`/admin/weeks/${w.id}`}>
                <Pencil /> Edit week
              </Link>
            </Button>
          )}
        </div>
      </div>

      {(w.description || w.objectives.length > 0) && (
        <div className="grid gap-6 lg:grid-cols-3">
          <div className="lg:col-span-2">{w.description ? <Markdown>{w.description}</Markdown> : null}</div>
          {w.objectives.length > 0 && (
            <Card>
              <CardHeader>
                <CardTitle className="flex items-center gap-2">
                  <Target className="size-4 text-primary" /> Learning objectives
                </CardTitle>
              </CardHeader>
              <CardContent>
                <ul className="space-y-2">
                  {w.objectives.map((o, i) => (
                    <li key={i} className="flex gap-2 text-sm">
                      <span className="mt-2 size-1.5 shrink-0 rounded-full bg-primary" />
                      {o}
                    </li>
                  ))}
                </ul>
              </CardContent>
            </Card>
          )}
        </div>
      )}

      <Section title="Lessons" description={`${days.length} instructional days`}>
        {w.lessons.length === 0 ? (
          <p className="text-[13px] text-muted-foreground">No lessons have been published for this week yet.</p>
        ) : (
          <div className="grid gap-4 md:grid-cols-3">
            {w.lessons.map((l) => (
              <Link
                key={l.id}
                to={`/lessons/${l.id}`}
                className={cn(
                  'group flex flex-col rounded-xl border bg-card p-4 shadow-xs transition hover:border-primary/40 hover:shadow-md focus-visible:ring-2 focus-visible:ring-ring/50 focus-visible:outline-none',
                  l.completed && 'border-success/30',
                )}
              >
                <div className="flex items-center justify-between text-xs text-muted-foreground">
                  <span className="font-medium">Day {l.day_number}</span>
                  {isAdmin ? (
                    <PublishBadge status={l.status} />
                  ) : l.completed ? (
                    <span className="flex items-center gap-1 text-success">
                      <CheckCircle2 className="size-4" /> Completed
                    </span>
                  ) : l.locked_by_prerequisites ? (
                    <span className="flex items-center gap-1">
                      <Lock className="size-3.5" /> Prerequisites
                    </span>
                  ) : (
                    <Circle className="size-4" />
                  )}
                </div>
                <h3 className="mt-3 font-medium group-hover:text-primary">{l.title}</h3>
                {l.summary && <p className="mt-1 line-clamp-3 text-[13px] text-muted-foreground">{l.summary}</p>}
                <div className="mt-auto flex items-center gap-3 pt-4 text-xs text-muted-foreground">
                  <span className="flex items-center gap-1">
                    <Clock className="size-3.5" /> {formatMinutes(l.estimated_minutes)}
                  </span>
                  {l.video_url && <Badge variant="outline">Video</Badge>}
                </div>
              </Link>
            ))}
          </div>
        )}
      </Section>

      <div className="grid gap-8 lg:grid-cols-2">
        <Section title="Assignments">
          <AssignmentRows items={w.assignments} emptyText="No assignments for this week." />
        </Section>
        <Section title="Resources">
          <ResourceList resources={w.resources} emptyText="No downloadable resources for this week." />
        </Section>
      </div>

      {w.instructor_notes && (
        <Card className="border-primary/20 bg-primary/5">
          <CardHeader>
            <CardTitle className="flex items-center gap-2">
              <NotebookPen className="size-4 text-primary" /> Instructor notes
            </CardTitle>
          </CardHeader>
          <CardContent>
            <Markdown>{w.instructor_notes}</Markdown>
          </CardContent>
        </Card>
      )}

      <div className="flex items-center justify-between border-t pt-6">
        {w.previous_week_id ? (
          <Button asChild variant="ghost">
            <Link to={`/weeks/${w.previous_week_id}`}>
              <ArrowLeft /> Previous week
            </Link>
          </Button>
        ) : (
          <span />
        )}
        {w.next_week_id && (
          <Button asChild variant="ghost">
            <Link to={`/weeks/${w.next_week_id}`}>
              Next week <ArrowRight />
            </Link>
          </Button>
        )}
      </div>
    </div>
  )
}
