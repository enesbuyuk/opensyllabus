import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query'
import { Crown, Pencil, Plus, Trash2, Users } from 'lucide-react'
import * as React from 'react'
import { toast } from 'sonner'
import { assignmentsApi, coursesApi } from '@/api/endpoints'
import { Field } from '@/components/form'
import { ConfirmDialog } from '@/components/ui/alert-dialog'
import { Badge } from '@/components/ui/badge'
import { Button } from '@/components/ui/button'
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card'
import { Dialog, DialogContent, DialogFooter, DialogHeader, DialogTitle } from '@/components/ui/dialog'
import { Input, NativeSelect } from '@/components/ui/input'
import { Checkbox } from '@/components/ui/misc'
import { errorMessage } from '@/lib/errors'
import type { AssignmentDetail, EnrollmentItem, GroupInfo } from '@/types/api'

function GroupDialog({
  assignment,
  group,
  groups,
  students,
  open,
  onOpenChange,
}: {
  assignment: AssignmentDetail
  group: GroupInfo | null
  groups: GroupInfo[]
  students: EnrollmentItem[]
  open: boolean
  onOpenChange: (o: boolean) => void
}) {
  const qc = useQueryClient()
  const [name, setName] = React.useState('')
  const [members, setMembers] = React.useState<string[]>([])
  const [leader, setLeader] = React.useState('')
  React.useEffect(() => {
    if (open) {
      setName(group?.name ?? `Group ${groups.length + 1}`)
      setMembers(group?.members.map((m) => m.student_id) ?? [])
      setLeader(group?.leader_id ?? '')
    }
  }, [open, group, groups.length])

  // Students already in another group cannot be picked.
  const taken = new Set(groups.filter((g) => g.id !== group?.id).flatMap((g) => g.members.map((m) => m.student_id)))
  const available = students.filter((s) => !taken.has(s.user_id))
  const chosen = available.filter((s) => members.includes(s.user_id))
  const effectiveLeader = members.includes(leader) ? leader : (chosen[0]?.user_id ?? '')

  const save = useMutation({
    mutationFn: () => {
      const body = { name, leader_id: effectiveLeader, member_ids: members }
      return group ? assignmentsApi.updateGroup(assignment.id, group.id, body) : assignmentsApi.createGroup(assignment.id, body)
    },
    onSuccess: () => {
      qc.invalidateQueries({ queryKey: ['assignment-groups', assignment.id] })
      qc.invalidateQueries({ queryKey: ['assignment-submissions', assignment.id] })
      toast.success('Group saved')
      onOpenChange(false)
    },
    onError: (e) => toast.error(errorMessage(e)),
  })

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent>
        <DialogHeader>
          <DialogTitle>{group ? 'Edit group' : 'New group'}</DialogTitle>
        </DialogHeader>
        <Field label="Name" htmlFor="g-name" required>
          <Input id="g-name" value={name} onChange={(e) => setName(e.target.value)} />
        </Field>
        <Field label="Members" hint={taken.size ? 'Students already in another group are not listed.' : undefined}>
          <div className="max-h-64 space-y-0.5 overflow-y-auto rounded-md border p-1">
            {available.length === 0 ? (
              <p className="px-2 py-1.5 text-sm text-muted-foreground">Every enrolled student is already in a group.</p>
            ) : (
              available.map((s) => (
                <label key={s.user_id} className="flex cursor-pointer items-center gap-3 rounded-md px-2 py-1.5 hover:bg-accent">
                  <Checkbox checked={members.includes(s.user_id)} onCheckedChange={(c) => setMembers((m) => (c ? [...m, s.user_id] : m.filter((x) => x !== s.user_id)))} />
                  <span className="min-w-0">
                    <span className="block truncate text-sm">{s.name}</span>
                    <span className="block truncate text-xs text-muted-foreground">{s.email}</span>
                  </span>
                </label>
              ))
            )}
          </div>
        </Field>
        <Field label="Group leader" htmlFor="g-leader" hint="Only the leader can upload and submit for the group.">
          <NativeSelect id="g-leader" value={effectiveLeader} onChange={(e) => setLeader(e.target.value)} disabled={!chosen.length}>
            {chosen.map((s) => (
              <option key={s.user_id} value={s.user_id}>
                {s.name}
              </option>
            ))}
          </NativeSelect>
        </Field>
        <DialogFooter>
          <Button variant="outline" onClick={() => onOpenChange(false)}>
            Cancel
          </Button>
          <Button onClick={() => save.mutate()} loading={save.isPending} disabled={!name.trim() || !effectiveLeader}>
            Save group
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  )
}

/** Instructor view of a group assignment's groups. */
export function GroupsCard({ assignment }: { assignment: AssignmentDetail }) {
  const qc = useQueryClient()
  const groups = useQuery({ queryKey: ['assignment-groups', assignment.id], queryFn: () => assignmentsApi.groups(assignment.id) })
  const enrollments = useQuery({ queryKey: ['enrollments', assignment.course_id], queryFn: () => coursesApi.enrollments(assignment.course_id) })
  const [dialog, setDialog] = React.useState<{ open: boolean; group: GroupInfo | null }>({ open: false, group: null })
  const [deleting, setDeleting] = React.useState<GroupInfo | null>(null)
  const del = useMutation({
    mutationFn: (g: GroupInfo) => assignmentsApi.deleteGroup(assignment.id, g.id),
    onSuccess: () => {
      qc.invalidateQueries({ queryKey: ['assignment-groups', assignment.id] })
      qc.invalidateQueries({ queryKey: ['assignment-submissions', assignment.id] })
      toast.success('Group deleted')
      setDeleting(null)
    },
    onError: (e) => toast.error(errorMessage(e)),
  })

  const list = groups.data ?? []
  const students = (enrollments.data ?? []).filter((s) => s.is_active)
  const grouped = new Set(list.flatMap((g) => g.members.map((m) => m.student_id)))
  const ungrouped = students.filter((s) => !grouped.has(s.user_id))

  return (
    <Card>
      <CardHeader className="flex-row items-start justify-between gap-3">
        <div className="space-y-1">
          <CardTitle className="flex items-center gap-2">
            <Users className="size-4 text-muted-foreground" /> Groups
          </CardTitle>
          <CardDescription>
            {ungrouped.length ? `Not in a group yet: ${ungrouped.map((s) => s.name).join(', ')}` : 'Every enrolled student is in a group.'}
          </CardDescription>
        </div>
        <Button variant="outline" size="sm" onClick={() => setDialog({ open: true, group: null })} disabled={!ungrouped.length}>
          <Plus /> New group
        </Button>
      </CardHeader>
      <CardContent className="space-y-2">
        {groups.isLoading ? (
          <p className="text-sm text-muted-foreground">Loading…</p>
        ) : list.length === 0 ? (
          <p className="text-sm text-muted-foreground">No groups yet. Students cannot submit until they are in a group.</p>
        ) : (
          list.map((g) => (
            <div key={g.id} className="flex items-start justify-between gap-3 rounded-lg border px-3 py-2">
              <div className="min-w-0">
                <div className="text-sm font-medium">{g.name}</div>
                <div className="mt-1 flex flex-wrap gap-1">
                  {g.members.map((m) =>
                    m.student_id === g.leader_id ? (
                      <Badge key={m.student_id} variant="info">
                        <Crown /> {m.name}
                      </Badge>
                    ) : (
                      <Badge key={m.student_id} variant="outline">
                        {m.name}
                      </Badge>
                    ),
                  )}
                </div>
              </div>
              <div className="flex shrink-0 gap-1">
                <Button variant="ghost" size="icon-sm" onClick={() => setDialog({ open: true, group: g })} aria-label={`Edit ${g.name}`}>
                  <Pencil />
                </Button>
                <Button variant="ghost" size="icon-sm" onClick={() => setDeleting(g)} aria-label={`Delete ${g.name}`}>
                  <Trash2 />
                </Button>
              </div>
            </div>
          ))
        )}
      </CardContent>
      <GroupDialog
        assignment={assignment}
        group={dialog.group}
        groups={list}
        students={students}
        open={dialog.open}
        onOpenChange={(o) => setDialog((d) => ({ ...d, open: o }))}
      />
      <ConfirmDialog
        open={!!deleting}
        onOpenChange={(o) => !o && setDeleting(null)}
        title={`Delete “${deleting?.name}”?`}
        description="The leader keeps the submission and its grade; the other members lose their copy."
        destructive
        confirmLabel="Delete group"
        loading={del.isPending}
        onConfirm={() => deleting && del.mutate(deleting)}
      />
    </Card>
  )
}
