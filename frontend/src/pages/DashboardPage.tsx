import { useQuery } from '@tanstack/react-query'
import {
  AlarmClock,
  ArrowRight,
  BookOpen,
  CheckCircle2,
  Circle,
  ClipboardCheck,
  GraduationCap,
  Inbox,
  ListChecks,
  Pin,
  Star,
  TrendingUp,
  TriangleAlert,
  Users,
} from 'lucide-react'
import { Link } from 'react-router'
import { calendarApi, miscApi } from '@/api/endpoints'
import { SimpleBarChart } from '@/components/charts'
import { CourseBanner } from '@/components/course-banner'
import { classPhase, LiveClassCard } from '@/components/live-class'
import { EmptyState, ErrorState, PageHeader, PageSkeleton, StatCard } from '@/components/page'
import { AssignmentStateBadge, SubmissionStatusBadge } from '@/components/status'
import { Badge } from '@/components/ui/badge'
import { Button } from '@/components/ui/button'
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card'
import { Avatar, Progress, ProgressRing } from '@/components/ui/misc'
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from '@/components/ui/table'
import { useCourses } from '@/hooks/queries'
import { useUser } from '@/lib/auth'
import { useBreadcrumbs } from '@/lib/breadcrumbs'
import { cn, formatDate, formatDateTime, formatMinutes, formatScore, nameParts, relativeTime } from '@/lib/utils'
import type { AdminDashboard, AssignmentListItem, StudentDashboard } from '@/types/api'

const firstNameOf = (name: string) => nameParts(name)[0] ?? ''

function greeting() {
  const h = new Date().getHours()
  return h < 12 ? 'Good morning' : h < 18 ? 'Good afternoon' : 'Good evening'
}

export function DashboardPage() {
  useBreadcrumbs([{ label: 'Dashboard' }])
  const q = useQuery({ queryKey: ['dashboard'], queryFn: miscApi.dashboard })
  if (q.isLoading) return <PageSkeleton />
  if (q.error || !q.data) return <ErrorState error={q.error} onRetry={() => q.refetch()} />
  return q.data.role === 'admin' ? <InstructorDashboard data={q.data} /> : <StudentDashboardView data={q.data} />
}

function DeadlineRow({ a }: { a: AssignmentListItem }) {
  const { timezone } = useUser()
  const due = a.due_at ? new Date(a.due_at).getTime() - Date.now() : null
  const urgent = due != null && due < 48 * 3600_000
  return (
    <Link to={`/assignments/${a.id}`} className="flex items-center gap-3 rounded-lg px-2 py-2 transition hover:bg-accent focus-visible:bg-accent focus-visible:outline-none">
      <div className={cn('flex size-8 shrink-0 items-center justify-center rounded-md border', urgent ? 'border-warning/40 bg-warning/10 text-warning' : 'bg-muted/40 text-muted-foreground')}>
        <AlarmClock className="size-4" />
      </div>
      <div className="min-w-0 flex-1">
        <div className="truncate text-sm font-medium">{a.title}</div>
        <div className="truncate text-xs text-muted-foreground">
          {a.week_number ? `Week ${a.week_number} · ` : ''}
          {a.due_at ? `Due ${formatDateTime(a.due_at, timezone)} (${relativeTime(a.due_at)})` : 'No due date'}
        </div>
      </div>
      {a.state && <AssignmentStateBadge state={a.state} />}
    </Link>
  )
}

/** The next scheduled class of the coming week (or the one running now), with its join link. */
function NextClass() {
  const { timezone } = useUser()
  const q = useQuery({
    queryKey: ['calendar', 'next-class'],
    queryFn: () => {
      const now = new Date()
      return calendarApi.entries({ from: new Date(now.getTime() - 3 * 3600_000).toISOString(), to: new Date(now.getTime() + 7 * 86_400_000).toISOString() })
    },
    refetchInterval: 5 * 60_000,
  })
  const next = (q.data ?? [])
    .filter((e) => e.kind === 'class' && classPhase(e.starts_at, e.ends_at) !== 'ended')
    .sort((a, b) => a.starts_at.localeCompare(b.starts_at))[0]
  if (!next) return null
  const live = classPhase(next.starts_at, next.ends_at) === 'live'
  return (
    <LiveClassCard
      url={next.meeting_url}
      startsAt={next.starts_at}
      endsAt={next.ends_at}
      timezone={timezone}
      title={`${live ? 'Class now' : 'Next class'}: ${next.title}`}
      lessonLink={next.link ?? undefined}
    />
  )
}

/** Cover cards for the given courses (all published ones when no id is passed). */
function CourseBanners({ courseId, showStudents = false }: { courseId?: string; showStudents?: boolean }) {
  const q = useCourses()
  const courses = (q.data ?? []).filter((c) => (courseId ? c.id === courseId : c.status === 'published')).slice(0, 4)
  if (!courses.length) return null
  return (
    <div className={cn('grid gap-4', courses.length > 1 && 'lg:grid-cols-2')}>
      {courses.map((c) => (
        <CourseBanner key={c.id} course={c} showStudents={showStudents} />
      ))}
    </div>
  )
}

function StudentDashboardView({ data }: { data: StudentDashboard }) {
  const { user, timezone } = useUser()
  const course = data.courses[0]
  const firstName = firstNameOf(user.name)

  if (!course) {
    return (
      <>
        <PageHeader title={`${greeting()}, ${firstName}`} description="Welcome to OpenSyllabus." />
        <EmptyState icon={GraduationCap} title="No active courses yet" description="Your instructor will enroll you in a course. It will appear here once it is published." />
      </>
    )
  }

  const weekChart = course.weeks
    .filter((w) => w.state !== 'locked')
    .slice(-12)
    .map((w) => ({ week: `W${w.week_number}`, completion: w.completion_percent }))

  return (
    <div className="space-y-6">
      <PageHeader
        eyebrow={course.course_title}
        title={`${greeting()}, ${firstName}`}
        description={
          data.current_week
            ? `You are in week ${data.current_week.week_number}: ${data.current_week.title}.`
            : course.current_week
              ? `Week ${course.current_week} of the course.`
              : 'Your course has not started yet.'
        }
        actions={
          data.continue_learning && (
            <Button asChild>
              <Link to={`/lessons/${data.continue_learning.lesson_id}`}>
                Continue learning <ArrowRight />
              </Link>
            </Button>
          )
        }
      />

      <CourseBanners courseId={course.course_id} />

      <div className="grid gap-4 sm:grid-cols-2 lg:grid-cols-4">
        <div className="flex items-center gap-4 rounded-xl border bg-card p-5 shadow-xs">
          <ProgressRing value={course.completion_percent} size={64} stroke={6} />
          <div className="min-w-0">
            <div className="text-base font-semibold">Course completion</div>
            <div className="text-xs text-muted-foreground">{course.current_week ? `Week ${course.current_week} of ${course.weeks.length}` : 'Not started yet'}</div>
          </div>
        </div>
        <StatCard label="Lessons completed" value={`${course.lessons_completed} / ${course.lessons_total}`} icon={BookOpen} hint={`${course.lesson_percent}% of available lessons`} />
        <StatCard
          label="Pending assignments"
          value={course.assignments_pending}
          icon={ListChecks}
          tone={course.assignments_pending ? 'warning' : 'default'}
          hint={`${course.assignments_awaiting_review} awaiting review`}
        />
        <StatCard label="Average grade" value={course.average_grade_percent != null ? `${course.average_grade_percent}%` : '—'} icon={Star} hint={`${course.grades.length} graded`} />
      </div>

      <NextClass />

      <div className="grid gap-6 lg:grid-cols-3">
        <Card className="lg:col-span-2">
          <CardHeader className="flex-row items-start justify-between">
            <div className="space-y-1">
              <CardTitle>This week</CardTitle>
              <CardDescription>{data.current_week ? `Week ${data.current_week.week_number} · ${data.current_week.title}` : 'No week is open right now'}</CardDescription>
            </div>
            {data.current_week && (
              <Button asChild variant="ghost" size="sm">
                <Link to={`/weeks/${data.current_week.week_id}`}>
                  Open week <ArrowRight />
                </Link>
              </Button>
            )}
          </CardHeader>
          <CardContent>
            {data.current_week?.lessons.length ? (
              <div className="grid gap-3 sm:grid-cols-3">
                {data.current_week.lessons.map((l) => (
                  <Link key={l.id} to={`/lessons/${l.id}`} className="group rounded-lg border bg-muted/20 p-3 transition hover:border-primary/40 hover:bg-muted/40 focus-visible:ring-2 focus-visible:ring-ring/50 focus-visible:outline-none">
                    <div className="flex items-center justify-between text-xs text-muted-foreground">
                      <span>Day {l.day_number}</span>
                      {l.completed ? <CheckCircle2 className="size-4 text-success" aria-label="Completed" /> : <Circle className="size-4" aria-label="Not completed" />}
                    </div>
                    <div className="mt-2 line-clamp-2 text-sm font-medium group-hover:text-primary">{l.title}</div>
                    <div className="mt-1 text-xs text-muted-foreground">{formatMinutes(l.estimated_minutes)}</div>
                  </Link>
                ))}
              </div>
            ) : (
              <p className="text-sm text-muted-foreground">Lessons for this week have not been published yet.</p>
            )}
            {data.current_week && (
              <div className="mt-4 flex items-center gap-3">
                <Progress value={data.current_week.completion_percent} />
                <span className="text-xs whitespace-nowrap text-muted-foreground tabular-nums">{data.current_week.completion_percent}% of this week</span>
              </div>
            )}
          </CardContent>
        </Card>

        <Card>
          <CardHeader>
            <CardTitle>Upcoming deadlines</CardTitle>
            <CardDescription>Next 14 days</CardDescription>
          </CardHeader>
          <CardContent className="px-3">
            {data.upcoming_deadlines.length ? (
              <div className="space-y-0.5">
                {data.upcoming_deadlines.map((a) => (
                  <DeadlineRow key={a.id} a={a} />
                ))}
              </div>
            ) : (
              <p className="px-2 text-sm text-muted-foreground">Nothing due in the next two weeks.</p>
            )}
          </CardContent>
        </Card>
      </div>

      <div className="grid gap-6 lg:grid-cols-3">
        <Card className="lg:col-span-2">
          <CardHeader className="flex-row items-start justify-between">
            <div className="space-y-1">
              <CardTitle>Pending assignments</CardTitle>
              <CardDescription>Not yet submitted, including revision requests</CardDescription>
            </div>
            <Button asChild variant="ghost" size="sm">
              <Link to="/assignments">
                View all <ArrowRight />
              </Link>
            </Button>
          </CardHeader>
          <CardContent className="px-3">
            {data.pending_assignments.length ? (
              <div className="space-y-0.5">
                {data.pending_assignments.map((a) => (
                  <DeadlineRow key={a.id} a={a} />
                ))}
              </div>
            ) : (
              <p className="px-2 text-sm text-muted-foreground">You're all caught up.</p>
            )}
          </CardContent>
        </Card>

        <Card>
          <CardHeader>
            <CardTitle>Latest grades</CardTitle>
            <CardDescription>Returned by your instructor</CardDescription>
          </CardHeader>
          <CardContent className="space-y-2">
            {data.latest_grades.length ? (
              data.latest_grades.map((g) => (
                <Link key={g.assignment_id} to={`/assignments/${g.assignment_id}`} className="flex items-center gap-3 rounded-lg px-2 py-1.5 hover:bg-accent">
                  <div className="min-w-0 flex-1">
                    <div className="truncate text-sm">{g.assignment_title}</div>
                    <div className="text-xs text-muted-foreground">{formatDate(g.released_at, timezone)}</div>
                  </div>
                  <span className="text-sm font-medium tabular-nums">{formatScore(g.score, g.max_score)}</span>
                </Link>
              ))
            ) : (
              <p className="text-sm text-muted-foreground">No grades yet.</p>
            )}
          </CardContent>
        </Card>
      </div>

      <div className="grid gap-6 lg:grid-cols-3">
        <Card className="lg:col-span-2">
          <CardHeader>
            <CardTitle>Weekly completion (%)</CardTitle>
            <CardDescription>Lessons and graded assignments completed per open week</CardDescription>
          </CardHeader>
          <CardContent>
            {weekChart.length ? (
              <SimpleBarChart data={weekChart} xKey="week" series={[{ key: 'completion', name: 'Completion' }]} unit="%" max={100} ariaLabel="Completion percentage per week" />
            ) : (
              <p className="text-sm text-muted-foreground">No weeks are open yet.</p>
            )}
          </CardContent>
        </Card>
        <Card>
          <CardHeader className="flex-row items-start justify-between">
            <div className="space-y-1">
              <CardTitle>Announcements</CardTitle>
              <CardDescription>From your instructor</CardDescription>
            </div>
            <Button asChild variant="ghost" size="sm">
              <Link to="/announcements">All</Link>
            </Button>
          </CardHeader>
          <CardContent className="space-y-4">
            {data.announcements.length ? (
              data.announcements.map((a) => (
                <Link key={a.id} to={`/announcements#${a.id}`} className="block rounded-lg px-2 py-1 hover:bg-accent">
                  <div className="flex items-center gap-2 text-sm font-medium">
                    {a.pinned && <Pin className="size-3.5 text-primary" aria-label="Pinned" />}
                    <span className="truncate">{a.title}</span>
                  </div>
                  <div className="text-xs text-muted-foreground">{relativeTime(a.published_at)}</div>
                </Link>
              ))
            ) : (
              <p className="text-sm text-muted-foreground">No announcements yet.</p>
            )}
          </CardContent>
        </Card>
      </div>
    </div>
  )
}

function InstructorDashboard({ data }: { data: AdminDashboard }) {
  const { user, timezone } = useUser()
  const chart = data.weekly_activity.map((w) => ({ week: formatDate(w.week_start + 'T12:00:00Z', 'UTC').replace(/ \d{4}$/, ''), submissions: w.submissions, lessons: w.lessons_completed }))
  return (
    <div className="space-y-6">
      <PageHeader
        title={`${greeting()}, ${firstNameOf(user.name)}`}
        description="Here's what needs your attention today."
        actions={
          <Button asChild>
            <Link to="/admin/reviews">
              <ClipboardCheck /> Review submissions
            </Link>
          </Button>
        }
      />
      <CourseBanners showStudents />
      <div className="grid gap-4 sm:grid-cols-2 lg:grid-cols-5">
        <StatCard label="Active students" value={data.total_students} icon={Users} />
        <StatCard label="Active courses" value={data.active_courses} icon={GraduationCap} />
        <StatCard label="Pending reviews" value={data.pending_submissions} icon={Inbox} tone={data.pending_submissions ? 'warning' : 'default'} />
        <StatCard label="Overdue" value={data.overdue_assignments} icon={TriangleAlert} tone={data.overdue_assignments ? 'destructive' : 'default'} hint="Missing submissions" />
        <StatCard label="Avg. completion" value={data.average_completion != null ? `${data.average_completion}%` : '—'} icon={TrendingUp} />
      </div>

      <NextClass />

      <div className="grid gap-6 lg:grid-cols-3">
        <Card className="lg:col-span-2">
          <CardHeader>
            <CardTitle>Weekly activity</CardTitle>
            <CardDescription>Submitted attempts and completed lessons per week (last 8 weeks)</CardDescription>
          </CardHeader>
          <CardContent>
            <SimpleBarChart
              data={chart}
              xKey="week"
              series={[
                { key: 'submissions', name: 'Submissions' },
                { key: 'lessons', name: 'Lessons completed' },
              ]}
              ariaLabel="Weekly submissions and lesson completions"
            />
          </CardContent>
        </Card>
        <Card>
          <CardHeader>
            <CardTitle>Upcoming deadlines</CardTitle>
            <CardDescription>Next 14 days</CardDescription>
          </CardHeader>
          <CardContent className="px-3">
            {data.upcoming_deadlines.length ? (
              data.upcoming_deadlines.map((a) => (
                <Link key={a.id} to={`/assignments/${a.id}`} className="flex items-center gap-3 rounded-lg px-2 py-2 hover:bg-accent">
                  <AlarmClock className="size-4 shrink-0 text-muted-foreground" />
                  <div className="min-w-0 flex-1">
                    <div className="truncate text-sm">{a.title}</div>
                    <div className="text-xs text-muted-foreground">{formatDateTime(a.due_at, timezone)}</div>
                  </div>
                  <Badge variant="outline">{a.submission_count ?? 0} in</Badge>
                </Link>
              ))
            ) : (
              <p className="px-2 text-sm text-muted-foreground">No deadlines in the next two weeks.</p>
            )}
          </CardContent>
        </Card>
      </div>

      <div className="grid gap-6 lg:grid-cols-3">
        <Card className="lg:col-span-2">
          <CardHeader className="flex-row items-start justify-between">
            <div className="space-y-1">
              <CardTitle>Needs review</CardTitle>
              <CardDescription>Most recent submissions waiting for feedback</CardDescription>
            </div>
            <Button asChild variant="ghost" size="sm">
              <Link to="/admin/reviews">
                Open workspace <ArrowRight />
              </Link>
            </Button>
          </CardHeader>
          <CardContent className="px-0">
            {data.review_queue.length ? (
              <Table>
                <TableHeader>
                  <TableRow>
                    <TableHead className="pl-5">Student</TableHead>
                    <TableHead>Assignment</TableHead>
                    <TableHead>Submitted</TableHead>
                    <TableHead className="pr-5">Status</TableHead>
                  </TableRow>
                </TableHeader>
                <TableBody>
                  {data.review_queue.map((s) => (
                    <TableRow key={s.id}>
                      <TableCell className="pl-5">
                        <Link to={`/admin/reviews?submission=${s.id}`} className="flex items-center gap-2 font-medium hover:underline">
                          <Avatar name={s.student_name} className="size-6 text-[10px]" />
                          {s.student_name}
                        </Link>
                      </TableCell>
                      <TableCell className="max-w-56 truncate">{s.assignment_title}</TableCell>
                      <TableCell className="text-muted-foreground">{relativeTime(s.submitted_at)}</TableCell>
                      <TableCell className="pr-5">
                        <SubmissionStatusBadge status={s.status} late={s.is_late} />
                      </TableCell>
                    </TableRow>
                  ))}
                </TableBody>
              </Table>
            ) : (
              <div className="px-5">
                <EmptyState icon={CheckCircle2} title="Review queue is empty" description="New submissions will appear here." />
              </div>
            )}
          </CardContent>
        </Card>
        <Card>
          <CardHeader>
            <CardTitle>Recent activity</CardTitle>
          </CardHeader>
          <CardContent className="space-y-3">
            {data.recent_activity.length ? (
              data.recent_activity.map((a, i) => (
                <Link key={i} to={a.link} className="flex gap-3 rounded-md px-1 py-0.5 hover:bg-accent">
                  <Avatar name={a.student_name} className="size-7 text-[10px]" />
                  <div className="min-w-0 text-[13px]">
                    <span className="font-medium">{a.student_name}</span>{' '}
                    <span className="text-muted-foreground">{a.kind === 'submission' ? 'submitted' : 'completed'}</span> <span className="break-words">{a.title}</span>
                    <div className="text-xs text-muted-foreground">{relativeTime(a.at)}</div>
                  </div>
                </Link>
              ))
            ) : (
              <p className="text-sm text-muted-foreground">No student activity yet.</p>
            )}
          </CardContent>
        </Card>
      </div>

      <Card>
        <CardHeader className="flex-row items-start justify-between">
          <div className="space-y-1">
            <CardTitle>Student progress</CardTitle>
            <CardDescription>Completion counts lessons and returned (graded) assignments</CardDescription>
          </div>
          <Button asChild variant="ghost" size="sm">
            <Link to="/admin/gradebook">
              Gradebook <ArrowRight />
            </Link>
          </Button>
        </CardHeader>
        <CardContent className="px-0">
          {data.student_overview.length ? (
            <Table>
              <TableHeader>
                <TableRow>
                  <TableHead className="pl-5">Student</TableHead>
                  <TableHead className="hidden md:table-cell">Course</TableHead>
                  <TableHead className="w-48">Completion</TableHead>
                  <TableHead>Avg. grade</TableHead>
                  <TableHead>Pending</TableHead>
                  <TableHead className="hidden pr-5 sm:table-cell">Last sign-in</TableHead>
                </TableRow>
              </TableHeader>
              <TableBody>
                {data.student_overview.map((s) => (
                  <TableRow key={`${s.course_id}-${s.student_id}`}>
                    <TableCell className="pl-5">
                      <Link to={`/progress?student=${s.student_id}&course=${s.course_id}`} className="flex items-center gap-2 font-medium hover:underline">
                        <Avatar name={s.name} className="size-6 text-[10px]" />
                        {s.name}
                      </Link>
                    </TableCell>
                    <TableCell className="hidden max-w-48 truncate text-muted-foreground md:table-cell">{s.course_title}</TableCell>
                    <TableCell>
                      <div className="flex items-center gap-2">
                        <Progress value={s.completion_percent} className="w-24" />
                        <span className="text-xs tabular-nums">{s.completion_percent}%</span>
                      </div>
                    </TableCell>
                    <TableCell className="tabular-nums">{s.average_grade_percent != null ? `${s.average_grade_percent}%` : '—'}</TableCell>
                    <TableCell className="tabular-nums">{s.assignments_pending}</TableCell>
                    <TableCell className="hidden pr-5 text-muted-foreground sm:table-cell">{s.last_login_at ? relativeTime(s.last_login_at) : 'Never'}</TableCell>
                  </TableRow>
                ))}
              </TableBody>
            </Table>
          ) : (
            <div className="px-5">
              <EmptyState
                icon={Users}
                title="No enrolled students"
                description="Create student accounts and enroll them in a published course."
                action={
                  <Button asChild variant="outline" size="sm">
                    <Link to="/admin/students">Manage students</Link>
                  </Button>
                }
              />
            </div>
          )}
        </CardContent>
      </Card>
    </div>
  )
}
