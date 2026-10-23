import { CheckCircle2, Circle, CircleDashed, Clock, FileEdit, Lock, RotateCcw, Send, TriangleAlert } from 'lucide-react'
import { Badge, type BadgeVariant } from './ui/badge'
import { STATE_LABELS, SUBMISSION_STATUS_LABELS } from '@/lib/labels'
import type { StudentAssignmentState, SubmissionStatus, WeekState } from '@/types/api'

const STATE_STYLE: Record<StudentAssignmentState | 'missing', { variant: BadgeVariant; icon: typeof Circle }> = {
  not_started: { variant: 'outline', icon: Circle },
  draft: { variant: 'secondary', icon: FileEdit },
  submitted: { variant: 'info', icon: Send },
  late: { variant: 'warning', icon: Clock },
  under_review: { variant: 'default', icon: CircleDashed },
  graded: { variant: 'success', icon: CheckCircle2 },
  revision_requested: { variant: 'warning', icon: RotateCcw },
  missing: { variant: 'destructive', icon: TriangleAlert },
}

export function AssignmentStateBadge({ state }: { state: StudentAssignmentState | 'missing' }) {
  const { variant, icon: Icon } = STATE_STYLE[state]
  return (
    <Badge variant={variant}>
      <Icon />
      {STATE_LABELS[state]}
    </Badge>
  )
}

export function SubmissionStatusBadge({ status, late }: { status: SubmissionStatus; late?: boolean }) {
  const state: StudentAssignmentState = status === 'submitted' && late ? 'late' : (status as StudentAssignmentState)
  if (state in STATE_STYLE) return <AssignmentStateBadge state={state} />
  return <Badge variant="outline">{SUBMISSION_STATUS_LABELS[status]}</Badge>
}

export function PublishBadge({ status }: { status: string }) {
  if (status === 'published') return <Badge variant="success">Published</Badge>
  if (status === 'archived') return <Badge variant="outline">Archived</Badge>
  return <Badge variant="secondary">Draft</Badge>
}

export function WeekStateBadge({ state }: { state: WeekState }) {
  switch (state) {
    case 'completed':
      return (
        <Badge variant="success">
          <CheckCircle2 /> Completed
        </Badge>
      )
    case 'current':
      return <Badge variant="default">Current week</Badge>
    case 'locked':
      return (
        <Badge variant="outline">
          <Lock /> Locked
        </Badge>
      )
    default:
      return <Badge variant="secondary">Available</Badge>
  }
}
