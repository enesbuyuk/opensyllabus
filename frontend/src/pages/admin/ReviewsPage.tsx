import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query'
import { ArrowLeft, ClipboardCheck, ExternalLink, Inbox } from 'lucide-react'
import * as React from 'react'
import { Link, useSearchParams } from 'react-router'
import { assignmentsApi, coursesApi, submissionsApi, usersApi } from '@/api/endpoints'
import { EmptyState, ErrorState, PageHeader } from '@/components/page'
import { SubmissionStatusBadge } from '@/components/status'
import { Badge } from '@/components/ui/badge'
import { Button } from '@/components/ui/button'
import { NativeSelect } from '@/components/ui/input'
import { Avatar, Skeleton } from '@/components/ui/misc'
import { Tabs, TabsContent, TabsList, TabsTrigger } from '@/components/ui/tabs'
import { FeedbackPanel } from '@/features/assignments/FeedbackPanel'
import { GroupGradePanel } from '@/features/assignments/GroupGradePanel'
import { ReviewPanel } from '@/features/assignments/ReviewPanel'
import { SubmissionContent } from '@/features/assignments/SubmissionContent'
import { useCourses } from '@/hooks/queries'
import { useAuth } from '@/lib/auth'
import { useBreadcrumbs } from '@/lib/breadcrumbs'
import { ASSIGNMENT_KIND_LABELS } from '@/lib/labels'
import { cn, formatDateTime, formatScore, relativeTime } from '@/lib/utils'

const FILTERS = ['course', 'week', 'assignment', 'student', 'status'] as const

function SubmissionView({ id, onBack, onSelect }: { id: string; onBack: () => void; onSelect: (id: string) => void }) {
  const { timezone } = useAuth()
  const qc = useQueryClient()
  const q = useQuery({ queryKey: ['submission', id], queryFn: () => submissionsApi.get(id) })
  const start = useMutation({
    mutationFn: () => submissionsApi.startReview(id),
    onSuccess: () => {
      qc.invalidateQueries({ queryKey: ['submission', id] })
      qc.invalidateQueries({ queryKey: ['submissions'] })
    },
  })
  React.useEffect(() => {
    if (q.data?.status === 'submitted' && !start.isPending && !start.isSuccess) start.mutate()
  }, [q.data?.status, start])

  if (q.isLoading) return <Skeleton className="h-[60vh] rounded-xl" />
  if (q.error || !q.data) return <ErrorState error={q.error} onRetry={() => q.refetch()} />
  const d = q.data
  const latest = d.revisions[0]

  return (
    <div className="space-y-4">
      <div className="flex flex-wrap items-start justify-between gap-3">
        <div className="flex min-w-0 items-start gap-3">
          <Button variant="ghost" size="icon-sm" className="lg:hidden" onClick={onBack} aria-label="Back to list">
            <ArrowLeft />
          </Button>
          <Avatar name={d.student.name} className="size-10" />
          <div className="min-w-0">
            <div className="font-semibold">
              {d.group ? (
                <>
                  {d.group.name} <span className="font-normal text-muted-foreground">({d.group.members.map((m) => m.name).join(', ')})</span>
                </>
              ) : (
                d.student.name
              )}
            </div>
            <Link to={`/assignments/${d.assignment.id}`} className="flex items-center gap-1 truncate text-[13px] text-muted-foreground hover:text-foreground">
              {d.assignment.title} <ExternalLink className="size-3" />
            </Link>
            <div className="mt-1 text-xs text-muted-foreground">
              {d.course_title}
              {d.week_number ? ` · Week ${d.week_number}` : ''} · {ASSIGNMENT_KIND_LABELS[d.assignment.kind]}
            </div>
          </div>
        </div>
        <div className="flex flex-col items-end gap-1">
          <SubmissionStatusBadge status={d.status} late={d.is_late} />
          <span className="text-xs text-muted-foreground">
            Attempt {d.attempt} · {formatDateTime(d.submitted_at, timezone)}
          </span>
          {d.is_late && <Badge variant="warning">Submitted after deadline</Badge>}
        </div>
      </div>
      <div className="grid gap-4 2xl:grid-cols-2">
        <div className="rounded-xl border bg-card">
          <Tabs defaultValue="current">
            <div className="flex items-center justify-between border-b px-4">
              <TabsList className="border-b-0">
                <TabsTrigger value="current">Submission</TabsTrigger>
                <TabsTrigger value="history">History ({d.revisions.length})</TabsTrigger>
                <TabsTrigger value="feedback">Feedback ({d.feedback.length})</TabsTrigger>
              </TabsList>
            </div>
            <TabsContent value="current" className="max-h-[70vh] overflow-y-auto p-4">
              {latest ? <SubmissionContent body={latest.body_md} labReport={latest.lab_report} repositoryUrl={latest.repository_url} attachments={latest.attachments} /> : <p className="text-sm text-muted-foreground">Nothing submitted yet.</p>}
            </TabsContent>
            <TabsContent value="history" className="max-h-[70vh] space-y-2 overflow-y-auto p-4">
              {d.revisions.map((r) => (
                <details key={r.id} className="rounded-lg border" open={r.attempt === d.attempt}>
                  <summary className="flex cursor-pointer items-center justify-between px-3 py-2 text-sm select-none">
                    <span className="font-medium">Attempt {r.attempt}</span>
                    <span className="text-xs text-muted-foreground">
                      {formatDateTime(r.submitted_at, timezone)} {r.is_late && <span className="text-warning">· late</span>}
                    </span>
                  </summary>
                  <div className="border-t p-3">
                    <SubmissionContent body={r.body_md} labReport={r.lab_report} repositoryUrl={r.repository_url} attachments={r.attachments} />
                  </div>
                </details>
              ))}
            </TabsContent>
            <TabsContent value="feedback" className="max-h-[70vh] overflow-y-auto p-4">
              {d.feedback.length || d.grade ? <FeedbackPanel detail={d} /> : <p className="text-sm text-muted-foreground">No feedback yet.</p>}
            </TabsContent>
          </Tabs>
        </div>
        <div className="rounded-xl border bg-card p-4">
          <div className="mb-3 flex items-center justify-between">
            <h3 className="text-sm font-semibold">Review</h3>
            {d.grade && (
              <span className="text-xs text-muted-foreground">
                {d.grade.released_at ? 'Returned' : 'Draft'} grade {formatScore(d.grade.score, d.grade.max_score)}
              </span>
            )}
          </div>
          {d.mirror_of ? (
            <div className="space-y-3 text-sm text-muted-foreground">
              <p>This is {d.student.name}'s copy of the group submission. Grade the group from the leader's submission.</p>
              <Button variant="outline" size="sm" onClick={() => onSelect(d.mirror_of!)}>
                Open group submission
              </Button>
            </div>
          ) : (
            <div className="space-y-6">
              <ReviewPanel detail={d} />
              <GroupGradePanel detail={d} />
            </div>
          )}
        </div>
      </div>
    </div>
  )
}

export function ReviewsPage() {
  useBreadcrumbs([{ label: 'Submission Reviews' }])
  const [params, setParams] = useSearchParams()
  const courses = useCourses()
  const f = Object.fromEntries(FILTERS.map((k) => [k, params.get(k) ?? ''])) as Record<(typeof FILTERS)[number], string>
  const status = f.status || (params.has('status') ? '' : 'pending')
  const selected = params.get('submission')
  const curriculum = useQuery({ queryKey: ['curriculum', f.course], queryFn: () => coursesApi.curriculum(f.course), enabled: !!f.course })
  const assignments = useQuery({ queryKey: ['assignments', f.course, 'review'], queryFn: () => assignmentsApi.list({ course_id: f.course || undefined }) })
  const students = useQuery({ queryKey: ['students'], queryFn: () => usersApi.list({ role: 'student' }) })
  const list = useQuery({
    queryKey: ['submissions', f.course, f.week, f.assignment, f.student, status],
    queryFn: () => submissionsApi.list({ course_id: f.course || undefined, week_id: f.week || undefined, assignment_id: f.assignment || undefined, student_id: f.student || undefined, status: status || undefined }),
  })

  const setFilter = (k: string, v: string) =>
    setParams(
      (p) => {
        const n = new URLSearchParams(p)
        if (k === 'status') n.set('status', v)
        else if (v) n.set(k, v)
        else n.delete(k)
        if (k === 'course') {
          n.delete('week')
          n.delete('assignment')
        }
        return n
      },
      { replace: true },
    )
  const select = (id: string | null) =>
    setParams((p) => {
      const n = new URLSearchParams(p)
      if (id) n.set('submission', id)
      else n.delete('submission')
      return n
    })

  const items = list.data ?? []
  return (
    <>
      <PageHeader title="Submission Reviews" description="Read submissions, grade with rubrics and return feedback." />
      <div className="mb-4 grid gap-2 sm:grid-cols-3 lg:grid-cols-5">
        <NativeSelect value={f.course} onChange={(e) => setFilter('course', e.target.value)} aria-label="Course">
          <option value="">All courses</option>
          {courses.data?.map((c) => (
            <option key={c.id} value={c.id}>
              {c.title}
            </option>
          ))}
        </NativeSelect>
        <NativeSelect value={f.week} onChange={(e) => setFilter('week', e.target.value)} disabled={!f.course} aria-label="Week">
          <option value="">All weeks</option>
          {curriculum.data?.weeks.map((w) => (
            <option key={w.id} value={w.id}>
              Week {w.week_number}
            </option>
          ))}
        </NativeSelect>
        <NativeSelect value={f.assignment} onChange={(e) => setFilter('assignment', e.target.value)} aria-label="Assignment">
          <option value="">All assignments</option>
          {assignments.data
            ?.filter((a) => !f.week || a.week_id === f.week)
            .map((a) => (
              <option key={a.id} value={a.id}>
                {a.title}
              </option>
            ))}
        </NativeSelect>
        <NativeSelect value={f.student} onChange={(e) => setFilter('student', e.target.value)} aria-label="Student">
          <option value="">All students</option>
          {students.data?.map((s) => (
            <option key={s.id} value={s.id}>
              {s.name}
            </option>
          ))}
        </NativeSelect>
        <NativeSelect value={status} onChange={(e) => setFilter('status', e.target.value)} aria-label="Status">
          <option value="pending">Needs review</option>
          <option value="">All statuses</option>
          <option value="submitted">Submitted</option>
          <option value="under_review">Under review</option>
          <option value="revision_requested">Revision requested</option>
          <option value="graded">Graded</option>
          <option value="late">Late</option>
        </NativeSelect>
      </div>
      <div className="grid gap-4 lg:grid-cols-[22rem_1fr]">
        <div className={cn('lg:block', selected && 'hidden')}>
          {list.isLoading ? (
            <Skeleton className="h-96 rounded-xl" />
          ) : list.error ? (
            <ErrorState error={list.error} onRetry={() => list.refetch()} />
          ) : items.length === 0 ? (
            <EmptyState icon={Inbox} title="No submissions" description={status === 'pending' ? 'Nothing waiting for review.' : 'No submissions match the filters.'} />
          ) : (
            <ul className="max-h-[75vh] divide-y overflow-y-auto rounded-xl border bg-card" aria-label="Submissions">
              {items.map((s) => (
                <li key={s.id}>
                  <button
                    type="button"
                    onClick={() => select(s.id)}
                    aria-current={selected === s.id}
                    className={cn('flex w-full items-start gap-3 px-3 py-3 text-left transition hover:bg-accent/60 focus-visible:bg-accent focus-visible:outline-none', selected === s.id && 'bg-accent')}
                  >
                    <Avatar name={s.student_name} className="size-7 text-[10px]" />
                    <div className="min-w-0 flex-1">
                      <div className="flex items-center justify-between gap-2">
                        <span className="truncate text-sm font-medium">{s.group_name ? `${s.group_name} (${s.student_name})` : s.student_name}</span>
                        <span className="shrink-0 text-[11px] text-muted-foreground">{relativeTime(s.submitted_at)}</span>
                      </div>
                      <div className="truncate text-xs text-muted-foreground">
                        {s.week_number ? `W${s.week_number} · ` : ''}
                        {s.assignment_title}
                      </div>
                      <div className="mt-1.5 flex items-center gap-2">
                        <SubmissionStatusBadge status={s.status} late={s.is_late} />
                        {s.attempt > 1 && <Badge variant="outline">Attempt {s.attempt}</Badge>}
                        {s.score != null && <span className="ml-auto text-xs tabular-nums">{formatScore(s.score, s.max_grade)}</span>}
                      </div>
                    </div>
                  </button>
                </li>
              ))}
            </ul>
          )}
        </div>
        <div className={cn(!selected && 'hidden lg:block')}>
          {selected ? (
            <SubmissionView key={selected} id={selected} onBack={() => select(null)} onSelect={select} />
          ) : (
            <EmptyState icon={ClipboardCheck} title="Select a submission" description="Choose a submission from the list to read it and enter feedback." className="h-full min-h-80" />
          )}
        </div>
      </div>
    </>
  )
}
