import { useMutation, useQueryClient } from '@tanstack/react-query'
import { RotateCcw, Save, Send } from 'lucide-react'
import * as React from 'react'
import { toast } from 'sonner'
import { submissionsApi } from '@/api/endpoints'
import { Field } from '@/components/form'
import { MarkdownEditor } from '@/components/markdown-editor'
import { Button } from '@/components/ui/button'
import { Input } from '@/components/ui/input'
import { errorMessage } from '@/lib/errors'
import type { SubmissionDetail } from '@/types/api'

/** Grading form: rubric or single score, feedback, and the three review actions. */
export function ReviewPanel({ detail }: { detail: SubmissionDetail }) {
  const qc = useQueryClient()
  const { rubric, grade, assignment } = detail
  const draftFeedback = [...detail.feedback].reverse().find((f) => f.attempt === detail.attempt && !f.released_at)
  const [scores, setScores] = React.useState<Record<string, string>>({})
  const [score, setScore] = React.useState('')
  const [feedback, setFeedback] = React.useState('')

  React.useEffect(() => {
    setScores(Object.fromEntries(rubric.map((c) => [c.id, grade?.rubric_scores[c.id] != null ? String(grade.rubric_scores[c.id]) : ''])))
    setScore(grade ? String(grade.score + grade.late_penalty) : '')
    setFeedback(draftFeedback?.body_md ?? '')
    // Reset when switching submissions or after the server state changes.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [detail.id, detail.updated_at])

  const rubricTotal = rubric.reduce((s, c) => s + (Number(scores[c.id]) || 0), 0)
  const rubricComplete = rubric.length > 0 && rubric.every((c) => scores[c.id] !== '' && scores[c.id] != null)
  const effectiveScore = rubric.length ? (rubricComplete ? rubricTotal : null) : score === '' ? null : Number(score)
  const latePenalty = detail.is_late && assignment.late_policy === 'penalty' && effectiveScore != null ? Math.round(effectiveScore * assignment.late_penalty_percent) / 100 : 0

  const review = useMutation({
    mutationFn: (action: 'save' | 'return' | 'request_revision') =>
      submissionsApi.review(detail.id, {
        action,
        score: rubric.length ? undefined : effectiveScore,
        rubric_scores: rubric.length ? Object.fromEntries(Object.entries(scores).filter(([, v]) => v !== '').map(([k, v]) => [k, Number(v)])) : {},
        feedback,
      }),
    onSuccess: (d, action) => {
      qc.setQueryData(['submission', detail.id], d)
      qc.invalidateQueries({ queryKey: ['submissions'] })
      qc.invalidateQueries({ queryKey: ['dashboard'] })
      qc.invalidateQueries({ queryKey: ['gradebook'] })
      toast.success(action === 'return' ? 'Returned to student' : action === 'request_revision' ? 'Revision requested' : 'Review saved (not visible to the student yet)')
    },
    onError: (e) => toast.error(errorMessage(e)),
  })

  const graded = detail.status === 'graded'
  return (
    <div className="space-y-4">
      {rubric.length > 0 ? (
        <div className="space-y-2">
          <div className="text-[13px] font-medium">Rubric</div>
          {rubric.map((c) => (
            <div key={c.id} className="flex items-center gap-3 rounded-lg border px-3 py-2">
              <div className="min-w-0 flex-1">
                <div className="text-sm">{c.title}</div>
                {c.description && <div className="text-xs text-muted-foreground">{c.description}</div>}
              </div>
              <Input
                type="number"
                inputMode="decimal"
                min={0}
                max={c.max_points}
                step="0.5"
                value={scores[c.id] ?? ''}
                onChange={(e) => setScores((s) => ({ ...s, [c.id]: e.target.value }))}
                className="w-20 text-right"
                aria-label={`${c.title} score`}
              />
              <span className="w-12 text-xs text-muted-foreground tabular-nums">/ {c.max_points}</span>
            </div>
          ))}
          <div className="flex justify-between px-1 text-sm">
            <span className="text-muted-foreground">Total</span>
            <span className="font-medium tabular-nums">
              {rubricTotal} / {assignment.max_grade}
            </span>
          </div>
        </div>
      ) : (
        <Field label={`Grade (out of ${assignment.max_grade})`} htmlFor="rv-score">
          <Input id="rv-score" type="number" inputMode="decimal" min={0} max={assignment.max_grade} step="0.5" value={score} onChange={(e) => setScore(e.target.value)} className="w-32" />
        </Field>
      )}
      {latePenalty > 0 && (
        <p className="rounded-md border border-warning/30 bg-warning/10 px-3 py-2 text-xs">
          Late submission: a {assignment.late_penalty_percent}% penalty (−{latePenalty}) is applied automatically. Final grade: {Math.max(0, (effectiveScore ?? 0) - latePenalty)}.
        </p>
      )}
      <Field label="Feedback" hint="Markdown supported. Visible to the student when you return the review or request a revision.">
        <MarkdownEditor value={feedback} onChange={setFeedback} minRows={8} placeholder="What went well, what to improve…" />
      </Field>
      <div className="flex flex-wrap justify-end gap-2">
        {!graded && (
          <Button variant="ghost" onClick={() => review.mutate('save')} loading={review.isPending && review.variables === 'save'} disabled={review.isPending}>
            <Save /> Save draft
          </Button>
        )}
        <Button variant="outline" onClick={() => review.mutate('request_revision')} disabled={review.isPending || !feedback.trim()} loading={review.isPending && review.variables === 'request_revision'}>
          <RotateCcw /> Request revision
        </Button>
        <Button onClick={() => review.mutate('return')} disabled={review.isPending || effectiveScore == null} loading={review.isPending && review.variables === 'return'}>
          <Send /> {graded ? 'Update returned grade' : 'Return to student'}
        </Button>
      </div>
    </div>
  )
}
