import { Field } from '@/components/form'
import { MarkdownEditor } from '@/components/markdown-editor'
import { Input } from '@/components/ui/input'
import { LAB_REPORT_SECTIONS } from '@/lib/labels'
import type { LabReport } from '@/types/api'

/** Structured laboratory report template. Every section supports Markdown. */
export function LabReportForm({ value, onChange, disabled }: { value: LabReport; onChange: (v: LabReport) => void; disabled?: boolean }) {
  return (
    <div className="space-y-5">
      {LAB_REPORT_SECTIONS.map((s) => (
        <Field key={s.key} label={s.label} htmlFor={`lab-${s.key}`} hint={s.hint} required={s.required}>
          {s.rows === 1 ? (
            <Input id={`lab-${s.key}`} value={value[s.key]} onChange={(e) => onChange({ ...value, [s.key]: e.target.value })} disabled={disabled} />
          ) : (
            <MarkdownEditor id={`lab-${s.key}`} value={value[s.key]} onChange={(v) => onChange({ ...value, [s.key]: v })} minRows={s.rows} disabled={disabled} />
          )}
        </Field>
      ))}
    </div>
  )
}
