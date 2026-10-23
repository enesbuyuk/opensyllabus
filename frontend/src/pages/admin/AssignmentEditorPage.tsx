import { zodResolver } from '@hookform/resolvers/zod'
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query'
import { Archive, Plus, Send, Trash2, X } from 'lucide-react'
import * as React from 'react'
import { Controller, useFieldArray, useForm } from 'react-hook-form'
import { useNavigate, useParams, useSearchParams } from 'react-router'
import { toast } from 'sonner'
import { z } from 'zod'
import { assignmentsApi, coursesApi } from '@/api/endpoints'
import { Field } from '@/components/form'
import { MarkdownEditor } from '@/components/markdown-editor'
import { ErrorState, PageHeader, PageSkeleton } from '@/components/page'
import { PublishBadge } from '@/components/status'
import { ConfirmDialog } from '@/components/ui/alert-dialog'
import { Button } from '@/components/ui/button'
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card'
import { Input, NativeSelect, Textarea } from '@/components/ui/input'
import { Label } from '@/components/ui/label'
import { Switch } from '@/components/ui/misc'
import { useCourses, usePlatformSettings } from '@/hooks/queries'
import { useAuth } from '@/lib/auth'
import { useBreadcrumbs } from '@/lib/breadcrumbs'
import { errorMessage, handleFormError } from '@/lib/errors'
import { ASSIGNMENT_KIND_LABELS, LATE_POLICY_LABELS } from '@/lib/labels'
import { cn, isoToWallTime, wallTimeToIso } from '@/lib/utils'
import type { AssignmentKind } from '@/types/api'

const schema = z
  .object({
    course_id: z.string().min(1, 'Select a course'),
    week_id: z.string(),
    lesson_id: z.string(),
    title: z.string().trim().min(3, 'At least 3 characters').max(200),
    description: z.string().max(2000),
    instructions_md: z.string().max(100_000),
    reference_md: z.string().max(50_000),
    kind: z.enum(['written', 'file_upload', 'code', 'lab_report', 'research', 'repository', 'mixed']),
    max_grade: z.coerce.number<number>().positive('Must be greater than 0').max(1000),
    due_at: z.string(),
    publish_at: z.string(),
    late_policy: z.enum(['accept', 'penalty', 'reject']),
    late_penalty_percent: z.coerce.number<number>().int().min(0).max(100),
    allowed_file_types: z.array(z.string()),
    max_upload_mb: z.coerce.number<number>().int().min(1).max(100),
    allow_resubmission: z.boolean(),
    rubric: z.array(z.object({ id: z.string(), title: z.string().trim().min(1, 'Required'), description: z.string(), max_points: z.coerce.number<number>().positive('> 0') })),
  })
  .refine((v) => !v.publish_at || !v.due_at || v.publish_at < v.due_at, { message: 'Due date must be after the publication date', path: ['due_at'] })
  .refine((v) => !v.rubric.length || Math.abs(v.rubric.reduce((s, c) => s + c.max_points, 0) - v.max_grade) < 0.001, {
    message: 'Rubric points must add up to the maximum grade',
    path: ['rubric'],
  })
type FormValues = z.infer<typeof schema>

const FILE_GROUPS: { label: string; exts: string[] }[] = [
  { label: 'Documents', exts: ['pdf', 'docx', 'pptx', 'md', 'txt'] },
  { label: 'Images', exts: ['png', 'jpg', 'jpeg', 'gif', 'webp'] },
  { label: 'Data', exts: ['csv', 'tsv', 'xlsx', 'json'] },
  { label: 'Code', exts: ['py', 'ipynb', 'c', 'h', 'cpp', 'hpp', 'ino', 'rs', 'js', 'ts', 'm'] },
  { label: 'Hardware', exts: ['kicad_sch', 'kicad_pcb', 'kicad_pro', 'cir', 'net', 'v', 'sv', 'vhd'] },
  { label: 'Archives', exts: ['zip'] },
]

export function AssignmentEditorPage() {
  const { id } = useParams()
  const [params] = useSearchParams()
  const navigate = useNavigate()
  const qc = useQueryClient()
  const { timezone } = useAuth()
  const courses = useCourses()
  const settings = usePlatformSettings()
  const existing = useQuery({ queryKey: ['assignment', id], queryFn: () => assignmentsApi.get(id!), enabled: !!id })
  const [confirmDelete, setConfirmDelete] = React.useState(false)
  useBreadcrumbs([{ label: 'Assignments', to: '/assignments' }, { label: id ? 'Edit assignment' : 'New assignment' }])

  const form = useForm<FormValues>({ resolver: zodResolver(schema) })
  const rubric = useFieldArray({ control: form.control, name: 'rubric' })
  const courseId = form.watch('course_id')
  const weekId = form.watch('week_id')
  const kind = form.watch('kind')
  const latePolicy = form.watch('late_policy')
  const curriculum = useQuery({ queryKey: ['curriculum', courseId], queryFn: () => coursesApi.curriculum(courseId), enabled: !!courseId })
  const weekLessons = useQuery({ queryKey: ['course-lessons', courseId], queryFn: () => coursesApi.lessons(courseId), enabled: !!courseId })
  const weekNumber = curriculum.data?.weeks.find((w) => w.id === weekId)?.week_number

  const loaded = React.useRef(false)
  React.useEffect(() => {
    if (loaded.current) return
    if (id && !existing.data) return
    if (!id && !courses.data) return
    loaded.current = true
    const a = existing.data
    form.reset({
      course_id: a?.course_id ?? params.get('course') ?? courses.data?.[0]?.id ?? '',
      week_id: a?.week_id ?? params.get('week') ?? '',
      lesson_id: a?.lesson_id ?? params.get('lesson') ?? '',
      title: a?.title ?? '',
      description: a?.description ?? '',
      instructions_md: a?.instructions_md ?? '',
      reference_md: a?.reference_md ?? '',
      kind: a?.kind ?? 'written',
      max_grade: a?.max_grade ?? 100,
      due_at: isoToWallTime(a?.due_at, timezone),
      publish_at: isoToWallTime(a?.publish_at, timezone),
      late_policy: a?.late_policy ?? 'accept',
      late_penalty_percent: a?.late_penalty_percent ?? 10,
      allowed_file_types: a?.allowed_file_types ?? [],
      max_upload_mb: a?.max_upload_mb ?? 20,
      allow_resubmission: a?.allow_resubmission ?? false,
      rubric: a?.rubric.map((c) => ({ id: c.id, title: c.title, description: c.description, max_points: c.max_points })) ?? [],
    })
  }, [id, existing.data, courses.data, params, timezone, form])

  const save = (status?: 'draft' | 'published' | 'archived') =>
    form.handleSubmit(async (v) => {
      const body = {
        ...v,
        due_at: v.due_at ? wallTimeToIso(v.due_at, timezone) : '',
        publish_at: v.publish_at ? wallTimeToIso(v.publish_at, timezone) : '',
        status: status ?? existing.data?.status ?? 'draft',
      }
      try {
        let targetId = id
        if (id) await assignmentsApi.update(id, body)
        else targetId = (await assignmentsApi.create(body)).id
        qc.invalidateQueries({ queryKey: ['assignments'] })
        qc.invalidateQueries({ queryKey: ['assignment', targetId] })
        qc.invalidateQueries({ queryKey: ['week'] })
        toast.success(status === 'published' ? 'Assignment published' : status === 'archived' ? 'Assignment archived' : 'Assignment saved')
        navigate(`/assignments/${targetId}`)
      } catch (e) {
        handleFormError(e, form.setError)
      }
    })()

  const del = useMutation({
    mutationFn: () => assignmentsApi.remove(id!),
    onSuccess: () => {
      qc.invalidateQueries({ queryKey: ['assignments'] })
      toast.success('Assignment deleted')
      navigate('/assignments')
    },
    onError: (e) => {
      setConfirmDelete(false)
      toast.error(errorMessage(e))
    },
  })

  if ((id && existing.isLoading) || courses.isLoading) return <PageSkeleton />
  if (existing.error) return <ErrorState error={existing.error} onRetry={() => existing.refetch()} />
  const { errors, isSubmitting } = form.formState
  const status = existing.data?.status ?? 'draft'
  const rubricTotal = (form.watch('rubric') ?? []).reduce((s, c) => s + (Number(c.max_points) || 0), 0)
  const acceptsFiles = !['written', 'repository'].includes(kind ?? '')
  const supported = new Set(settings.data?.supported_file_types ?? [])

  return (
    <form onSubmit={(e) => e.preventDefault()} className="space-y-6">
      <PageHeader
        title={id ? existing.data?.title : 'New assignment'}
        eyebrow={id ? 'Edit assignment' : 'Create assignment'}
        actions={
          <>
            {id && <PublishBadge status={status} />}
            <Button type="button" variant="outline" onClick={() => save()} loading={isSubmitting}>
              {status === 'draft' ? 'Save draft' : 'Save changes'}
            </Button>
            {status !== 'published' && (
              <Button type="button" onClick={() => save('published')} disabled={isSubmitting}>
                <Send /> Publish
              </Button>
            )}
          </>
        }
      />
      <div className="grid gap-6 xl:grid-cols-3">
        <div className="space-y-6 xl:col-span-2">
          <Card>
            <CardHeader>
              <CardTitle>Assignment</CardTitle>
            </CardHeader>
            <CardContent className="space-y-4">
              <Field label="Title" htmlFor="a-title" error={errors.title?.message} required>
                <Input id="a-title" {...form.register('title')} />
              </Field>
              <Field label="Short description" htmlFor="a-desc" hint="Shown in lists and notifications.">
                <Textarea id="a-desc" rows={2} {...form.register('description')} />
              </Field>
              <Field label="Instructions" error={errors.instructions_md?.message}>
                <Controller control={form.control} name="instructions_md" render={({ field }) => <MarkdownEditor value={field.value ?? ''} onChange={field.onChange} minRows={14} />} />
              </Field>
              <Field label="Reference materials" hint="Links, datasheets, formulas or reading list.">
                <Controller control={form.control} name="reference_md" render={({ field }) => <MarkdownEditor value={field.value ?? ''} onChange={field.onChange} minRows={5} />} />
              </Field>
            </CardContent>
          </Card>
          <Card>
            <CardHeader className="flex-row items-start justify-between gap-3">
              <div className="space-y-1">
                <CardTitle>Rubric</CardTitle>
                <CardDescription>
                  Optional. Criteria points must add up to the maximum grade ({rubricTotal} / {form.watch('max_grade') || 0}).
                </CardDescription>
              </div>
              <div className="flex gap-2">
                {rubric.fields.length > 0 && Math.abs(rubricTotal - Number(form.watch('max_grade'))) > 0.001 && (
                  <Button type="button" variant="ghost" size="sm" onClick={() => form.setValue('max_grade', rubricTotal, { shouldDirty: true })}>
                    Use {rubricTotal} as max grade
                  </Button>
                )}
                <Button type="button" variant="outline" size="sm" onClick={() => rubric.append({ id: '', title: '', description: '', max_points: 10 })}>
                  <Plus /> Criterion
                </Button>
              </div>
            </CardHeader>
            <CardContent className="space-y-2">
              {errors.rubric?.root?.message || errors.rubric?.message ? <p className="text-xs text-destructive">{errors.rubric?.root?.message ?? errors.rubric?.message}</p> : null}
              {rubric.fields.length === 0 && <p className="text-sm text-muted-foreground">No rubric, grade with a single score.</p>}
              {rubric.fields.map((f, i) => (
                <div key={f.id} className="grid grid-cols-[1fr_6rem_auto] items-start gap-2 rounded-lg border p-2 sm:grid-cols-[1fr_1.4fr_6rem_auto]">
                  <Input placeholder="Criterion" aria-label="Criterion title" aria-invalid={!!errors.rubric?.[i]?.title} {...form.register(`rubric.${i}.title`)} />
                  <Input placeholder="Description (optional)" aria-label="Criterion description" className="col-span-3 row-start-2 sm:col-span-1 sm:row-start-auto" {...form.register(`rubric.${i}.description`)} />
                  <Input type="number" step="0.5" min={0} aria-label="Points" aria-invalid={!!errors.rubric?.[i]?.max_points} {...form.register(`rubric.${i}.max_points`)} />
                  <Button type="button" variant="ghost" size="icon" onClick={() => rubric.remove(i)} aria-label="Remove criterion">
                    <X />
                  </Button>
                </div>
              ))}
            </CardContent>
          </Card>
        </div>

        <div className="space-y-6">
          <Card>
            <CardHeader>
              <CardTitle>Placement</CardTitle>
            </CardHeader>
            <CardContent className="space-y-4">
              <Field label="Course" htmlFor="a-course" error={errors.course_id?.message}>
                <NativeSelect id="a-course" disabled={!!id} {...form.register('course_id')}>
                  {courses.data?.map((c) => (
                    <option key={c.id} value={c.id}>
                      {c.title}
                    </option>
                  ))}
                </NativeSelect>
              </Field>
              <Field label="Week" htmlFor="a-week" error={errors.week_id?.message}>
                <NativeSelect id="a-week" {...form.register('week_id', { onChange: () => form.setValue('lesson_id', '') })}>
                  <option value="">No specific week</option>
                  {curriculum.data?.weeks.map((w) => (
                    <option key={w.id} value={w.id}>
                      Week {w.week_number}: {w.title}
                    </option>
                  ))}
                </NativeSelect>
              </Field>
              <Field label="Lesson (optional)" htmlFor="a-lesson" error={errors.lesson_id?.message}>
                <NativeSelect id="a-lesson" disabled={!weekId} {...form.register('lesson_id')}>
                  <option value="">None</option>
                  {(weekLessons.data ?? [])
                    .filter((l) => l.week_number === weekNumber)
                    .map((l) => (
                      <option key={l.id} value={l.id}>
                        Day {l.day_number}: {l.title}
                      </option>
                    ))}
                </NativeSelect>
              </Field>
            </CardContent>
          </Card>
          <Card>
            <CardHeader>
              <CardTitle>Submission & grading</CardTitle>
            </CardHeader>
            <CardContent className="space-y-4">
              <Field label="Assignment type" htmlFor="a-kind">
                <NativeSelect id="a-kind" {...form.register('kind')}>
                  {(Object.keys(ASSIGNMENT_KIND_LABELS) as AssignmentKind[]).map((k) => (
                    <option key={k} value={k}>
                      {ASSIGNMENT_KIND_LABELS[k]}
                    </option>
                  ))}
                </NativeSelect>
              </Field>
              <Field label="Maximum grade" htmlFor="a-max" error={errors.max_grade?.message}>
                <Input id="a-max" type="number" step="0.5" min={1} {...form.register('max_grade')} />
              </Field>
              <Field label="Publication date" htmlFor="a-pub" hint={`Optional. Hidden from students until then (${timezone}).`}>
                <Input id="a-pub" type="datetime-local" {...form.register('publish_at')} />
              </Field>
              <Field label="Due date" htmlFor="a-due" error={errors.due_at?.message} hint={`Stored in UTC, shown in each user's timezone.`}>
                <Input id="a-due" type="datetime-local" {...form.register('due_at')} />
              </Field>
              <Field label="Late submissions" htmlFor="a-late">
                <NativeSelect id="a-late" {...form.register('late_policy')}>
                  {Object.entries(LATE_POLICY_LABELS).map(([k, v]) => (
                    <option key={k} value={k}>
                      {v}
                    </option>
                  ))}
                </NativeSelect>
              </Field>
              {latePolicy === 'penalty' && (
                <Field label="Late penalty (%)" htmlFor="a-pen" error={errors.late_penalty_percent?.message}>
                  <Input id="a-pen" type="number" min={0} max={100} {...form.register('late_penalty_percent')} />
                </Field>
              )}
              <Controller
                control={form.control}
                name="allow_resubmission"
                render={({ field }) => (
                  <Label className="font-normal">
                    <Switch checked={field.value} onCheckedChange={field.onChange} />
                    Allow resubmission before the deadline
                  </Label>
                )}
              />
            </CardContent>
          </Card>
          {acceptsFiles && (
            <Card>
              <CardHeader>
                <CardTitle>File uploads</CardTitle>
                <CardDescription>Select allowed types, or none to accept every supported type.</CardDescription>
              </CardHeader>
              <CardContent className="space-y-4">
                <Field label="Maximum upload size (MB)" htmlFor="a-mb" error={errors.max_upload_mb?.message}>
                  <Input id="a-mb" type="number" min={1} max={100} {...form.register('max_upload_mb')} />
                </Field>
                <Controller
                  control={form.control}
                  name="allowed_file_types"
                  render={({ field }) => (
                    <div className="space-y-3">
                      {FILE_GROUPS.map((g) => (
                        <div key={g.label}>
                          <div className="mb-1 text-xs text-muted-foreground">{g.label}</div>
                          <div className="flex flex-wrap gap-1.5">
                            {g.exts
                              .filter((e) => !supported.size || supported.has(e))
                              .map((e) => {
                                const on = field.value?.includes(e)
                                return (
                                  <button
                                    key={e}
                                    type="button"
                                    aria-pressed={on}
                                    onClick={() => field.onChange(on ? field.value.filter((x) => x !== e) : [...(field.value ?? []), e])}
                                    className={cn('rounded-md border px-2 py-0.5 font-mono text-xs transition hover:bg-accent', on && 'border-primary bg-primary/15 text-primary')}
                                  >
                                    .{e}
                                  </button>
                                )
                              })}
                          </div>
                        </div>
                      ))}
                    </div>
                  )}
                />
              </CardContent>
            </Card>
          )}
          {id && (
            <Card>
              <CardHeader>
                <CardTitle>Danger zone</CardTitle>
              </CardHeader>
              <CardContent className="flex flex-col gap-2">
                {status !== 'archived' && (
                  <Button type="button" variant="outline" onClick={() => save('archived')}>
                    <Archive /> Archive assignment
                  </Button>
                )}
                {status === 'published' && (
                  <Button type="button" variant="ghost" onClick={() => save('draft')}>
                    Unpublish (back to draft)
                  </Button>
                )}
                <Button type="button" variant="ghost" className="text-destructive" onClick={() => setConfirmDelete(true)}>
                  <Trash2 /> Delete
                </Button>
              </CardContent>
            </Card>
          )}
        </div>
      </div>
      <ConfirmDialog
        open={confirmDelete}
        onOpenChange={setConfirmDelete}
        title="Delete this assignment?"
        description="Assignments with student work cannot be deleted; archive them instead."
        destructive
        confirmLabel="Delete"
        loading={del.isPending}
        onConfirm={() => del.mutate()}
      />
    </form>
  )
}
