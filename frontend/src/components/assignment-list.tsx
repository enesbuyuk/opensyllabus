import { ChevronRight, ClipboardList } from 'lucide-react'
import { Link } from 'react-router'
import { ASSIGNMENT_KIND_LABELS } from '@/lib/labels'
import { useAuth } from '@/lib/auth'
import { cn, formatDateTime, formatScore, isPast } from '@/lib/utils'
import type { AssignmentListItem } from '@/types/api'
import { AssignmentStateBadge, PublishBadge } from './status'
import { Badge } from './ui/badge'

export function AssignmentRows({ items, showCourse, emptyText = 'No assignments.' }: { items: AssignmentListItem[]; showCourse?: boolean; emptyText?: string }) {
  const { timezone, isAdmin } = useAuth()
  if (!items.length) return <p className="text-[13px] text-muted-foreground">{emptyText}</p>
  return (
    <ul className="divide-y overflow-hidden rounded-lg border bg-card">
      {items.map((a) => {
        const overdue = !isAdmin && a.due_at && isPast(a.due_at) && (a.state === 'not_started' || a.state === 'draft')
        return (
          <li key={a.id}>
            <Link to={`/assignments/${a.id}`} className="flex items-center gap-3 px-4 py-3 transition hover:bg-accent/60 focus-visible:bg-accent focus-visible:outline-none">
              <ClipboardList className="size-4 shrink-0 text-muted-foreground" />
              <div className="min-w-0 flex-1">
                <div className="truncate text-sm font-medium">{a.title}</div>
                <div className="mt-0.5 flex flex-wrap gap-x-3 text-xs text-muted-foreground">
                  {showCourse && <span>{a.course_title}</span>}
                  {a.week_number && <span>Week {a.week_number}</span>}
                  <span>{ASSIGNMENT_KIND_LABELS[a.kind]}</span>
                  <span className={cn(overdue && 'font-medium text-destructive')}>{a.due_at ? `Due ${formatDateTime(a.due_at, timezone)}` : 'No due date'}</span>
                </div>
              </div>
              <div className="flex shrink-0 items-center gap-2">
                {isAdmin ? (
                  <>
                    {!!a.pending_review_count && <Badge variant="warning">{a.pending_review_count} to review</Badge>}
                    <span className="hidden text-xs text-muted-foreground sm:inline">{a.submission_count ?? 0} submitted</span>
                    <PublishBadge status={a.status} />
                  </>
                ) : (
                  <>
                    {a.score != null && <span className="text-sm font-medium tabular-nums">{formatScore(a.score, a.max_grade)}</span>}
                    {overdue ? <AssignmentStateBadge state="missing" /> : a.state && <AssignmentStateBadge state={a.state} />}
                  </>
                )}
                <ChevronRight className="size-4 text-muted-foreground" />
              </div>
            </Link>
          </li>
        )
      })}
    </ul>
  )
}
