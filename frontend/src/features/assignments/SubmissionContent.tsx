import { ExternalLink, GitBranch } from 'lucide-react'
import { AttachmentList } from '@/components/files'
import { Markdown } from '@/components/markdown'
import { LAB_REPORT_SECTIONS } from '@/lib/labels'
import type { FileMeta, LabReport } from '@/types/api'

/** Read-only rendering of a submission attempt (text, lab report, repository link, files). */
export function SubmissionContent({ body, labReport, repositoryUrl, attachments }: { body: string; labReport: LabReport | null; repositoryUrl: string | null; attachments: FileMeta[] }) {
  const sections = labReport ? LAB_REPORT_SECTIONS.filter((s) => labReport[s.key]?.trim()) : []
  const empty = !body.trim() && !sections.length && !repositoryUrl && !attachments.length
  if (empty) return <p className="text-sm text-muted-foreground">No content.</p>
  return (
    <div className="space-y-6">
      {sections.length > 0 && (
        <div className="space-y-5">
          {sections.map((s) => (
            <section key={s.key}>
              <h4 className="mb-1.5 text-xs font-semibold tracking-wide text-muted-foreground uppercase">{s.label}</h4>
              <Markdown>{labReport![s.key]}</Markdown>
            </section>
          ))}
        </div>
      )}
      {body.trim() && <Markdown>{body}</Markdown>}
      {repositoryUrl && (
        <a href={repositoryUrl} target="_blank" rel="noopener noreferrer nofollow" className="flex items-center gap-3 rounded-lg border bg-muted/30 px-3 py-2.5 text-sm hover:bg-muted/60">
          <GitBranch className="size-4 text-muted-foreground" />
          <span className="min-w-0 flex-1 truncate font-mono text-[13px]">{repositoryUrl}</span>
          <ExternalLink className="size-4 text-muted-foreground" />
        </a>
      )}
      {attachments.length > 0 && (
        <div>
          <h4 className="mb-2 text-xs font-semibold tracking-wide text-muted-foreground uppercase">Attachments</h4>
          <AttachmentList files={attachments} />
        </div>
      )}
    </div>
  )
}
