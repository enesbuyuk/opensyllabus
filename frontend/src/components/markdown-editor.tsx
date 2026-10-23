import * as React from 'react'
import { Eye, PencilLine } from 'lucide-react'
import { Textarea } from './ui/input'
import { Markdown } from './markdown'
import { cn } from '@/lib/utils'

interface Props {
  value: string
  onChange: (v: string) => void
  placeholder?: string
  minRows?: number
  id?: string
  disabled?: boolean
  invalid?: boolean
}

/** Markdown textarea with a write/preview toggle. */
export function MarkdownEditor({ value, onChange, placeholder, minRows = 8, id, disabled, invalid }: Props) {
  const [mode, setMode] = React.useState<'write' | 'preview'>('write')
  return (
    <div className={cn('overflow-hidden rounded-md border border-input shadow-xs dark:bg-input/15', invalid && 'border-destructive')}>
      <div className="flex items-center justify-between border-b bg-muted/40 px-2 py-1">
        <div className="flex gap-1" role="tablist" aria-label="Editor mode">
          {(['write', 'preview'] as const).map((m) => (
            <button
              key={m}
              type="button"
              role="tab"
              aria-selected={mode === m}
              onClick={() => setMode(m)}
              className={cn(
                'inline-flex items-center gap-1.5 rounded-md px-2 py-1 text-xs font-medium text-muted-foreground transition hover:text-foreground focus-visible:ring-2 focus-visible:ring-ring/50 focus-visible:outline-none',
                mode === m && 'bg-background text-foreground shadow-xs dark:bg-input/50',
              )}
            >
              {m === 'write' ? <PencilLine className="size-3.5" /> : <Eye className="size-3.5" />}
              {m === 'write' ? 'Write' : 'Preview'}
            </button>
          ))}
        </div>
        <span className="hidden text-[11px] text-muted-foreground sm:inline">Markdown supported · code blocks with ```language</span>
      </div>
      {mode === 'write' ? (
        <Textarea
          id={id}
          value={value}
          onChange={(e) => onChange(e.target.value)}
          placeholder={placeholder}
          disabled={disabled}
          aria-invalid={invalid || undefined}
          className="rounded-none border-0 font-mono text-[13px] shadow-none focus-visible:ring-0 dark:bg-transparent"
          style={{ minHeight: `${minRows * 1.6 + 1}rem` }}
        />
      ) : (
        <div className="min-h-32 px-4 py-3" style={{ minHeight: `${minRows * 1.6 + 1}rem` }}>
          {value.trim() ? <Markdown>{value}</Markdown> : <p className="text-sm text-muted-foreground">Nothing to preview yet.</p>}
        </div>
      )}
    </div>
  )
}
