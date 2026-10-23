import { useQuery } from '@tanstack/react-query'
import { ListChecks, Plus } from 'lucide-react'
import * as React from 'react'
import { Link } from 'react-router'
import { assignmentsApi } from '@/api/endpoints'
import { AssignmentRows } from '@/components/assignment-list'
import { CourseSelect } from '@/components/course-select'
import { EmptyState, ErrorState, PageHeader, PageSkeleton } from '@/components/page'
import { Button } from '@/components/ui/button'
import { Tabs, TabsList, TabsTrigger } from '@/components/ui/tabs'
import { useSelectedCourse } from '@/hooks/queries'
import { useAuth } from '@/lib/auth'
import { useBreadcrumbs } from '@/lib/breadcrumbs'
import type { AssignmentListItem } from '@/types/api'

const STUDENT_TABS = {
  todo: { label: 'To do', match: (a: AssignmentListItem) => ['not_started', 'draft', 'revision_requested'].includes(a.state ?? '') },
  submitted: { label: 'Submitted', match: (a: AssignmentListItem) => ['submitted', 'late', 'under_review'].includes(a.state ?? '') },
  graded: { label: 'Graded', match: (a: AssignmentListItem) => a.state === 'graded' },
  all: { label: 'All', match: () => true },
}
const ADMIN_TABS = {
  published: { label: 'Published', match: (a: AssignmentListItem) => a.status === 'published' },
  draft: { label: 'Drafts', match: (a: AssignmentListItem) => a.status === 'draft' },
  archived: { label: 'Archived', match: (a: AssignmentListItem) => a.status === 'archived' },
  all: { label: 'All', match: () => true },
}

export function AssignmentsPage() {
  useBreadcrumbs([{ label: 'Assignments' }])
  const { isAdmin } = useAuth()
  const { courses, course, select } = useSelectedCourse()
  const tabs = isAdmin ? ADMIN_TABS : STUDENT_TABS
  const [tab, setTab] = React.useState<string>(isAdmin ? 'published' : 'todo')
  const q = useQuery({ queryKey: ['assignments', course?.id], queryFn: () => assignmentsApi.list({ course_id: course?.id }), enabled: !!course })

  if (courses.isLoading || (course && q.isLoading)) return <PageSkeleton />
  if (!course) return <EmptyState icon={ListChecks} title="No courses available" description="Assignments appear once you are enrolled in a course." />
  if (q.error) return <ErrorState error={q.error} onRetry={() => q.refetch()} />
  const items = q.data ?? []
  const current = (tabs as Record<string, { label: string; match: (a: AssignmentListItem) => boolean }>)[tab]
  const filtered = items.filter(current.match)

  return (
    <>
      <PageHeader
        eyebrow={course.title}
        title="Assignments"
        description={isAdmin ? 'Create, publish and track assignments.' : 'Your assignments, deadlines and grades.'}
        actions={
          <>
            <CourseSelect courses={courses.data ?? []} value={course.id} onChange={select} />
            {isAdmin && (
              <Button asChild>
                <Link to={`/admin/assignments/new?course=${course.id}`}>
                  <Plus /> New assignment
                </Link>
              </Button>
            )}
          </>
        }
      />
      <Tabs value={tab} onValueChange={setTab} className="mb-4">
        <TabsList>
          {Object.entries(tabs).map(([k, t]) => (
            <TabsTrigger key={k} value={k}>
              {t.label}
              <span className="text-xs text-muted-foreground tabular-nums">{items.filter(t.match).length}</span>
            </TabsTrigger>
          ))}
        </TabsList>
      </Tabs>
      {filtered.length ? (
        <AssignmentRows items={filtered} />
      ) : (
        <EmptyState icon={ListChecks} title="Nothing here" description={tab === 'todo' ? "You're all caught up." : 'No assignments match this filter.'} />
      )}
    </>
  )
}
