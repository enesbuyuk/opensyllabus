import { useQuery } from '@tanstack/react-query'
import { BarChart3 } from 'lucide-react'
import { miscApi } from '@/api/endpoints'
import { SimpleBarChart } from '@/components/charts'
import { CourseSelect } from '@/components/course-select'
import { EmptyState, ErrorState, PageHeader, PageSkeleton, StatCard } from '@/components/page'
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card'
import { Progress } from '@/components/ui/misc'
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from '@/components/ui/table'
import { useSelectedCourse } from '@/hooks/queries'
import { useBreadcrumbs } from '@/lib/breadcrumbs'
import { SUBMISSION_STATUS_LABELS } from '@/lib/labels'
import type { SubmissionStatus } from '@/types/api'

export function AnalyticsPage() {
  useBreadcrumbs([{ label: 'Analytics' }])
  const { courses, course, select } = useSelectedCourse()
  const q = useQuery({ queryKey: ['analytics', course?.id], queryFn: () => miscApi.analytics(course?.id), enabled: !!course })
  if (courses.isLoading || q.isLoading) return <PageSkeleton />
  if (!course) return <EmptyState icon={BarChart3} title="No courses" />
  if (q.error || !q.data) return <ErrorState error={q.error} onRetry={() => q.refetch()} />
  const a = q.data
  const avg = (xs: number[]) => (xs.length ? Math.round((xs.reduce((s, x) => s + x, 0) / xs.length) * 10) / 10 : null)
  const grades = a.students.map((s) => s.average_grade_percent).filter((g): g is number => g != null)
  const totalSubs = a.submission_status.reduce((s, x) => s + x.count, 0)
  const late = a.students.reduce((s, x) => s + x.late_count, 0)

  return (
    <div className="space-y-6">
      <PageHeader eyebrow={a.course_title ?? undefined} title="Analytics" description="All figures are computed from recorded student activity." actions={<CourseSelect courses={courses.data ?? []} value={course.id} onChange={select} />} />
      <div className="grid gap-4 sm:grid-cols-2 lg:grid-cols-4">
        <StatCard label="Students" value={a.students.length} />
        <StatCard label="Average completion" value={avg(a.students.map((s) => s.completion_percent)) != null ? `${avg(a.students.map((s) => s.completion_percent))}%` : '—'} />
        <StatCard label="Average grade" value={avg(grades) != null ? `${avg(grades)}%` : '—'} hint={`${grades.length} students with grades`} />
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
    </div>
  )
}
