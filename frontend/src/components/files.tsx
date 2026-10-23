import * as React from 'react'
import { Download, Eye, File as FileIcon, FileCode, FileImage, FileSpreadsheet, FileText, Loader2, Trash2, UploadCloud } from 'lucide-react'
import { fileUrl } from '@/api/client'
import { Button } from './ui/button'
import { Dialog, DialogContent, DialogHeader, DialogTitle } from './ui/dialog'
import { cn, formatBytes } from '@/lib/utils'
import type { FileMeta } from '@/types/api'

export function fileIcon(contentType: string | null | undefined) {
  const t = contentType ?? ''
  if (t.startsWith('image/')) return FileImage
  if (t === 'application/pdf' || t === 'text/markdown' || t === 'text/plain') return FileText
  if (t.includes('csv') || t.includes('spreadsheet') || t.includes('tab-separated')) return FileSpreadsheet
  if (t.startsWith('text/') || t.includes('json')) return FileCode
  return FileIcon
}

export function canPreview(contentType: string) {
  return contentType.startsWith('image/') || contentType === 'application/pdf' || contentType.startsWith('text/') || contentType.includes('json')
}

/** Inline preview for images, PDFs and text files (served by the authorization-checked API). */
export function FilePreviewDialog({ file, onOpenChange }: { file: FileMeta | null; onOpenChange: (o: boolean) => void }) {
  const [text, setText] = React.useState<string | null>(null)
  const isText = !!file && !file.content_type.startsWith('image/') && file.content_type !== 'application/pdf'
  React.useEffect(() => {
    if (!file || !isText) return
    let cancelled = false
    fetch(fileUrl(file.id), { credentials: 'same-origin' })
      .then((r) => r.text())
      .then((t) => !cancelled && setText(t.length > 200_000 ? `${t.slice(0, 200_000)}\n… (truncated)` : t))
      .catch(() => !cancelled && setText('Unable to load preview.'))
    return () => {
      cancelled = true
      setText(null)
    }
  }, [file, isText])
  return (
    <Dialog open={!!file} onOpenChange={onOpenChange}>
      <DialogContent wide className="max-w-5xl">
        <DialogHeader>
          <DialogTitle className="truncate">{file?.original_name}</DialogTitle>
        </DialogHeader>
        {file && (
          <div className="min-h-64">
            {file.content_type.startsWith('image/') && <img src={fileUrl(file.id)} alt={file.original_name} className="mx-auto max-h-[70vh] rounded-lg border" />}
            {file.content_type === 'application/pdf' && <iframe src={fileUrl(file.id)} title={file.original_name} className="h-[70vh] w-full rounded-lg border bg-white" />}
            {isText && (
              <pre className="max-h-[70vh] overflow-auto rounded-lg border bg-muted/40 p-4 font-mono text-xs leading-5 whitespace-pre">{text ?? 'Loading…'}</pre>
            )}
          </div>
        )}
        {file && (
          <div className="flex justify-end">
            <Button asChild variant="outline" size="sm">
              <a href={fileUrl(file.id, true)}>
                <Download /> Download
              </a>
            </Button>
          </div>
        )}
      </DialogContent>
    </Dialog>
  )
}

export function AttachmentList({ files, onRemove, removing, emptyText }: { files: FileMeta[]; onRemove?: (f: FileMeta) => void; removing?: string | null; emptyText?: string }) {
  const [preview, setPreview] = React.useState<FileMeta | null>(null)
  if (!files.length) return emptyText ? <p className="text-[13px] text-muted-foreground">{emptyText}</p> : null
  return (
    <>
      <ul className="divide-y rounded-lg border">
        {files.map((f) => {
          const Icon = fileIcon(f.content_type)
          return (
            <li key={f.id} className="flex items-center gap-3 px-3 py-2">
              <Icon className="size-4 shrink-0 text-muted-foreground" />
              <div className="min-w-0 flex-1">
                <div className="truncate text-sm">{f.original_name}</div>
                <div className="text-xs text-muted-foreground">{formatBytes(f.size_bytes)}</div>
              </div>
              {canPreview(f.content_type) && (
                <Button variant="ghost" size="icon-sm" onClick={() => setPreview(f)} aria-label={`Preview ${f.original_name}`}>
                  <Eye />
                </Button>
              )}
              <Button asChild variant="ghost" size="icon-sm">
                <a href={fileUrl(f.id, true)} aria-label={`Download ${f.original_name}`}>
                  <Download />
                </a>
              </Button>
              {onRemove && (
                <Button variant="ghost" size="icon-sm" onClick={() => onRemove(f)} disabled={removing === f.id} aria-label={`Remove ${f.original_name}`}>
                  {removing === f.id ? <Loader2 className="animate-spin" /> : <Trash2 />}
                </Button>
              )}
            </li>
          )
        })}
      </ul>
      <FilePreviewDialog file={preview} onOpenChange={(o) => !o && setPreview(null)} />
    </>
  )
}

/** Drag-and-drop file picker that uploads files one by one. */
export function FileDropzone({ onFiles, accept, hint, disabled, uploading }: { onFiles: (files: File[]) => void; accept?: string; hint?: string; disabled?: boolean; uploading?: boolean }) {
  const inputRef = React.useRef<HTMLInputElement>(null)
  const [over, setOver] = React.useState(false)
  return (
    <div
      onDragOver={(e) => {
        e.preventDefault()
        if (!disabled) setOver(true)
      }}
      onDragLeave={() => setOver(false)}
      onDrop={(e) => {
        e.preventDefault()
        setOver(false)
        if (!disabled && e.dataTransfer.files.length) onFiles(Array.from(e.dataTransfer.files))
      }}
      className={cn(
        'flex flex-col items-center justify-center gap-2 rounded-lg border border-dashed px-4 py-6 text-center transition',
        over ? 'border-primary bg-primary/5' : 'bg-muted/20',
        disabled && 'opacity-50',
      )}
    >
      {uploading ? <Loader2 className="size-5 animate-spin text-primary" /> : <UploadCloud className="size-5 text-muted-foreground" />}
      <div className="text-sm">
        <button type="button" className="font-medium text-primary hover:underline disabled:no-underline" onClick={() => inputRef.current?.click()} disabled={disabled || uploading}>
          Choose files
        </button>{' '}
        <span className="text-muted-foreground">or drag and drop</span>
      </div>
      {hint && <p className="text-xs text-muted-foreground">{hint}</p>}
      <input
        ref={inputRef}
        type="file"
        multiple
        accept={accept}
        className="sr-only"
        tabIndex={-1}
        onChange={(e) => {
          if (e.target.files?.length) onFiles(Array.from(e.target.files))
          e.target.value = ''
        }}
      />
    </div>
  )
}
