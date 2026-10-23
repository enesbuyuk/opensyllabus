import * as React from 'react'
import { BookText, Code2, Download, ExternalLink, Eye, FileText, FlaskConical, Image, Link2, PlayCircle, ScrollText } from 'lucide-react'
import { fileUrl } from '@/api/client'
import { RESOURCE_KIND_LABELS } from '@/lib/labels'
import { formatBytes } from '@/lib/utils'
import type { FileMeta, Resource, ResourceKind } from '@/types/api'
import { canPreview, FilePreviewDialog } from './files'
import { Markdown } from './markdown'
import { Badge } from './ui/badge'
import { Button } from './ui/button'
import { Dialog, DialogContent, DialogDescription, DialogHeader, DialogTitle } from './ui/dialog'

export const RESOURCE_ICONS: Record<ResourceKind, typeof FileText> = {
  pdf: FileText,
  image: Image,
  markdown: BookText,
  code: Code2,
  link: Link2,
  video: PlayCircle,
  datasheet: ScrollText,
  lab: FlaskConical,
}

export function asFileMeta(r: Resource): FileMeta | null {
  if (!r.file_id) return null
  return { id: r.file_id, original_name: r.file_name ?? r.title, content_type: r.file_type ?? 'application/octet-stream', size_bytes: r.file_size ?? 0, created_at: r.created_at }
}

/** Actions to open a resource: preview, download, external link or note viewer. */
export function ResourceActions({ r, onPreview, onRead }: { r: Resource; onPreview: (f: FileMeta) => void; onRead: (r: Resource) => void }) {
  const file = asFileMeta(r)
  return (
    <div className="flex shrink-0 items-center gap-1">
      {r.content_md && (
        <Button variant="ghost" size="sm" onClick={() => onRead(r)}>
          <BookText /> Read
        </Button>
      )}
      {file && canPreview(file.content_type) && (
        <Button variant="ghost" size="icon-sm" onClick={() => onPreview(file)} aria-label={`Preview ${r.title}`}>
          <Eye />
        </Button>
      )}
      {file && (
        <Button asChild variant="ghost" size="icon-sm">
          <a href={fileUrl(file.id, true)} aria-label={`Download ${r.title}`}>
            <Download />
          </a>
        </Button>
      )}
      {r.url && (
        <Button asChild variant="ghost" size="icon-sm">
          <a href={r.url} target="_blank" rel="noopener noreferrer" aria-label={`Open ${r.title}`}>
            <ExternalLink />
          </a>
        </Button>
      )}
    </div>
  )
}

export function ResourceViewers({ preview, setPreview, reading, setReading }: { preview: FileMeta | null; setPreview: (f: FileMeta | null) => void; reading: Resource | null; setReading: (r: Resource | null) => void }) {
  return (
    <>
      <FilePreviewDialog file={preview} onOpenChange={(o) => !o && setPreview(null)} />
      <Dialog open={!!reading} onOpenChange={(o) => !o && setReading(null)}>
        <DialogContent wide>
          <DialogHeader>
            <DialogTitle>{reading?.title}</DialogTitle>
            {reading?.description && <DialogDescription>{reading.description}</DialogDescription>}
          </DialogHeader>
          {reading && <Markdown>{reading.content_md}</Markdown>}
        </DialogContent>
      </Dialog>
    </>
  )
}

export function ResourceList({ resources, emptyText = 'No resources.' }: { resources: Resource[]; emptyText?: string }) {
  const [preview, setPreview] = React.useState<FileMeta | null>(null)
  const [reading, setReading] = React.useState<Resource | null>(null)
  if (!resources.length) return <p className="text-[13px] text-muted-foreground">{emptyText}</p>
  return (
    <>
      <ul className="divide-y rounded-lg border">
        {resources.map((r) => {
          const Icon = RESOURCE_ICONS[r.kind]
          return (
            <li key={r.id} className="flex items-center gap-3 px-3 py-2.5">
              <div className="flex size-8 shrink-0 items-center justify-center rounded-md border bg-muted/40">
                <Icon className="size-4 text-muted-foreground" />
              </div>
              <div className="min-w-0 flex-1">
                <div className="flex items-center gap-2">
                  <span className="truncate text-sm font-medium">{r.title}</span>
                  {r.visibility === 'instructor' && <Badge variant="warning">Instructor only</Badge>}
                </div>
                <div className="truncate text-xs text-muted-foreground">
                  {RESOURCE_KIND_LABELS[r.kind]}
                  {r.file_size ? ` · ${formatBytes(r.file_size)}` : ''}
                  {r.description ? ` · ${r.description}` : ''}
                </div>
              </div>
              <ResourceActions r={r} onPreview={setPreview} onRead={setReading} />
            </li>
          )
        })}
      </ul>
      <ResourceViewers preview={preview} setPreview={setPreview} reading={reading} setReading={setReading} />
    </>
  )
}
