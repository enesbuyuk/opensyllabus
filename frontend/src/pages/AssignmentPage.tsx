import { useQuery } from '@tanstack/react-query'
import { CalendarClock, ClipboardCheck, FileType2, Pencil, Scale, Timer } from 'lucide-react'
import { Link, useParams } from 'react-router'
import { assignmentsApi } from '@/api/endpoints'
import { Markdown } from '@/components/markdown'
import { ErrorState, PageSkeleton } from '@/components/page'
import { AssignmentStateBadge, PublishBadge, SubmissionStatusBadge } from '@/components/status'
import { Badge } from '@/components/ui/badge'
import { Button } from '@/components/ui/button'
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card'
import { Avatar } from '@/components/ui/misc'
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from '@/components/ui/table'
import { GroupsCard } from '@/features/assignments/GroupsCard'
import { SubmissionWorkspace } from '@/features/assignments/SubmissionWorkspace'
import { useAuth } from '@/lib/auth'
import { useBreadcrumbs } from '@/lib/breadcrumbs'
import { ASSIGNMENT_KIND_LABELS, LATE_POLICY_LABELS } from '@/lib/labels'
import { formatDateTime, formatScore, relativeTime } from '@/lib/utils'
import type { AssignmentDetail } from '@/types/api'

function Meta({ a }: { a: AssignmentDetail }) {
  const { timezone } = useAuth()
  const items = [
    { icon: CalendarClock, label: 'Due', value: a.due_at ? `${formatDateTime(a.due_at, timezone)} (${relativeTime(a.due_at)})` : 'No due date' },
    { icon: FileType2, label: 'Type', value: ASSIGNMENT_KIND_LABELS[a.kind] },
    { icon: Scale, label: 'Maximum grade', value: String(a.max_grade) },
    {
      icon: Timer,
      label: 'Late policy',
      value: a.late_policy === 'penalty' ? `${LATE_POLICY_LABELS.penalty} (${a.late_penalty_percent}%)` : LATE_POLICY_LABELS[a.late_policy],
    },
  ]
  return (
    <dl className="grid gap-3 rounded-xl border bg-card p-4 sm:grid-cols-2 lg:grid-cols-4">
      {items.map((i) => (
        <div key={i.label} className="flex gap-3">
          <i.icon className="mt-0.5 size-4 shrink-0 text-muted-foreground" />
          <div className="min-w-0">
            <dt className="text-xs text-muted-foreground">{i.label}</dt>
            <dd className="text-sm">{i.value}</dd>
          </div>
        </div>
      ))}
    </dl>
  )
}

function Instructions({ a }: { a: AssignmentDetail }) {
  return (
    <div className="space-y-6">
      <Card>
        <CardHeader>
          <CardTitle>Instructions</CardTitle>
        </CardHeader>
        <CardContent>{a.instructions_md ? <Markdown>{a.instructions_md}</Markdown> : <p className="text-sm text-muted-foreground">{a.description || 'No instructions provided.'}</p>}</CardContent>
      </Card>
      {a.reference_md && (
        <Card>
          <CardHeader>
            <CardTitle>Reference materials</CardTitle>
          </CardHeader>
          <CardContent>
            <Markdown>{a.reference_md}</Markdown>
          </CardContent>
        </Card>
      )}
      {a.rubric.length > 0 && (
        <Card>
          <CardHeader>
            <CardTitle>Rubric</CardTitle>
          </CardHeader>
          <CardContent className="px-0">
            <Table>
              <TableBody>
                {a.rubric.map((c) => (
                  <TableRow key={c.id}>
                    <TableCell className="pl-5">
                      <div className="text-sm font-medium">{c.title}</div>
                      {c.description && <div className="text-xs text-muted-foreground">{c.description}</div>}
                    </TableCell>
                    <TableCell className="pr-5 text-right whitespace-nowrap tabular-nums">{c.max_points} pts</TableCell>
                  </TableRow>
                ))}
              </TableBody>
            </Table>
          </CardContent>
        </Card>
      )}
    </div>
  )
}

function InstructorSubmissions({ a }: { a: AssignmentDetail }) {
  const q = useQuery({ queryKey: ['assignment-submissions', a.id], queryFn: () => assignmentsApi.submissions(a.id) })
  return (
    <Card>
      <CardHeader className="flex-row items-center justify-between">
        <CardTitle>Submissions</CardTitle>
        <Button asChild variant="outline" size="sm">
          <Link to={`/admin/reviews?assignment=${a.id}&course=${a.course_id}`}>
            <ClipboardCheck /> Review workspace
          </Link>
        </Button>
      </CardHeader>
      <CardContent className="px-0">
        {q.data?.length ? (
          <Table>
            <TableHeader>
              <TableRow>
                <TableHead className="pl-5">{a.is_group ? 'Group' : 'Student'}</TableHead>
                <TableHead>Status</TableHead>
                <TableHead>Submitted</TableHead>
                <TableHead className="pr-5 text-right">Grade</TableHead>
              </TableRow>
            </TableHeader>
            <TableBody>
              {q.data.map((s) => (
                <TableRow key={s.id}>
                  <TableCell className="pl-5">
                    <Link to={`/admin/reviews?submission=${s.id}`} className="flex items-center gap-2 font-medium hover:underline">
                      <Avatar name={s.group_name ?? s.student_name} className="size-6 text-[10px]" />
                      {s.group_name ?? s.student_name}
                      {s.group_name && <span className="font-normal text-muted-foreground">({s.student_name})</span>}
                    </Link>
                  </TableCell>
                  <TableCell>
                    <SubmissionStatusBadge status={s.status} late={s.is_late} />
                  </TableCell>
                  <TableCell className="text-muted-foreground">{relativeTime(s.submitted_at)}</TableCell>
                  <TableCell className="pr-5 text-right tabular-nums">{formatScore(s.score, s.max_grade)}</TableCell>
                </TableRow>
              ))}
            </TableBody>
          </Table>
        ) : (
          <p className="px-5 text-sm text-muted-foreground">{q.isLoading ? 'Loading…' : 'No submissions yet.'}</p>
        )}
      </CardContent>
    </Card>
  )
}

export function AssignmentPage() {
  const { id = '' } = useParams()
  const { isAdmin } = useAuth()
  const q = useQuery({ queryKey: ['assignment', id], queryFn: () => assignmentsApi.get(id) })
  useBreadcrumbs([{ label: 'Assignments', to: '/assignments' }, { label: q.data?.title ?? 'Assignment' }])
  if (q.isLoading) return <PageSkeleton />
  if (q.error || !q.data) return <ErrorState error={q.error} onRetry={() => q.refetch()} />
  const a = q.data

  return (
    <div className="space-y-6">
      <div className="flex flex-col gap-4 sm:flex-row sm:items-end sm:justify-between">
        <div className="space-y-2">
          <div className="flex flex-wrap items-center gap-2 text-xs text-muted-foreground">
            <span>{a.course_title}</span>
            {a.week_number && (
              <Link to={`/weeks/${a.week_id}`} className="hover:text-foreground">
                · Week {a.week_number}
              </Link>
            )}
            {a.lesson_title && <span>· {a.lesson_title}</span>}
          </div>
          <h1 className="text-2xl font-semibold tracking-tight">{a.title}</h1>
          <div className="flex flex-wrap items-center gap-2">
            {isAdmin ? <PublishBadge status={a.status} /> : a.state && <AssignmentStateBadge state={a.state} />}
            {a.allow_resubmission && <Badge variant="outline">Resubmission allowed</Badge>}
            {a.is_group && <Badge variant="outline">Group assignment</Badge>}
            {!isAdmin && a.score != null && <Badge variant="success">Grade {formatScore(a.score, a.max_grade)}</Badge>}
          </div>
          {a.description && <p className="max-w-3xl text-sm text-muted-foreground">{a.description}</p>}
        </div>
        {isAdmin && (
          <Button asChild variant="outline">
            <Link to={`/admin/assignments/${a.id}/edit`}>
              <Pencil /> Edit assignment
            </Link>
          </Button>
        )}
      </div>
      <Meta a={a} />
      <div className="grid gap-6 xl:grid-cols-5">
        <div className="xl:col-span-2">
          <Instructions a={a} />
        </div>
        <div className="space-y-6 xl:col-span-3">
          {isAdmin && a.is_group && <GroupsCard assignment={a} />}
          {isAdmin ? <InstructorSubmissions a={a} /> : <SubmissionWorkspace assignment={a} />}
        </div>
      </div>
    </div>
  )
}
