import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query'
import { AlertTriangle, CheckCircle2, Crown, History, Save, Send, Users } from 'lucide-react'
import * as React from 'react'
import { toast } from 'sonner'
import { assignmentsApi } from '@/api/endpoints'
import { AttachmentList, FileDropzone } from '@/components/files'
import { Field } from '@/components/form'
import { MarkdownEditor } from '@/components/markdown-editor'
import { ErrorState } from '@/components/page'
import { SubmissionStatusBadge } from '@/components/status'
import { Badge } from '@/components/ui/badge'
import { Button } from '@/components/ui/button'
import { Card, CardContent, CardDescription, CardFooter, CardHeader, CardTitle } from '@/components/ui/card'
import { Dialog, DialogContent, DialogDescription, DialogFooter, DialogHeader, DialogTitle } from '@/components/ui/dialog'
import { Input } from '@/components/ui/input'
import { Skeleton } from '@/components/ui/misc'
import { useAuth } from '@/lib/auth'
import { errorMessage } from '@/lib/errors'
import { emptyLabReport, LAB_REPORT_SECTIONS } from '@/lib/labels'
import { formatDateTime, isPast } from '@/lib/utils'
import type { AssignmentDetail, GroupInfo, LabReport, SubmissionDetail } from '@/types/api'
import { FeedbackPanel } from './FeedbackPanel'
import { LabReportForm } from './LabReportForm'
import { SubmissionContent } from './SubmissionContent'

const ACCEPTS_TEXT = ['written', 'research', 'code', 'mixed']
const ACCEPTS_FILES = ['file_upload', 'code', 'lab_report', 'research', 'mixed']
const ACCEPTS_REPO = ['repository', 'code', 'mixed']

interface DraftState {
  body: string
  repo: string
  lab: LabReport
}

function fromDetail(d: SubmissionDetail): DraftState {
  return { body: d.body_md, repo: d.repository_url ?? '', lab: { ...emptyLabReport(), ...(d.lab_report ?? {}) } }
}

function missingRequirement(a: AssignmentDetail, d: DraftState, files: number): string | null {
  const hasBody = !!d.body.trim()
  const hasRepo = !!d.repo.trim()
  switch (a.kind) {
    case 'written':
    case 'research':
      return hasBody ? null : 'Write a response before submitting.'
    case 'file_upload':
      return files ? null : 'Attach at least one file before submitting.'
    case 'repository':
      return hasRepo ? null : 'Provide a repository URL before submitting.'
    case 'code':
    case 'mixed':
      return hasBody || hasRepo || files ? null : 'Add a response, link or attachment before submitting.'
    case 'lab_report': {
      const missing = LAB_REPORT_SECTIONS.find((s) => s.required && !d.lab[s.key].trim())
      return missing ? `The lab report section “${missing.label}” is required.` : null
    }
  }
}

function GroupBanner({ group, studentId }: { group: GroupInfo | null; studentId: string }) {
  if (!group) {
    return (
      <div className="flex items-start gap-2 rounded-xl border border-warning/30 bg-warning/10 px-4 py-3 text-[13px]">
        <Users className="mt-0.5 size-4 shrink-0 text-warning" />
        This is a group assignment and you have not been assigned to a group yet. Your instructor will add you to one.
      </div>
    )
  }
  const leader = group.members.find((m) => m.student_id === group.leader_id)
  const isLeader = group.leader_id === studentId
  return (
    <div className="space-y-2 rounded-xl border bg-card px-4 py-3">
      <div className="flex items-center gap-2 text-sm font-medium">
        <Users className="size-4 text-muted-foreground" /> {group.name}
      </div>
      <p className="text-[13px] text-muted-foreground">
        {isLeader
          ? 'You are the group leader: what you submit here counts for the whole group, and everyone receives the same grade.'
          : `${leader?.name ?? 'Your group leader'} submits for the group. The submission and grade appear here for you too.`}
      </p>
      <div className="flex flex-wrap gap-1">
        {group.members.map((m) =>
          m.student_id === group.leader_id ? (
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
  )
}

export function SubmissionWorkspace({ assignment }: { assignment: AssignmentDetail }) {
  const { timezone } = useAuth()
  const qc = useQueryClient()
  const key = ['my-submission', assignment.id]
  const q = useQuery({ queryKey: key, queryFn: () => assignmentsApi.mySubmission(assignment.id) })
  const [draft, setDraft] = React.useState<DraftState | null>(null)
  const [dirty, setDirty] = React.useState(false)
  const [reviewOpen, setReviewOpen] = React.useState(false)
  const [uploading, setUploading] = React.useState(false)
  const [removing, setRemoving] = React.useState<string | null>(null)

  React.useEffect(() => {
    if (q.data && !dirty) setDraft(fromDetail(q.data))
  }, [q.data, dirty])

  // Warn before leaving with unsaved changes.
  React.useEffect(() => {
    if (!dirty) return
    const handler = (e: BeforeUnloadEvent) => e.preventDefault()
    window.addEventListener('beforeunload', handler)
    return () => window.removeEventListener('beforeunload', handler)
  }, [dirty])

  const refresh = () => {
    qc.invalidateQueries({ queryKey: key })
    qc.invalidateQueries({ queryKey: ['assignment', assignment.id] })
    qc.invalidateQueries({ queryKey: ['assignments'] })
    qc.invalidateQueries({ queryKey: ['dashboard'] })
  }

  const update = (patch: Partial<DraftState>) => {
    setDraft((d) => (d ? { ...d, ...patch } : d))
    setDirty(true)
  }

  const payload = () => ({
    body_md: draft!.body,
    repository_url: draft!.repo.trim(),
    ...(assignment.kind === 'lab_report' ? { lab_report: draft!.lab } : {}),
  })

  const save = useMutation({
    mutationFn: () => assignmentsApi.saveDraft(assignment.id, payload()),
    onSuccess: () => {
      setDirty(false)
      toast.success('Draft saved')
      refresh()
    },
    onError: (e) => toast.error(errorMessage(e)),
  })

  const submit = useMutation({
    mutationFn: async () => {
      if (dirty) await assignmentsApi.saveDraft(assignment.id, payload())
      return assignmentsApi.submit(assignment.id)
    },
    onSuccess: () => {
      setDirty(false)
      setReviewOpen(false)
      toast.success('Submission received')
      refresh()
    },
    onError: (e) => toast.error(errorMessage(e)),
  })

  const upload = async (files: File[]) => {
    setUploading(true)
    try {
      // Persist text changes first so the draft row exists and nothing is lost.
      if (dirty) {
        await assignmentsApi.saveDraft(assignment.id, payload())
        setDirty(false)
      }
      for (const f of files) {
        try {
          await assignmentsApi.uploadAttachment(assignment.id, f)
          toast.success(`Uploaded ${f.name}`)
        } catch (e) {
          toast.error(`${f.name}: ${errorMessage(e)}`)
        }
      }
    } finally {
      setUploading(false)
      refresh()
    }
  }

  const remove = async (fileId: string) => {
    setRemoving(fileId)
    try {
      await assignmentsApi.deleteAttachment(assignment.id, fileId)
      refresh()
    } catch (e) {
      toast.error(errorMessage(e))
    } finally {
      setRemoving(null)
    }
  }

  if (q.isLoading || (q.data && !draft)) return <Skeleton className="h-80 rounded-xl" />
  if (q.error || !q.data || !draft) return <ErrorState error={q.error} onRetry={() => q.refetch()} />
  const d = q.data
  const editable = d.can_edit
  const accept = assignment.allowed_file_types.length ? assignment.allowed_file_types.map((e) => `.${e}`).join(',') : undefined
  const blocker = missingRequirement(assignment, draft, d.attachments.length)
  const willBeLate = !!assignment.due_at && isPast(assignment.due_at)
  const latest = d.revisions[0]
  const groupMember = assignment.is_group && d.group?.leader_id !== d.student.id

  return (
    <div className="space-y-6">
      {assignment.is_group && <GroupBanner group={d.group} studentId={d.student.id} />}
      <FeedbackPanel detail={d} />

      {editable ? (
        <Card>
          <CardHeader className="flex-row items-start justify-between gap-3">
            <div className="space-y-1">
              <CardTitle>{d.status === 'revision_requested' ? 'Revise your submission' : 'Your work'}</CardTitle>
              <CardDescription>
                {d.id ? `Last saved ${formatDateTime(d.updated_at, timezone)}` : 'Start writing. Save a draft anytime and submit when ready.'}
              </CardDescription>
            </div>
            <SubmissionStatusBadge status={d.status} late={d.is_late} />
          </CardHeader>
          <CardContent className="space-y-6">
            {d.status === 'submitted' && (
              <div className="rounded-md border border-info/30 bg-info/10 px-3 py-2 text-[13px]">
                Resubmission is allowed until the deadline. Editing withdraws your submission to a draft until you submit again.
              </div>
            )}
            {assignment.kind === 'lab_report' && <LabReportForm value={draft.lab} onChange={(lab) => update({ lab })} />}
            {ACCEPTS_TEXT.includes(assignment.kind) && (
              <Field label={assignment.kind === 'code' ? 'Code and explanation' : 'Response'} htmlFor="response">
                <MarkdownEditor
                  id="response"
                  value={draft.body}
                  onChange={(body) => update({ body })}
                  minRows={12}
                  placeholder={assignment.kind === 'code' ? 'Paste code in fenced blocks, e.g. ```python … ```' : 'Write your answer in Markdown…'}
                />
              </Field>
            )}
            {ACCEPTS_REPO.includes(assignment.kind) && (
              <Field label="Repository URL" htmlFor="repo" hint="Link to a GitHub/GitLab repository or another https URL.">
                <Input id="repo" type="url" inputMode="url" placeholder="https://github.com/username/project" value={draft.repo} onChange={(e) => update({ repo: e.target.value })} />
              </Field>
            )}
            {ACCEPTS_FILES.includes(assignment.kind) && (
              <Field label="Attachments" hint={undefined}>
                <div className="space-y-3">
                  <AttachmentList files={d.attachments} onRemove={(f) => remove(f.id)} removing={removing} />
                  <FileDropzone
                    onFiles={upload}
                    accept={accept}
                    uploading={uploading}
                    hint={`${assignment.allowed_file_types.length ? `Accepted: ${assignment.allowed_file_types.map((e) => `.${e}`).join(', ')}` : 'PDF, images, CSV, code files, ZIP and more'} · up to ${assignment.max_upload_mb} MB each`}
                  />
                </div>
              </Field>
            )}
          </CardContent>
          <CardFooter className="flex-col items-stretch gap-3 sm:flex-row sm:items-center sm:justify-between">
            <span className="text-xs text-muted-foreground">{dirty ? 'You have unsaved changes.' : 'All changes saved.'}</span>
            <div className="flex gap-2">
              <Button variant="outline" onClick={() => save.mutate()} loading={save.isPending} disabled={!dirty}>
                <Save /> Save draft
              </Button>
              <Button onClick={() => setReviewOpen(true)} disabled={submit.isPending}>
                <Send /> Review & submit
              </Button>
            </div>
          </CardFooter>
        </Card>
      ) : (
        <Card>
          <CardHeader className="flex-row items-start justify-between gap-3">
            <div className="space-y-1">
              <CardTitle>Your submission</CardTitle>
              <CardDescription>
                {d.submitted_at
                  ? `Attempt ${d.attempt} · submitted ${formatDateTime(d.submitted_at, timezone)}`
                  : groupMember
                    ? 'Only the group leader can submit this assignment.'
                    : 'This assignment is no longer accepting submissions.'}
              </CardDescription>
            </div>
            <SubmissionStatusBadge status={d.status} late={d.is_late} />
          </CardHeader>
          <CardContent>
            {latest ? (
              <SubmissionContent body={latest.body_md} labReport={latest.lab_report} repositoryUrl={latest.repository_url} attachments={latest.attachments} />
            ) : (
              <p className="text-sm text-muted-foreground">{groupMember ? 'Your group has not submitted yet.' : 'You did not submit this assignment.'}</p>
            )}
          </CardContent>
        </Card>
      )}

      {d.revisions.length > (editable ? 0 : 1) && (
        <Card>
          <CardHeader>
            <CardTitle className="flex items-center gap-2">
              <History className="size-4 text-muted-foreground" /> Submission history
            </CardTitle>
          </CardHeader>
          <CardContent className="space-y-2">
            {d.revisions.map((r) => (
              <details key={r.id} className="group rounded-lg border">
                <summary className="flex cursor-pointer items-center justify-between gap-3 px-4 py-2.5 text-sm select-none">
                  <span className="font-medium">Attempt {r.attempt}</span>
                  <span className="text-xs text-muted-foreground">
                    {formatDateTime(r.submitted_at, timezone)}
                    {r.is_late && <span className="ml-2 text-warning">Late</span>}
                  </span>
                </summary>
                <div className="border-t p-4">
                  <SubmissionContent body={r.body_md} labReport={r.lab_report} repositoryUrl={r.repository_url} attachments={r.attachments} />
                </div>
              </details>
            ))}
          </CardContent>
        </Card>
      )}

      <Dialog open={reviewOpen} onOpenChange={setReviewOpen}>
        <DialogContent wide>
          <DialogHeader>
            <DialogTitle>Review your submission</DialogTitle>
            <DialogDescription>Check everything below. After submitting, your work is locked{assignment.allow_resubmission ? ' unless you resubmit before the deadline' : ''}.</DialogDescription>
          </DialogHeader>
          <div className="max-h-[50vh] overflow-y-auto rounded-lg border p-4">
            <SubmissionContent
              body={ACCEPTS_TEXT.includes(assignment.kind) ? draft.body : ''}
              labReport={assignment.kind === 'lab_report' ? draft.lab : null}
              repositoryUrl={draft.repo.trim() || null}
              attachments={d.attachments}
            />
          </div>
          {blocker && (
            <div className="flex items-start gap-2 rounded-md border border-destructive/30 bg-destructive/10 px-3 py-2 text-[13px] text-destructive">
              <AlertTriangle className="mt-0.5 size-4 shrink-0" /> {blocker}
            </div>
          )}
          {!blocker && willBeLate && (
            <div className="flex items-start gap-2 rounded-md border border-warning/30 bg-warning/10 px-3 py-2 text-[13px]">
              <AlertTriangle className="mt-0.5 size-4 shrink-0 text-warning" />
              The deadline has passed. This submission will be marked late
              {assignment.late_policy === 'penalty' ? ` and a ${assignment.late_penalty_percent}% penalty applies` : ''}.
            </div>
          )}
          <DialogFooter>
            <Button variant="outline" onClick={() => setReviewOpen(false)}>
              Keep editing
            </Button>
            <Button onClick={() => submit.mutate()} loading={submit.isPending} disabled={!!blocker}>
              <CheckCircle2 /> Submit {d.attempt > 0 ? `attempt ${d.attempt + 1}` : 'assignment'}
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
    </div>
  )
}
