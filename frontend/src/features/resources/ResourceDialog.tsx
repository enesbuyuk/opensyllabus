import { zodResolver } from '@hookform/resolvers/zod'
import { useQuery, useQueryClient } from '@tanstack/react-query'
import * as React from 'react'
import { Controller, useForm } from 'react-hook-form'
import { toast } from 'sonner'
import { z } from 'zod'
import { coursesApi, resourcesApi } from '@/api/endpoints'
import { AttachmentList, FileDropzone } from '@/components/files'
import { Field } from '@/components/form'
import { MarkdownEditor } from '@/components/markdown-editor'
import { Button } from '@/components/ui/button'
import { Dialog, DialogContent, DialogDescription, DialogFooter, DialogHeader, DialogTitle } from '@/components/ui/dialog'
import { Input, NativeSelect } from '@/components/ui/input'
import { useCourses } from '@/hooks/queries'
import { errorMessage, handleFormError } from '@/lib/errors'
import { RESOURCE_KIND_LABELS } from '@/lib/labels'
import type { FileMeta, Resource, ResourceKind } from '@/types/api'
import { asFileMeta } from '@/components/resource-list'

const schema = z.object({
  course_id: z.string().min(1, 'Select a course'),
  module_id: z.string(),
  week_id: z.string(),
  lesson_id: z.string(),
  title: z.string().trim().min(1, 'Title is required').max(200),
  description: z.string().max(5000),
  kind: z.enum(['pdf', 'image', 'markdown', 'code', 'link', 'video', 'datasheet', 'lab']),
  url: z.string().trim().refine((v) => !v || /^https?:\/\/.+/.test(v), 'Must be an http(s) URL'),
  content_md: z.string().max(200_000),
  visibility: z.enum(['students', 'instructor']),
})
type FormValues = z.infer<typeof schema>

export interface ResourceDefaults {
  course_id?: string
  week_id?: string
  lesson_id?: string
}

/** Create or edit a library resource (file, link, video or Markdown note). */
export function ResourceDialog({ open, onOpenChange, resource, defaults }: { open: boolean; onOpenChange: (o: boolean) => void; resource?: Resource | null; defaults?: ResourceDefaults }) {
  const courses = useCourses()
  const qc = useQueryClient()
  const [file, setFile] = React.useState<FileMeta | null>(null)
  const [uploading, setUploading] = React.useState(false)
  const form = useForm<FormValues>({ resolver: zodResolver(schema) })
  const courseId = form.watch('course_id')
  const weekId = form.watch('week_id')
  const kind = form.watch('kind')
  const curriculum = useQuery({ queryKey: ['curriculum', courseId], queryFn: () => coursesApi.curriculum(courseId), enabled: open && !!courseId })
  const lessons = useQuery({ queryKey: ['course-lessons', courseId], queryFn: () => coursesApi.lessons(courseId), enabled: open && !!courseId })

  React.useEffect(() => {
    if (!open) return
    setFile(resource ? asFileMeta(resource) : null)
    form.reset({
      course_id: resource?.course_id ?? defaults?.course_id ?? courses.data?.[0]?.id ?? '',
      module_id: resource?.module_id ?? '',
      week_id: resource?.week_id ?? defaults?.week_id ?? '',
      lesson_id: resource?.lesson_id ?? defaults?.lesson_id ?? '',
      title: resource?.title ?? '',
      description: resource?.description ?? '',
      kind: resource?.kind ?? 'pdf',
      url: resource?.url ?? '',
      content_md: resource?.content_md ?? '',
      visibility: resource?.visibility ?? 'students',
    })
  }, [open, resource, defaults, courses.data, form])

  const weekNumber = curriculum.data?.weeks.find((w) => w.id === weekId)?.week_number
  const weekLessons = (lessons.data ?? []).filter((l) => !weekNumber || l.week_number === weekNumber)

  const upload = async (files: File[]) => {
    const f = files[0]
    if (!f) return
    setUploading(true)
    try {
      const meta = await resourcesApi.uploadFile(f)
      setFile(meta)
      if (!form.getValues('title')) form.setValue('title', f.name.replace(/\.[^.]+$/, ''))
      if (meta.content_type.startsWith('image/')) form.setValue('kind', 'image')
      else if (meta.content_type === 'application/pdf' && !['datasheet', 'lab'].includes(form.getValues('kind'))) form.setValue('kind', 'pdf')
    } catch (e) {
      toast.error(errorMessage(e))
    } finally {
      setUploading(false)
    }
  }

  const onSubmit = form.handleSubmit(async (v) => {
    const body = { ...v, file_id: file?.id ?? '' }
    if (!body.file_id && !body.url && !body.content_md.trim()) {
      toast.error('Attach a file, add a URL or write content')
      return
    }
    try {
      if (resource) await resourcesApi.update(resource.id, body)
      else await resourcesApi.create(body)
      toast.success(resource ? 'Resource updated' : 'Resource added')
      qc.invalidateQueries({ queryKey: ['resources'] })
      qc.invalidateQueries({ queryKey: ['week'] })
      qc.invalidateQueries({ queryKey: ['lesson'] })
      onOpenChange(false)
    } catch (e) {
      handleFormError(e, form.setError)
    }
  })
  const { errors, isSubmitting } = form.formState
  const needsUrl = kind === 'link' || kind === 'video'

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent wide>
        <DialogHeader>
          <DialogTitle>{resource ? 'Edit resource' : 'Add resource'}</DialogTitle>
          <DialogDescription>Files are stored privately and served only to enrolled students.</DialogDescription>
        </DialogHeader>
        <form onSubmit={onSubmit} className="space-y-4">
          <div className="grid gap-4 sm:grid-cols-2">
            <Field label="Title" htmlFor="r-title" error={errors.title?.message} required>
              <Input id="r-title" {...form.register('title')} />
            </Field>
            <Field label="Type" htmlFor="r-kind">
              <NativeSelect id="r-kind" {...form.register('kind')}>
                {(Object.keys(RESOURCE_KIND_LABELS) as ResourceKind[]).map((k) => (
                  <option key={k} value={k}>
                    {RESOURCE_KIND_LABELS[k]}
                  </option>
                ))}
              </NativeSelect>
            </Field>
          </div>
          <Field label="Description" htmlFor="r-desc">
            <Input id="r-desc" {...form.register('description')} />
          </Field>
          <div className="grid gap-4 sm:grid-cols-2">
            <Field label="Course" htmlFor="r-course" error={errors.course_id?.message}>
              <NativeSelect id="r-course" disabled={!!resource} {...form.register('course_id')}>
                {courses.data?.map((c) => (
                  <option key={c.id} value={c.id}>
                    {c.title}
                  </option>
                ))}
              </NativeSelect>
            </Field>
            <Field label="Module" htmlFor="r-module" error={errors.module_id?.message}>
              <NativeSelect id="r-module" {...form.register('module_id')}>
                <option value="">None</option>
                {curriculum.data?.modules.map((m) => (
                  <option key={m.id} value={m.id}>
                    {m.title}
                  </option>
                ))}
              </NativeSelect>
            </Field>
            <Field label="Week" htmlFor="r-week" error={errors.week_id?.message}>
              <NativeSelect id="r-week" {...form.register('week_id')}>
                <option value="">None</option>
                {curriculum.data?.weeks.map((w) => (
                  <option key={w.id} value={w.id}>
                    Week {w.week_number}: {w.title}
                  </option>
                ))}
              </NativeSelect>
            </Field>
            <Field label="Lesson" htmlFor="r-lesson" error={errors.lesson_id?.message}>
              <NativeSelect id="r-lesson" {...form.register('lesson_id')}>
                <option value="">None</option>
                {weekLessons.map((l) => (
                  <option key={l.id} value={l.id}>
                    W{l.week_number} D{l.day_number}: {l.title}
                  </option>
                ))}
              </NativeSelect>
            </Field>
          </div>
          {needsUrl ? (
            <Field label="URL" htmlFor="r-url" error={errors.url?.message} required>
              <Input id="r-url" type="url" placeholder="https://" {...form.register('url')} />
            </Field>
          ) : (
            <Field label="File" hint="Optional when you provide a URL or Markdown content.">
              {file ? <AttachmentList files={[file]} onRemove={() => setFile(null)} /> : <FileDropzone onFiles={upload} uploading={uploading} hint="One file · PDF, images, code, CSV, ZIP…" />}
            </Field>
          )}
          {!needsUrl && (
            <Field label="External URL (optional)" htmlFor="r-url2" error={errors.url?.message}>
              <Input id="r-url2" type="url" placeholder="https://" {...form.register('url')} />
            </Field>
          )}
          {(kind === 'markdown' || kind === 'code' || kind === 'lab') && (
            <Field label="Content" hint="Markdown with code blocks.">
              <Controller control={form.control} name="content_md" render={({ field }) => <MarkdownEditor value={field.value} onChange={field.onChange} minRows={8} />} />
            </Field>
          )}
          <Field label="Visibility" htmlFor="r-vis">
            <NativeSelect id="r-vis" {...form.register('visibility')}>
              <option value="students">Visible to enrolled students</option>
              <option value="instructor">Instructor only</option>
            </NativeSelect>
          </Field>
          <DialogFooter>
            <Button type="button" variant="outline" onClick={() => onOpenChange(false)}>
              Cancel
            </Button>
            <Button type="submit" loading={isSubmitting} disabled={uploading}>
              {resource ? 'Save changes' : 'Add resource'}
            </Button>
          </DialogFooter>
        </form>
      </DialogContent>
    </Dialog>
  )
}
