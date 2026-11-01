import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query'
import { CalendarCheck, CheckCheck, Save } from 'lucide-react'
import * as React from 'react'
import { Link } from 'react-router'
import { toast } from 'sonner'
import { coursesApi } from '@/api/endpoints'
import { CourseSelect } from '@/components/course-select'
import { EmptyState, ErrorState, PageHeader, PageSkeleton } from '@/components/page'
import { Button } from '@/components/ui/button'
import { Card, CardContent, CardFooter } from '@/components/ui/card'
import { NativeSelect } from '@/components/ui/input'
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from '@/components/ui/table'
import { Tabs, TabsContent, TabsList, TabsTrigger } from '@/components/ui/tabs'
import { useSelectedCourse } from '@/hooks/queries'
import { useBreadcrumbs } from '@/lib/breadcrumbs'
import { errorMessage } from '@/lib/errors'
import { ATTENDANCE_LABELS, ATTENDANCE_STYLE, attendanceSessionLabel as sessionLabel } from '@/lib/labels'
import { cn, formatPlainDate } from '@/lib/utils'
import type { AttendanceSession, AttendanceStatus, CourseAttendance } from '@/types/api'

const STATUSES: AttendanceStatus[] = ['present', 'late', 'absent', 'excused']

const sessionKey = (s: { week_number: number; day_number: number }) => `${s.week_number}-${s.day_number}`

function TakeAttendance({ courseId, data }: { courseId: string; data: CourseAttendance }) {
  const qc = useQueryClient()
  const now = new Date().toISOString()
  // Default to the most recent class that has started.
  const started = data.sessions.filter((s) => s.starts_at && s.starts_at <= now)
  const [key, setKey] = React.useState(() => sessionKey(started.at(-1) ?? data.sessions[0]))
  const session = data.sessions.find((s) => sessionKey(s) === key) ?? data.sessions[0]
  const saved = React.useMemo(
    () =>
      Object.fromEntries(
        data.students.map((st) => [st.id, data.records.find((r) => r.student_id === st.id && sessionKey(r) === sessionKey(session))?.status ?? null]),
      ) as Record<string, AttendanceStatus | null>,
    [data, session],
  )
  const [draft, setDraft] = React.useState(saved)
  React.useEffect(() => setDraft(saved), [saved])
  const changed = data.students.filter((st) => draft[st.id] !== saved[st.id])

  const save = useMutation({
    mutationFn: () =>
      coursesApi.markAttendance(courseId, {
        week_number: session.week_number,
        day_number: session.day_number,
        records: changed.map((st) => ({ student_id: st.id, status: draft[st.id] })),
      }),
    onSuccess: (d) => {
      qc.setQueryData(['attendance', courseId], d)
      toast.success('Attendance saved')
    },
    onError: (e) => toast.error(errorMessage(e)),
  })

  return (
    <Card>
      <div className="flex flex-wrap items-center gap-2 border-b px-5 py-3">
        <NativeSelect value={key} onChange={(e) => setKey(e.target.value)} className="w-full sm:w-80" aria-label="Class">
          {data.sessions.map((s) => (
            <option key={sessionKey(s)} value={sessionKey(s)}>
              {sessionLabel(s)}
              {s.marked ? ` (${s.marked}/${data.students.length} marked)` : ''}
            </option>
          ))}
        </NativeSelect>
        <Button
          variant="outline"
          size="sm"
          className="sm:ml-auto"
          onClick={() => setDraft((d) => Object.fromEntries(data.students.map((st) => [st.id, d[st.id] ?? 'present'])))}
        >
          <CheckCheck /> Mark the rest present
        </Button>
      </div>
      <CardContent className="px-0">
        {data.students.length === 0 ? (
          <p className="px-5 py-4 text-sm text-muted-foreground">No students are enrolled in this course.</p>
        ) : (
          <ul className="divide-y">
            {data.students.map((st) => (
              <li key={st.id} className="flex flex-wrap items-center gap-3 px-5 py-2.5">
                <span className="min-w-0 flex-1 truncate text-sm font-medium">{st.name}</span>
                <div className="flex gap-1" role="group" aria-label={`Attendance for ${st.name}`}>
                  {STATUSES.map((status) => (
                    <button
                      key={status}
                      type="button"
                      aria-pressed={draft[st.id] === status}
                      onClick={() => setDraft((d) => ({ ...d, [st.id]: d[st.id] === status ? null : status }))}
                      className={cn(
                        'h-8 rounded-md border px-2.5 text-xs font-medium transition-colors hover:bg-accent',
                        draft[st.id] === status ? ATTENDANCE_STYLE[status] : 'text-muted-foreground',
                      )}
                    >
                      {ATTENDANCE_LABELS[status]}
                    </button>
                  ))}
                </div>
              </li>
            ))}
          </ul>
        )}
      </CardContent>
      <CardFooter className="justify-between gap-3 border-t pt-4">
        <span className="text-xs text-muted-foreground">{changed.length ? `${changed.length} unsaved change${changed.length > 1 ? 's' : ''}` : 'All changes saved.'}</span>
        <Button onClick={() => save.mutate()} loading={save.isPending} disabled={!changed.length}>
          <Save /> Save attendance
        </Button>
      </CardFooter>
    </Card>
  )
}

function Overview({ data }: { data: CourseAttendance }) {
  const marked = data.sessions.filter((s) => s.marked > 0)
  const status = (studentId: string, s: AttendanceSession) => data.records.find((r) => r.student_id === studentId && sessionKey(r) === sessionKey(s))?.status
  return (
    <Card>
      <CardContent className="overflow-x-auto px-0">
        <Table>
          <TableHeader>
            <TableRow>
              <TableHead className="pl-5">Student</TableHead>
              <TableHead className="text-right">Attendance</TableHead>
              {STATUSES.map((s) => (
                <TableHead key={s} className="text-right">
                  {ATTENDANCE_LABELS[s]}
                </TableHead>
              ))}
              {marked.map((s) => (
                <TableHead key={sessionKey(s)} className="text-center text-[11px] whitespace-nowrap" title={sessionLabel(s)}>
                  W{s.week_number}
                  <br />
                  {s.date ? formatPlainDate(s.date, { year: undefined }) : `D${s.day_number}`}
                </TableHead>
              ))}
            </TableRow>
          </TableHeader>
          <TableBody>
            {data.students.map((st) => {
              const sum = data.summaries.find((x) => x.student_id === st.id)
              return (
                <TableRow key={st.id}>
                  <TableCell className="pl-5 font-medium whitespace-nowrap">{st.name}</TableCell>
                  <TableCell className="text-right font-medium tabular-nums">{sum?.rate_percent != null ? `${sum.rate_percent}%` : '—'}</TableCell>
                  {STATUSES.map((s) => (
                    <TableCell key={s} className="text-right tabular-nums">
                      {sum?.[s] ?? 0}
                    </TableCell>
                  ))}
                  {marked.map((s) => {
                    const v = status(st.id, s)
                    return (
                      <TableCell key={sessionKey(s)} className="px-1 text-center">
                        {v ? (
                          <span className={cn('inline-flex size-6 items-center justify-center rounded border text-[11px] font-semibold', ATTENDANCE_STYLE[v])} title={ATTENDANCE_LABELS[v]}>
                            {ATTENDANCE_LABELS[v][0]}
                          </span>
                        ) : (
                          <span className="text-muted-foreground">·</span>
                        )}
                      </TableCell>
                    )
                  })}
                </TableRow>
              )
            })}
          </TableBody>
        </Table>
      </CardContent>
      <CardFooter className="border-t pt-4 text-xs text-muted-foreground">Attendance counts present and late classes out of all marked classes; excused classes are left out.</CardFooter>
    </Card>
  )
}

export function AttendancePage() {
  useBreadcrumbs([{ label: 'Attendance' }])
  const { courses, course, select } = useSelectedCourse()
  const q = useQuery({ queryKey: ['attendance', course?.id], queryFn: () => coursesApi.attendance(course!.id), enabled: !!course })

  if (courses.isLoading || q.isLoading) return <PageSkeleton rows={1} />
  if (!course) return <EmptyState icon={CalendarCheck} title="No courses" />
  if (q.error || !q.data) return <ErrorState error={q.error} onRetry={() => q.refetch()} />

  return (
    <>
      <PageHeader
        eyebrow={course.title}
        title="Attendance"
        description="Take attendance for each scheduled class and follow every student's attendance rate."
        actions={<CourseSelect courses={courses.data ?? []} value={course.id} onChange={select} />}
      />
      {q.data.sessions.length === 0 ? (
        <EmptyState
          icon={CalendarCheck}
          title="No class schedule"
          description="Classes come from the course's weekly class schedule. Set it up in the course settings first."
          action={
            <Button asChild variant="outline">
              <Link to={`/admin/courses/${course.id}`}>Open course settings</Link>
            </Button>
          }
        />
      ) : (
        <Tabs defaultValue="take">
          <TabsList className="mb-4">
            <TabsTrigger value="take">Take attendance</TabsTrigger>
            <TabsTrigger value="overview">Overview</TabsTrigger>
          </TabsList>
          <TabsContent value="take">
            <TakeAttendance key={course.id} courseId={course.id} data={q.data} />
          </TabsContent>
          <TabsContent value="overview">
            <Overview data={q.data} />
          </TabsContent>
        </Tabs>
      )}
    </>
  )
}
