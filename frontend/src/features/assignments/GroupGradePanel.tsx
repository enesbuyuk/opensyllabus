import { useMutation, useQueryClient } from '@tanstack/react-query'
import { Crown, Users } from 'lucide-react'
import * as React from 'react'
import { toast } from 'sonner'
import { assignmentsApi } from '@/api/endpoints'
import { Button } from '@/components/ui/button'
import { Input } from '@/components/ui/input'
import { errorMessage } from '@/lib/errors'
import type { GroupInfo, GroupMember, SubmissionDetail } from '@/types/api'

function MemberRow({ detail, group, member }: { detail: SubmissionDetail; group: GroupInfo; member: GroupMember }) {
  const qc = useQueryClient()
  const [value, setValue] = React.useState(member.score_override != null ? String(member.score_override) : '')
  React.useEffect(() => setValue(member.score_override != null ? String(member.score_override) : ''), [member.score_override])
  const save = useMutation({
    mutationFn: (score: number | null) => assignmentsApi.setScoreOverride(group.assignment_id, group.id, member.student_id, score),
    onSuccess: (_, score) => {
      qc.invalidateQueries({ queryKey: ['submission', detail.id] })
      qc.invalidateQueries({ queryKey: ['submissions'] })
      qc.invalidateQueries({ queryKey: ['gradebook'] })
      toast.success(score == null ? `${member.name} gets the group grade again` : `Individual grade saved for ${member.name}`)
    },
    onError: (e) => toast.error(errorMessage(e)),
  })
  const current = member.score_override != null ? String(member.score_override) : ''
  return (
    <div className="flex items-center gap-2 rounded-lg border px-3 py-2">
      <span className="flex min-w-0 flex-1 items-center gap-1.5 truncate text-sm">
        {member.student_id === group.leader_id && <Crown className="size-3.5 shrink-0 text-info" aria-label="Leader" />}
        {member.name}
      </span>
      <Input
        type="number"
        inputMode="decimal"
        min={0}
        max={detail.assignment.max_grade}
        step="0.5"
        value={value}
        onChange={(e) => setValue(e.target.value)}
        placeholder={group.score != null ? String(group.score) : 'Group'}
        className="w-24 text-right"
        aria-label={`Individual grade for ${member.name}`}
      />
      <Button variant="outline" size="sm" onClick={() => save.mutate(value === '' ? null : Number(value))} loading={save.isPending} disabled={value === current}>
        Save
      </Button>
    </div>
  )
}

/** Per-student overrides of the shared group grade. */
export function GroupGradePanel({ detail }: { detail: SubmissionDetail }) {
  const group = detail.group
  if (!group) return null
  return (
    <div className="space-y-2">
      <div className="flex items-center gap-2 text-[13px] font-medium">
        <Users className="size-4 text-muted-foreground" /> {group.name}: individual grades
      </div>
      <p className="text-xs text-muted-foreground">
        Everyone gets the group grade{group.score != null ? ` (${group.score})` : ''}. Enter a grade to override it for one student, or clear it to go back to the group grade. Overrides apply once the group is graded.
      </p>
      {group.members.map((m) => (
        <MemberRow key={m.student_id} detail={detail} group={group} member={m} />
      ))}
    </div>
  )
}
