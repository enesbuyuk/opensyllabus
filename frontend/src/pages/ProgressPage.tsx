import { useQuery } from '@tanstack/react-query'
import { BookOpen, CalendarCheck, CheckCircle2, Clock, Info, ListChecks, Star, TrendingUp } from 'lucide-react'
import { Link, useSearchParams } from 'react-router'
import { coursesApi, miscApi } from '@/api/endpoints'
import { SimpleBarChart } from '@/components/charts'
import { CourseSelect } from '@/components/course-select'
import { EmptyState, ErrorState, PageHeader, PageSkeleton, StatCard } from '@/components/page'
import { WeekStateBadge } from '@/components/status'
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card'
import { NativeSelect } from '@/components/ui/input'
import { Progress } from '@/components/ui/misc'
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from '@/components/ui/table'
import { useSelectedCourse } from '@/hooks/queries'
import { useAuth } from '@/lib/auth'
import { useBreadcrumbs } from '@/lib/breadcrumbs'
import { ATTENDANCE_LABELS, ATTENDANCE_STYLE, attendanceSessionLabel } from '@/lib/labels'
import { cn, formatDate, formatScore } from '@/lib/utils'
import type { CourseProgress } from '@/types/api'

function ProgressView({ p }: { p: CourseProgress }) {
  const { timezone } = useAuth()
  const open = p.weeks.filter((w) => w.state !== 'locked')
  return (
    <div className="space-y-6">
      <div className="grid gap-4 sm:grid-cols-2 lg:grid-cols-4">
        <div className="rounded-xl border bg-card p-4 shadow-xs">
          <div className="flex items-center justify-between text-xs font-medium text-muted-foreground">
            Course completion <TrendingUp className="size-4" />
          </div>
          <div className="mt-2 text-2xl font-semibold tabular-nums">{p.completion_percent}%</div>
          <Progress value={p.completion_percent} className="mt-3" />
        </div>
        <StatCard label="Completed lessons" value={`${p.lessons_completed} / ${p.lessons_total}`} icon={BookOpen} hint={`${p.lesson_percent}%`} />
        <StatCard label="Completed assignments" value={`${p.assignments_completed} / ${p.assignments_total}`} icon={CheckCircle2} hint={`${p.assignments_awaiting_review} awaiting review · ${p.assignments_pending} pending`} />
        <StatCard label="Average grade" value={p.average_grade_percent != null ? `${p.average_grade_percent}%` : '—'} icon={Star} hint={p.assignments_late ? `${p.assignments_late} submitted late` : 'Grades are reported separately from completion'} />
      </div>

      <Card>
        <CardHeader>
          <CardTitle>Weekly completion (%)</CardTitle>
          <CardDescription>Open weeks only</CardDescription>
        </CardHeader>
        <CardContent>
          {open.length ? (
            <SimpleBarChart data={open.map((w) => ({ week: `W${w.week_number}`, completion: w.completion_percent }))} xKey="week" series={[{ key: 'completion', name: 'Completion' }]} unit="%" max={100} ariaLabel="Weekly completion" />
          ) : (
            <p className="text-sm text-muted-foreground">No weeks are open yet.</p>
          )}
        </CardContent>
      </Card>

      <div className="grid gap-6 lg:grid-cols-2">
        <Card>
          <CardHeader>
            <CardTitle>Grade history</CardTitle>
          </CardHeader>
          <CardContent className="px-0">
            {p.grades.length ? (
              <Table>
                <TableHeader>
                  <TableRow>
                    <TableHead className="pl-5">Assignment</TableHead>
                    <TableHead>Returned</TableHead>
                    <TableHead className="pr-5 text-right">Score</TableHead>
                  </TableRow>
                </TableHeader>
                <TableBody>
                  {p.grades.map((g) => (
                    <TableRow key={g.assignment_id}>
                      <TableCell className="pl-5">
                        <Link to={`/assignments/${g.assignment_id}`} className="hover:underline">
                          {g.assignment_title}
                        </Link>
                        {g.week_number && <div className="text-xs text-muted-foreground">Week {g.week_number}</div>}
                      </TableCell>
                      <TableCell className="text-muted-foreground">{formatDate(g.released_at, timezone)}</TableCell>
                      <TableCell className="pr-5 text-right tabular-nums">
                        {formatScore(g.score, g.max_score)} <span className="text-xs text-muted-foreground">({g.percent}%)</span>
                      </TableCell>
                    </TableRow>
                  ))}
                </TableBody>
              </Table>
            ) : (
              <p className="px-5 text-sm text-muted-foreground">No grades returned yet.</p>
            )}
          </CardContent>
        </Card>
        <Card>
          <CardHeader>
            <CardTitle>Weeks</CardTitle>
          </CardHeader>
          <CardContent className="max-h-96 space-y-1 overflow-y-auto px-3">
            {p.weeks.map((w) => (
              <div key={w.week_id} className="flex items-center gap-3 rounded-md px-2 py-1.5">
                <span className="w-8 text-xs text-muted-foreground tabular-nums">W{w.week_number}</span>
                <span className="min-w-0 flex-1 truncate text-sm">{w.title}</span>
                {w.state === 'locked' ? (
                  <WeekStateBadge state="locked" />
                ) : (
                  <div className="flex w-32 items-center gap-2">
                    <Progress value={w.completion_percent} />
                    <span className="w-9 text-right text-xs tabular-nums">{Math.round(w.completion_percent)}%</span>
                  </div>
                )}
              </div>
            ))}
          </CardContent>
        </Card>
      </div>
    </div>
  )
}

function MyAttendanceCard({ courseId }: { courseId: string }) {
  const q = useQuery({ queryKey: ['attendance', 'me', courseId], queryFn: () => coursesApi.myAttendance(courseId) })
  if (!q.data || q.data.sessions.length === 0) return null
  const { summary, sessions } = q.data
  const missed = sessions.filter((s) => s.status && s.status !== 'present').reverse()
  return (
    <Card>
      <CardHeader className="flex-row items-start justify-between gap-3">
        <div className="space-y-1">
          <CardTitle className="flex items-center gap-2">
            <CalendarCheck className="size-4 text-muted-foreground" /> Attendance
          </CardTitle>
          <CardDescription>
            {summary.present} present · {summary.late} late · {summary.absent} absent · {summary.excused} excused
          </CardDescription>
        </div>
        <div className="text-2xl font-semibold tabular-nums">{summary.rate_percent != null ? `${summary.rate_percent}%` : '—'}</div>
      </CardHeader>
      <CardContent>
        {missed.length ? (
          <ul className="space-y-1.5">
            {missed.map((s) => (
              <li key={s.key} className="flex items-center justify-between gap-3 text-sm">
                <span>{attendanceSessionLabel(s)}</span>
                <span className={cn('rounded-md border px-1.5 py-0.5 text-[11px] font-medium', ATTENDANCE_STYLE[s.status!])}>{ATTENDANCE_LABELS[s.status!]}</span>
              </li>
            ))}
          </ul>
        ) : (
          <p className="text-sm text-muted-foreground">{summary.present ? 'You attended every marked session.' : 'No attendance has been taken yet.'}</p>
        )}
      </CardContent>
    </Card>
  )
}

function HowCalculated() {
  return (
    <div className="flex gap-3 rounded-xl border bg-muted/20 p-4 text-[13px] text-muted-foreground">
      <Info className="mt-0.5 size-4 shrink-0" />
      <p>
        Completion counts published lessons you marked complete and assignments whose review has been returned with a grade. Uploading or submitting work alone does not count as
        completion. Grades are shown separately and do not affect the completion percentage.
      </p>
    </div>
  )
}

export function ProgressPage() {
  useBreadcrumbs([{ label: 'Progress' }])
  const { isAdmin } = useAuth()
  const [params, setParams] = useSearchParams()
  const { courses, course, select } = useSelectedCourse()
  const studentId = params.get('student') ?? ''
  const mine = useQuery({ queryKey: ['progress', 'me'], queryFn: miscApi.myProgress, enabled: !isAdmin })
  const roster = useQuery({ queryKey: ['progress', 'course', course?.id], queryFn: () => coursesApi.progress(course!.id), enabled: isAdmin && !!course })

  if (courses.isLoading || mine.isLoading || roster.isLoading) return <PageSkeleton />
  if (!course) return <EmptyState icon={TrendingUp} title="No courses available" />

  if (!isAdmin) {
    if (mine.error) return <ErrorState error={mine.error} onRetry={() => mine.refetch()} />
    const p = mine.data?.find((x) => x.course_id === course.id)
    return (
      <div className="space-y-6">
        <PageHeader eyebrow={course.title} title="My progress" actions={<CourseSelect courses={courses.data ?? []} value={course.id} onChange={select} />} />
        {p ? <ProgressView p={p} /> : <EmptyState icon={Clock} title="No progress yet" />}
        <MyAttendanceCard courseId={course.id} />
        <HowCalculated />
      </div>
    )
  }

  if (roster.error) return <ErrorState error={roster.error} onRetry={() => roster.refetch()} />
  const rows = roster.data ?? []
  const selected = rows.find((r) => r.student.id === studentId)
  return (
    <div className="space-y-6">
      <PageHeader
        eyebrow={course.title}
        title="Student progress"
        description="Select a student to see their detailed progress."
        actions={
          <>
            <CourseSelect courses={courses.data ?? []} value={course.id} onChange={select} />
            <NativeSelect
              value={studentId}
              onChange={(e) =>
                setParams((p) => {
                  const n = new URLSearchParams(p)
                  if (e.target.value) n.set('student', e.target.value)
                  else n.delete('student')
                  return n
                })
              }
              className="w-full sm:w-56"
              aria-label="Student"
            >
              <option value="">All students</option>
              {rows.map((r) => (
                <option key={r.student.id} value={r.student.id}>
                  {r.student.name}
                </option>
              ))}
            </NativeSelect>
          </>
        }
      />
      {selected ? (
        <ProgressView p={selected.progress} />
      ) : rows.length === 0 ? (
        <EmptyState icon={ListChecks} title="No students enrolled" />
      ) : (
        <Card>
          <CardContent className="px-0 pt-2">
            <Table>
              <TableHeader>
                <TableRow>
                  <TableHead className="pl-5">Student</TableHead>
                  <TableHead className="w-56">Completion</TableHead>
                  <TableHead>Lessons</TableHead>
                  <TableHead>Assignments</TableHead>
                  <TableHead className="pr-5">Avg. grade</TableHead>
                </TableRow>
              </TableHeader>
              <TableBody>
                {rows.map((r) => (
                  <TableRow key={r.student.id} className="cursor-pointer" onClick={() => setParams((p) => new URLSearchParams({ ...Object.fromEntries(p), student: r.student.id }))}>
                    <TableCell className="pl-5 font-medium">{r.student.name}</TableCell>
                    <TableCell>
                      <div className="flex items-center gap-2">
                        <Progress value={r.progress.completion_percent} className="w-28" />
                        <span className="text-xs tabular-nums">{r.progress.completion_percent}%</span>
                      </div>
                    </TableCell>
                    <TableCell className="tabular-nums">
                      {r.progress.lessons_completed}/{r.progress.lessons_total}
                    </TableCell>
                    <TableCell className="tabular-nums">
                      {r.progress.assignments_completed}/{r.progress.assignments_total}
                    </TableCell>
                    <TableCell className="pr-5 tabular-nums">{r.progress.average_grade_percent != null ? `${r.progress.average_grade_percent}%` : '—'}</TableCell>
                  </TableRow>
                ))}
              </TableBody>
            </Table>
          </CardContent>
        </Card>
      )}
      <HowCalculated />
    </div>
  )
}
