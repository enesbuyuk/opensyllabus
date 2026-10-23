import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query'
import { ArrowLeft, ArrowRight, CheckCircle2, Clock, FlaskConical, Lock, Pencil, Target, Undo2 } from 'lucide-react'
import { Link, useParams } from 'react-router'
import { toast } from 'sonner'
import { curriculumApi } from '@/api/endpoints'
import { AssignmentRows } from '@/components/assignment-list'
import { Markdown } from '@/components/markdown'
import { ErrorState, PageSkeleton, Section } from '@/components/page'
import { ResourceList } from '@/components/resource-list'
import { PublishBadge } from '@/components/status'
import { Button } from '@/components/ui/button'
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card'
import { VideoEmbed } from '@/components/video-embed'
import { useAuth } from '@/lib/auth'
import { useBreadcrumbs } from '@/lib/breadcrumbs'
import { errorMessage } from '@/lib/errors'
import { formatDateTime, formatMinutes } from '@/lib/utils'

export function LessonPage() {
  const { id = '' } = useParams()
  const { isAdmin, timezone } = useAuth()
  const qc = useQueryClient()
  const q = useQuery({ queryKey: ['lesson', id], queryFn: () => curriculumApi.lesson(id) })
  const l = q.data
  useBreadcrumbs([
    { label: 'Curriculum', to: '/curriculum' },
    ...(l ? [{ label: `Week ${l.week_number}`, to: `/weeks/${l.week_id}` }] : []),
    { label: l?.title ?? 'Lesson' },
  ])

  const toggle = useMutation({
    mutationFn: () => (l?.completed ? curriculumApi.uncomplete(id) : curriculumApi.complete(id)),
    onSuccess: () => {
      toast.success(l?.completed ? 'Marked as not completed' : 'Lesson completed. Nice work!')
      qc.invalidateQueries({ queryKey: ['lesson', id] })
      qc.invalidateQueries({ queryKey: ['week'] })
      qc.invalidateQueries({ queryKey: ['curriculum'] })
      qc.invalidateQueries({ queryKey: ['dashboard'] })
      qc.invalidateQueries({ queryKey: ['progress'] })
    },
    onError: (e) => toast.error(errorMessage(e)),
  })

  if (q.isLoading) return <PageSkeleton />
  if (q.error || !l) return <ErrorState error={q.error} onRetry={() => q.refetch()} />

  return (
    <div className="mx-auto max-w-4xl space-y-8">
      {isAdmin && l.status === 'draft' && (
        <div className="flex items-center justify-between gap-3 rounded-lg border border-warning/30 bg-warning/10 px-4 py-2.5 text-[13px]">
          <span>Preview: this lesson is a draft and hidden from students.</span>
          <Button asChild size="sm" variant="outline">
            <Link to={`/admin/lessons/${l.id}`}>Edit lesson</Link>
          </Button>
        </div>
      )}
      <header className="space-y-3">
        <div className="flex flex-wrap items-center gap-2 text-xs text-muted-foreground">
          <Link to={`/weeks/${l.week_id}`} className="hover:text-foreground">
            Week {l.week_number} · {l.week_title}
          </Link>
          <span>·</span>
          <span>Day {l.day_number}</span>
          {isAdmin && <PublishBadge status={l.status} />}
        </div>
        <h1 className="text-2xl font-semibold tracking-tight text-balance sm:text-3xl">{l.title}</h1>
        {l.summary && <p className="text-muted-foreground">{l.summary}</p>}
        <div className="flex flex-wrap items-center gap-3 pt-1">
          <span className="flex items-center gap-1 text-[13px] text-muted-foreground">
            <Clock className="size-3.5" /> {formatMinutes(l.estimated_minutes)}
          </span>
          {isAdmin && (
            <Button asChild variant="outline" size="sm">
              <Link to={`/admin/lessons/${l.id}`}>
                <Pencil /> Edit
              </Link>
            </Button>
          )}
        </div>
      </header>

      {!l.prerequisites_met ? (
        <Card className="border-warning/30">
          <CardHeader>
            <CardTitle className="flex items-center gap-2">
              <Lock className="size-4 text-warning" /> Complete the prerequisites first
            </CardTitle>
          </CardHeader>
          <CardContent>
            <ul className="space-y-2">
              {l.prerequisites.map((p) => (
                <li key={p.id}>
                  <Link to={`/lessons/${p.id}`} className="flex items-center gap-2 text-sm hover:text-primary">
                    {p.completed ? <CheckCircle2 className="size-4 text-success" /> : <span className="size-4 rounded-full border" />}
                    Week {p.week_number}, Day {p.day_number}: {p.title}
                  </Link>
                </li>
              ))}
            </ul>
          </CardContent>
        </Card>
      ) : (
        <>
          {l.objectives.length > 0 && (
            <div className="rounded-xl border bg-muted/20 p-4">
              <div className="mb-2 flex items-center gap-2 text-sm font-semibold">
                <Target className="size-4 text-primary" /> In this lesson you will
              </div>
              <ul className="grid gap-1.5 sm:grid-cols-2">
                {l.objectives.map((o, i) => (
                  <li key={i} className="flex gap-2 text-sm">
                    <span className="mt-2 size-1.5 shrink-0 rounded-full bg-primary" />
                    {o}
                  </li>
                ))}
              </ul>
            </div>
          )}
          {l.video_url && <VideoEmbed url={l.video_url} title={l.title} />}
          {l.content_md ? <Markdown>{l.content_md}</Markdown> : <p className="text-sm text-muted-foreground">No written content for this lesson.</p>}
          {l.lab_md && (
            <Card className="border-chart-2/30">
              <CardHeader>
                <CardTitle className="flex items-center gap-2">
                  <FlaskConical className="size-4 text-chart-2" /> Laboratory instructions
                </CardTitle>
              </CardHeader>
              <CardContent>
                <Markdown>{l.lab_md}</Markdown>
              </CardContent>
            </Card>
          )}
          {l.resources.length > 0 && (
            <Section title="Lesson resources">
              <ResourceList resources={l.resources} />
            </Section>
          )}
          {l.assignments.length > 0 && (
            <Section title="Exercises and assignments">
              <AssignmentRows items={l.assignments} />
            </Section>
          )}
        </>
      )}

      {!isAdmin && l.prerequisites_met && (
        <div className="flex flex-col items-center gap-3 rounded-xl border bg-card p-6 text-center">
          {l.completed ? (
            <>
              <CheckCircle2 className="size-6 text-success" />
              <p className="text-sm">You completed this lesson {l.completed_at ? `on ${formatDateTime(l.completed_at, timezone)}` : ''}.</p>
              <Button variant="ghost" size="sm" onClick={() => toggle.mutate()} loading={toggle.isPending}>
                <Undo2 /> Mark as not completed
              </Button>
            </>
          ) : (
            <>
              <p className="text-sm text-muted-foreground">Finished working through the material?</p>
              <Button onClick={() => toggle.mutate()} loading={toggle.isPending}>
                <CheckCircle2 /> Mark lesson as completed
              </Button>
            </>
          )}
        </div>
      )}

      <nav className="flex items-center justify-between border-t pt-6" aria-label="Lesson navigation">
        {l.previous_lesson_id ? (
          <Button asChild variant="ghost">
            <Link to={`/lessons/${l.previous_lesson_id}`}>
              <ArrowLeft /> Previous lesson
            </Link>
          </Button>
        ) : (
          <span />
        )}
        {l.next_lesson_id && (
          <Button asChild variant="ghost">
            <Link to={`/lessons/${l.next_lesson_id}`}>
              Next lesson <ArrowRight />
            </Link>
          </Button>
        )}
      </nav>
    </div>
  )
}
