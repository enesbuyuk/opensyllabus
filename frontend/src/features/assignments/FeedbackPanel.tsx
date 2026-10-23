import { MessageSquareText, RotateCcw, Star } from 'lucide-react'
import { Markdown } from '@/components/markdown'
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card'
import { Progress } from '@/components/ui/misc'
import { useAuth } from '@/lib/auth'
import { formatDateTime, formatScore } from '@/lib/utils'
import type { SubmissionDetail } from '@/types/api'

/** Released grade, rubric breakdown and instructor feedback. */
export function FeedbackPanel({ detail }: { detail: SubmissionDetail }) {
  const { timezone } = useAuth()
  const { grade, feedback, rubric } = detail
  if (!grade && !feedback.length) return null
  return (
    <Card className="border-primary/25">
      <CardHeader>
        <CardTitle className="flex items-center gap-2">
          <MessageSquareText className="size-4 text-primary" /> Instructor feedback
        </CardTitle>
      </CardHeader>
      <CardContent className="space-y-5">
        {grade && (
          <div className="rounded-lg border bg-muted/20 p-4">
            <div className="flex items-center justify-between">
              <span className="flex items-center gap-2 text-sm font-medium">
                <Star className="size-4 text-warning" /> Grade
              </span>
              <span className="text-xl font-semibold tabular-nums">{formatScore(grade.score, grade.max_score)}</span>
            </div>
            <Progress value={(grade.score / grade.max_score) * 100} className="mt-3" />
            {grade.late_penalty > 0 && <p className="mt-2 text-xs text-warning">Includes a late penalty of {grade.late_penalty} points.</p>}
            {rubric.length > 0 && Object.keys(grade.rubric_scores).length > 0 && (
              <ul className="mt-4 space-y-2 border-t pt-3">
                {rubric.map((c) => (
                  <li key={c.id} className="flex items-center justify-between gap-4 text-sm">
                    <span className="text-muted-foreground">{c.title}</span>
                    <span className="tabular-nums">{formatScore(grade.rubric_scores[c.id] ?? null, c.max_points)}</span>
                  </li>
                ))}
              </ul>
            )}
          </div>
        )}
        {feedback.map((f) => (
          <div key={f.id} className="space-y-2">
            <div className="flex flex-wrap items-center gap-2 text-xs text-muted-foreground">
              {f.kind === 'revision_request' && (
                <span className="flex items-center gap-1 font-medium text-warning">
                  <RotateCcw className="size-3.5" /> Revision requested
                </span>
              )}
              <span>{f.author_name ?? 'Instructor'}</span>
              <span>·</span>
              <span>Attempt {f.attempt}</span>
              <span>·</span>
              <span>{formatDateTime(f.released_at ?? f.created_at, timezone)}</span>
              {!f.released_at && <span className="text-warning">(not yet returned)</span>}
            </div>
            <Markdown className="rounded-lg border bg-card p-4">{f.body_md}</Markdown>
          </div>
        ))}
      </CardContent>
    </Card>
  )
}
