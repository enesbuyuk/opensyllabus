import * as React from 'react'
import { Plus, X } from 'lucide-react'
import { Button } from './ui/button'
import { Input } from './ui/input'

/** Edits a short list of strings, e.g. learning objectives. */
export function ListEditor({ value, onChange, placeholder = 'Add an item', max = 20 }: { value: string[]; onChange: (v: string[]) => void; placeholder?: string; max?: number }) {
  const [draft, setDraft] = React.useState('')
  const add = () => {
    const v = draft.trim()
    if (!v || value.length >= max) return
    onChange([...value, v])
    setDraft('')
  }
  return (
    <div className="space-y-2">
      {value.length > 0 && (
        <ul className="space-y-1.5">
          {value.map((item, i) => (
            <li key={i} className="group flex items-start gap-2 rounded-md border bg-muted/30 px-3 py-1.5 text-sm">
              <span className="mt-0.5 text-xs text-muted-foreground tabular-nums">{i + 1}.</span>
              <span className="flex-1">{item}</span>
              <button
                type="button"
                className="rounded p-0.5 text-muted-foreground opacity-60 hover:bg-accent hover:opacity-100 focus-visible:opacity-100"
                onClick={() => onChange(value.filter((_, j) => j !== i))}
                aria-label={`Remove ${item}`}
              >
                <X className="size-3.5" />
              </button>
            </li>
          ))}
        </ul>
      )}
      <div className="flex gap-2">
        <Input
          value={draft}
          placeholder={placeholder}
          onChange={(e) => setDraft(e.target.value)}
          onKeyDown={(e) => {
            if (e.key === 'Enter') {
              e.preventDefault()
              add()
            }
          }}
        />
        <Button type="button" variant="outline" size="icon" onClick={add} aria-label="Add item" disabled={!draft.trim()}>
          <Plus />
        </Button>
      </div>
    </div>
  )
}
