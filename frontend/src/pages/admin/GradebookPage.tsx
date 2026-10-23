import { useQuery } from '@tanstack/react-query'
import { Download, Sheet } from 'lucide-react'
import * as React from 'react'
import { Link } from 'react-router'
import { coursesApi } from '@/api/endpoints'
import { CourseSelect } from '@/components/course-select'
import { EmptyState, ErrorState, PageHeader, PageSkeleton } from '@/components/page'
import { Button } from '@/components/ui/button'
import { NativeSelect } from '@/components/ui/input'
import { Tabs, TabsList, TabsTrigger } from '@/components/ui/tabs'
import { Tooltip } from '@/components/ui/tooltip'
import { useSelectedCourse } from '@/hooks/queries'
import { useBreadcrumbs } from '@/lib/breadcrumbs'
import { STATE_LABELS } from '@/lib/labels'
import { cn, formatScore } from '@/lib/utils'
import type { GradebookCell } from '@/types/api'

const CELL_STYLE: Record<GradebookCell['state'], string> = {
  not_started: 'text-muted-foreground',
  draft: 'text-muted-foreground',
  submitted: 'bg-info/10 text-info',
  late: 'bg-warning/10 text-warning',
  under_review: 'bg-primary/10 text-primary',
  graded: '',
  revision_requested: 'bg-warning/10 text-warning',
  missing: 'bg-destructive/10 text-destructive',
}

const SHORT: Record<GradebookCell['state'], string> = {
  not_started: '—',
  draft: 'Draft',
  submitted: 'Submitted',
  late: 'Late',
  under_review: 'Reviewing',
  graded: '',
  revision_requested: 'Revise',
  missing: 'Missing',
}

export function GradebookPage() {
  useBreadcrumbs([{ label: 'Gradebook' }])
  const { courses, course, select } = useSelectedCourse()
  const [mode, setMode] = React.useState<'assignments' | 'weeks'>('assignments')
  const [weekId, setWeekId] = React.useState('')
  const curriculum = useQuery({ queryKey: ['curriculum', course?.id], queryFn: () => coursesApi.curriculum(course!.id), enabled: !!course })
  const q = useQuery({ queryKey: ['gradebook', course?.id, weekId], queryFn: () => coursesApi.gradebook(course!.id, { week_id: weekId || undefined }), enabled: !!course })

  if (courses.isLoading || q.isLoading) return <PageSkeleton rows={1} />
  if (!course) return <EmptyState icon={Sheet} title="No courses" />
  if (q.error || !q.data) return <ErrorState error={q.error} onRetry={() => q.refetch()} />
  const gb = q.data
  const csvHref = `/api/v1/courses/${course.id}/gradebook.csv${weekId ? `?week_id=${weekId}` : ''}`

  return (
    <>
      <PageHeader
        eyebrow={course.title}
        title="Gradebook"
        description="Grades and submission states per student. Unreturned grades are shown in italics."
        actions={
          <>
            <CourseSelect courses={courses.data ?? []} value={course.id} onChange={select} />
            <Button asChild variant="outline">
              <a href={csvHref} download>
                <Download /> Export CSV
              </a>
            </Button>
          </>
        }
      />
      <div className="mb-4 flex flex-wrap items-center gap-3">
        <Tabs value={mode} onValueChange={(v) => setMode(v as 'assignments' | 'weeks')}>
          <TabsList>
            <TabsTrigger value="assignments">By assignment</TabsTrigger>
            <TabsTrigger value="weeks">By week</TabsTrigger>
          </TabsList>
        </Tabs>
        {mode === 'assignments' && (
          <NativeSelect value={weekId} onChange={(e) => setWeekId(e.target.value)} className="w-48" aria-label="Filter by week">
            <option value="">All weeks</option>
            {curriculum.data?.weeks.map((w) => (
              <option key={w.id} value={w.id}>
                Week {w.week_number}
              </option>
            ))}
          </NativeSelect>
        )}
      </div>
      {gb.rows.length === 0 ? (
        <EmptyState icon={Sheet} title="No students enrolled" />
      ) : (
        <div className="overflow-x-auto rounded-xl border bg-card">
          <table className="w-full border-separate border-spacing-0 text-sm">
            <thead>
              <tr>
                <th className="sticky left-0 z-10 min-w-44 border-b bg-card px-4 py-2 text-left text-xs font-medium text-muted-foreground">Student</th>
                <th className="border-b px-3 py-2 text-right text-xs font-medium whitespace-nowrap text-muted-foreground">Completion</th>
                <th className="border-b px-3 py-2 text-right text-xs font-medium whitespace-nowrap text-muted-foreground">Avg. grade</th>
                {mode === 'assignments'
                  ? gb.columns.map((c) => (
                      <th key={c.id} className="min-w-28 border-b border-l px-3 py-2 text-left align-bottom text-xs font-medium text-muted-foreground">
                        <Link to={`/assignments/${c.id}`} className="line-clamp-2 hover:text-foreground" title={c.title}>
                          {c.week_number ? `W${c.week_number} · ` : ''}
                          {c.title}
                        </Link>
                        <div className="font-normal opacity-70">/ {c.max_grade}</div>
                      </th>
                    ))
                  : gb.week_numbers.map((n) => (
                      <th key={n} className="min-w-14 border-b border-l px-2 py-2 text-center text-xs font-medium text-muted-foreground">
                        W{n}
                      </th>
                    ))}
              </tr>
            </thead>
            <tbody>
              {gb.rows.map((r) => (
                <tr key={r.student.id} className="hover:bg-muted/30">
                  <td className="sticky left-0 z-10 border-b bg-card px-4 py-2">
                    <Link to={`/progress?student=${r.student.id}&course=${course.id}`} className="font-medium hover:underline">
                      {r.student.name}
                    </Link>
                  </td>
                  <td className="border-b px-3 py-2 text-right tabular-nums">{r.completion_percent}%</td>
                  <td className="border-b px-3 py-2 text-right tabular-nums">{r.average_grade_percent != null ? `${r.average_grade_percent}%` : '—'}</td>
                  {mode === 'assignments'
                    ? gb.columns.map((c) => {
                        const cell = r.cells[c.id]
                        const content =
                          cell.score != null && (cell.state === 'graded' || !cell.released) ? (
                            <span className={cn('tabular-nums', !cell.released && 'italic opacity-70')}>{formatScore(cell.score)}</span>
                          ) : (
                            SHORT[cell.state]
                          )
                        return (
                          <td key={c.id} className="border-b border-l p-0">
                            <Tooltip content={`${STATE_LABELS[cell.state]}${cell.attempt > 1 ? ` · attempt ${cell.attempt}` : ''}${cell.is_late ? ' · late' : ''}${cell.score != null && !cell.released ? ' · not returned' : ''}`}>
                              {cell.submission_id ? (
                                <Link to={`/admin/reviews?submission=${cell.submission_id}`} className={cn('block px-3 py-2 text-xs hover:underline', CELL_STYLE[cell.state])}>
                                  {content}
                                  {cell.is_late && cell.state === 'graded' && <span className="ml-1 text-warning">•</span>}
                                </Link>
                              ) : (
                                <span className={cn('block px-3 py-2 text-xs', CELL_STYLE[cell.state])}>{content}</span>
                              )}
                            </Tooltip>
                          </td>
                        )
                      })
                    : gb.week_numbers.map((n) => {
                        const v = r.weeks[String(n)] ?? 0
                        return (
                          <td key={n} className="border-b border-l px-2 py-2 text-center text-xs tabular-nums" style={{ background: `color-mix(in oklch, var(--chart-1) ${Math.round(v * 0.45)}%, transparent)` }}>
                            {Math.round(v)}
                          </td>
                        )
                      })}
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      )}
      {mode === 'weeks' && <p className="mt-3 text-xs text-muted-foreground">Cells show the week completion percentage (lessons completed and assignments returned with a grade).</p>}
    </>
  )
}
