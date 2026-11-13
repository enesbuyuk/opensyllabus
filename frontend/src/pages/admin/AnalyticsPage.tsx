import { useQuery } from '@tanstack/react-query'
import { BarChart3 } from 'lucide-react'
import { useSearchParams } from 'react-router'
import { coursesApi, miscApi } from '@/api/endpoints'
import { SimpleBarChart } from '@/components/charts'
import { CourseSelect } from '@/components/course-select'
import { EmptyState, ErrorState, PageHeader, PageSkeleton, StatCard } from '@/components/page'
import { Badge, type BadgeVariant } from '@/components/ui/badge'
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card'
import { NativeSelect } from '@/components/ui/input'
import { Progress } from '@/components/ui/misc'
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from '@/components/ui/table'
import { useSelectedCourse } from '@/hooks/queries'
import { useBreadcrumbs } from '@/lib/breadcrumbs'
import { ATTENDANCE_LABELS, ATTENDANCE_STYLE, attendanceSessionLabel, STATE_LABELS, SUBMISSION_STATUS_LABELS } from '@/lib/labels'
import { cn } from '@/lib/utils'
import type { Analytics, AttendanceStatus, GradebookCell, SubmissionStatus } from '@/types/api'

const avg = (xs: number[]) => (xs.length ? Math.round((xs.reduce((s, x) => s + x, 0) / xs.length) * 10) / 10 : null)
const pct = (v: number | null | undefined) => (v != null ? `${v}%` : '—')

const STATE_VARIANT: Record<GradebookCell['state'], BadgeVariant> = {
  not_started: 'secondary',
  draft: 'secondary',
  submitted: 'info',
  under_review: 'info',
  graded: 'success',
  revision_requested: 'warning',
  late: 'destructive',
  missing: 'destructive',
}

function StudentAnalytics({ courseId, studentId, a }: { courseId: string; studentId: string; a: Analytics }) {
  const gb = useQuery({ queryKey: ['gradebook', courseId, ''], queryFn: () => coursesApi.gradebook(courseId) })
  const att = useQuery({ queryKey: ['attendance', courseId], queryFn: () => coursesApi.attendance(courseId) })
  if (gb.isLoading || att.isLoading) return <PageSkeleton rows={2} />
  if (gb.error || !gb.data) return <ErrorState error={gb.error} onRetry={() => gb.refetch()} />
  const st = a.students.find((s) => s.student_id === studentId)!
  const row = gb.data.rows.find((r) => r.student.id === studentId)
  const classGrade = avg(a.students.map((s) => s.average_grade_percent).filter((g): g is number => g != null))
  const columns = gb.data.columns.filter((c) => c.status === 'published')
  const classAvg = Object.fromEntries(a.assignments.map((x) => [x.assignment_id, x.average_percent]))
  const sessions = att.data?.sessions.filter((s) => s.marked > 0) ?? []
  const records = att.data?.records.filter((r) => r.student_id === studentId) ?? []
  const summary = att.data?.summaries.find((s) => s.student_id === studentId)

  return (
    <>
      <div className="grid gap-4 sm:grid-cols-2 lg:grid-cols-4">
        <StatCard label="Completion" value={pct(st.completion_percent)} hint={`Class average ${pct(avg(a.students.map((s) => s.completion_percent)))}`} />
        <StatCard label="Average grade" value={pct(st.average_grade_percent)} hint={`Class average ${pct(classGrade)}`} />
        <StatCard label="Late submissions" value={st.late_count} hint={`Lessons ${pct(st.lesson_percent)} · assignments ${pct(st.assignment_percent)}`} />
        <StatCard label="Attendance" value={pct(summary?.rate_percent)} hint={summary ? `${summary.absent} absent · ${summary.late} late · ${summary.excused} excused` : 'No classes marked'} />
      </div>

      <Card>
        <CardHeader>
          <CardTitle>Completion by week (%)</CardTitle>
          <CardDescription>This student compared with the class average, published weeks only</CardDescription>
        </CardHeader>
        <CardContent>
          {a.weeks.length ? (
            <SimpleBarChart
              data={a.weeks.map((w) => ({ week: `W${w.week_number}`, student: row?.weeks[String(w.week_number)] ?? 0, average: w.average_completion }))}
              xKey="week"
              series={[
                { key: 'student', name: st.name },
                { key: 'average', name: 'Class average' },
              ]}
              unit="%"
              max={100}
              ariaLabel={`Completion by week for ${st.name}`}
            />
          ) : (
            <p className="text-sm text-muted-foreground">No published weeks yet.</p>
          )}
        </CardContent>
      </Card>

      <Card>
        <CardHeader>
          <CardTitle>Assignments</CardTitle>
          <CardDescription>Status and grade on each published assignment</CardDescription>
        </CardHeader>
        <CardContent className="px-0">
          {columns.length ? (
            <Table>
              <TableHeader>
                <TableRow>
                  <TableHead className="pl-5">Assignment</TableHead>
                  <TableHead>Status</TableHead>
                  <TableHead>Score</TableHead>
                  <TableHead className="text-right">Grade</TableHead>
                  <TableHead className="pr-5 text-right">Class avg.</TableHead>
                </TableRow>
              </TableHeader>
              <TableBody>
                {columns.map((c) => {
                  const cell = row?.cells[c.id]
                  return (
                    <TableRow key={c.id}>
                      <TableCell className="max-w-64 truncate pl-5">
                        {c.week_number ? <span className="text-muted-foreground">W{c.week_number} · </span> : null}
                        {c.title}
                      </TableCell>
                      <TableCell>
                        {cell ? (
                          <span className="flex flex-wrap gap-1">
                            <Badge variant={STATE_VARIANT[cell.state]}>{STATE_LABELS[cell.state]}</Badge>
                            {cell.is_late && cell.state !== 'late' && <Badge variant="destructive">Late</Badge>}
                          </span>
                        ) : (
                          '—'
                        )}
                      </TableCell>
                      <TableCell className="tabular-nums">
                        {cell?.score != null ? (
                          <>
                            {cell.score}/{c.max_grade}
                            {!cell.released && <span className="text-xs text-muted-foreground"> (not returned)</span>}
                          </>
                        ) : (
                          '—'
                        )}
                      </TableCell>
                      <TableCell className="text-right tabular-nums">{cell?.score != null && c.max_grade > 0 ? pct(Math.round((cell.score / c.max_grade) * 1000) / 10) : '—'}</TableCell>
                      <TableCell className="pr-5 text-right text-muted-foreground tabular-nums">{pct(classAvg[c.id])}</TableCell>
                    </TableRow>
                  )
                })}
              </TableBody>
            </Table>
          ) : (
            <p className="px-5 text-sm text-muted-foreground">No published assignments.</p>
          )}
        </CardContent>
      </Card>

      {sessions.length > 0 && (
        <Card>
          <CardHeader>
            <CardTitle>Attendance</CardTitle>
            <CardDescription>Every class and event with attendance taken</CardDescription>
          </CardHeader>
          <CardContent className="flex flex-wrap gap-1.5">
            {sessions.map((s) => {
              const v = records.find((r) => r.key === s.key)?.status as AttendanceStatus | undefined
              return (
                <span
                  key={s.key}
                  title={`${attendanceSessionLabel(s)}: ${v ? ATTENDANCE_LABELS[v] : 'Not marked'}`}
                  className={cn('inline-flex size-7 items-center justify-center rounded border text-[11px] font-semibold', v ? ATTENDANCE_STYLE[v] : 'text-muted-foreground')}
                >
                  {v ? ATTENDANCE_LABELS[v][0] : '·'}
                </span>
              )
            })}
          </CardContent>
        </Card>
      )}
    </>
  )
}

export function AnalyticsPage() {
  useBreadcrumbs([{ label: 'Analytics' }])
  const { courses, course, select } = useSelectedCourse()
  const [params, setParams] = useSearchParams()
  const q = useQuery({ queryKey: ['analytics', course?.id], queryFn: () => miscApi.analytics(course?.id), enabled: !!course })
  if (courses.isLoading || q.isLoading) return <PageSkeleton />
  if (!course) return <EmptyState icon={BarChart3} title="No courses" />
  if (q.error || !q.data) return <ErrorState error={q.error} onRetry={() => q.refetch()} />
  const a = q.data
  const grades = a.students.map((s) => s.average_grade_percent).filter((g): g is number => g != null)
  const totalSubs = a.submission_status.reduce((s, x) => s + x.count, 0)
  const late = a.students.reduce((s, x) => s + x.late_count, 0)
  const student = a.students.find((s) => s.student_id === params.get('student'))
  const selectStudent = (id: string) =>
    setParams(
      (p) => {
        const next = new URLSearchParams(p)
        if (id) next.set('student', id)
        else next.delete('student')
        return next
      },
      { replace: true },
    )

  return (
    <div className="space-y-6">
      <PageHeader
        eyebrow={a.course_title ?? undefined}
        title={student ? student.name : 'Analytics'}
        description={student ? 'Analytics for one student, computed from their recorded activity.' : 'All figures are computed from recorded student activity.'}
        actions={
          <div className="flex w-full flex-col gap-2 sm:w-auto sm:flex-row">
            <CourseSelect
              courses={courses.data ?? []}
              value={course.id}
              onChange={(id) => {
                select(id)
                selectStudent('')
              }}
            />
            {a.students.length > 0 && (
              <NativeSelect value={student?.student_id ?? ''} onChange={(e) => selectStudent(e.target.value)} className="w-full sm:w-56" aria-label="Student">
                <option value="">All students</option>
                {a.students.map((s) => (
                  <option key={s.student_id} value={s.student_id}>
                    {s.name}
                  </option>
                ))}
              </NativeSelect>
            )}
          </div>
        }
      />
      {student ? (
        <StudentAnalytics courseId={course.id} studentId={student.student_id} a={a} />
      ) : (
        <>
          <div className="grid gap-4 sm:grid-cols-2 lg:grid-cols-4">
            <StatCard label="Students" value={a.students.length} />
            <StatCard label="Average completion" value={pct(avg(a.students.map((s) => s.completion_percent)))} />
            <StatCard label="Average grade" value={pct(avg(grades))} hint={`${grades.length} students with grades`} />
            <StatCard label="Late submissions" value={late} hint={`of ${totalSubs} submissions`} />
          </div>

          {a.students.length === 0 ? (
            <EmptyState icon={BarChart3} title="No data yet" description="Analytics appear once students are enrolled and active." />
          ) : (
            <>
              <div className="grid gap-6 lg:grid-cols-2">
                <Card>
                  <CardHeader>
                    <CardTitle>Completion by student (%)</CardTitle>
                    <CardDescription>Lessons completed and assignments returned with a grade</CardDescription>
                  </CardHeader>
                  <CardContent>
                    <SimpleBarChart data={a.students.map((s) => ({ name: s.name.split(' ')[0], completion: s.completion_percent }))} xKey="name" series={[{ key: 'completion', name: 'Completion' }]} unit="%" max={100} ariaLabel="Completion by student" />
                  </CardContent>
                </Card>
                <Card>
                  <CardHeader>
                    <CardTitle>Average completion by week (%)</CardTitle>
                    <CardDescription>Published weeks only</CardDescription>
                  </CardHeader>
                  <CardContent>
                    {a.weeks.length ? (
                      <SimpleBarChart data={a.weeks.map((w) => ({ week: `W${w.week_number}`, completion: w.average_completion }))} xKey="week" series={[{ key: 'completion', name: 'Avg. completion' }]} unit="%" max={100} ariaLabel="Average completion by week" />
                    ) : (
                      <p className="text-sm text-muted-foreground">No published weeks yet.</p>
                    )}
                  </CardContent>
                </Card>
              </div>
              <div className="grid gap-6 lg:grid-cols-3">
                <Card className="lg:col-span-2">
                  <CardHeader>
                    <CardTitle>Assignments</CardTitle>
                    <CardDescription>Submission and grading coverage per published assignment</CardDescription>
                  </CardHeader>
                  <CardContent className="px-0">
                    {a.assignments.length ? (
                      <Table>
                        <TableHeader>
                          <TableRow>
                            <TableHead className="pl-5">Assignment</TableHead>
                            <TableHead>Submitted</TableHead>
                            <TableHead>Graded</TableHead>
                            <TableHead>Late</TableHead>
                            <TableHead className="pr-5 text-right">Avg. grade</TableHead>
                          </TableRow>
                        </TableHeader>
                        <TableBody>
                          {a.assignments.map((x) => (
                            <TableRow key={x.assignment_id}>
                              <TableCell className="max-w-64 truncate pl-5">
                                {x.week_number ? <span className="text-muted-foreground">W{x.week_number} · </span> : null}
                                {x.title}
                              </TableCell>
                              <TableCell className="tabular-nums">
                                {x.submitted}/{x.enrolled}
                              </TableCell>
                              <TableCell className="tabular-nums">{x.graded}</TableCell>
                              <TableCell className="tabular-nums">{x.late}</TableCell>
                              <TableCell className="pr-5 text-right tabular-nums">{x.average_percent != null ? `${x.average_percent}%` : '—'}</TableCell>
                            </TableRow>
                          ))}
                        </TableBody>
                      </Table>
                    ) : (
                      <p className="px-5 text-sm text-muted-foreground">No published assignments.</p>
                    )}
                  </CardContent>
                </Card>
                <Card>
                  <CardHeader>
                    <CardTitle>Submission status</CardTitle>
                    <CardDescription>{totalSubs} submissions in total</CardDescription>
                  </CardHeader>
                  <CardContent className="space-y-3">
                    {a.submission_status.length ? (
                      a.submission_status
                        .sort((x, y) => y.count - x.count)
                        .map((s) => (
                          <div key={s.status} className="space-y-1">
                            <div className="flex justify-between text-sm">
                              <span>{SUBMISSION_STATUS_LABELS[s.status as SubmissionStatus] ?? s.status}</span>
                              <span className="tabular-nums">{s.count}</span>
                            </div>
                            <Progress value={(s.count / totalSubs) * 100} />
                          </div>
                        ))
                    ) : (
                      <p className="text-sm text-muted-foreground">No submissions yet.</p>
                    )}
                  </CardContent>
                </Card>
              </div>
            </>
          )}
        </>
      )}
    </div>
  )
}
