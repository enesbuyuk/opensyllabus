import { zodResolver } from '@hookform/resolvers/zod'
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query'
import { Eye, Plus, Send } from 'lucide-react'
import * as React from 'react'
import { Controller, useForm } from 'react-hook-form'
import { Link, useParams } from 'react-router'
import { toast } from 'sonner'
import { z } from 'zod'
import { coursesApi, curriculumApi } from '@/api/endpoints'
import { Field } from '@/components/form'
import { ListEditor } from '@/components/list-editor'
import { MarkdownEditor } from '@/components/markdown-editor'
import { ErrorState, PageHeader, PageSkeleton } from '@/components/page'
import { ResourceList } from '@/components/resource-list'
import { PublishBadge } from '@/components/status'
import { Button } from '@/components/ui/button'
import { Card, CardContent, CardDescription, CardFooter, CardHeader, CardTitle } from '@/components/ui/card'
import { Input, NativeSelect, Textarea } from '@/components/ui/input'
import { Checkbox } from '@/components/ui/misc'
import { VideoEmbed } from '@/components/video-embed'
import { ResourceDialog } from '@/features/resources/ResourceDialog'
import { useBreadcrumbs } from '@/lib/breadcrumbs'
import { errorMessage, handleFormError } from '@/lib/errors'

const schema = z.object({
  title: z.string().trim().min(1, 'Title is required').max(200),
  day_number: z.coerce.number<number>().int().min(1).max(7),
  summary: z.string().max(2000),
  content_md: z.string().max(200_000),
  lab_md: z.string().max(100_000),
  video_url: z.string().trim().refine((v) => !v || /^https:\/\/.+/.test(v), 'Must be an https link'),
  objectives: z.array(z.string()),
  estimated_minutes: z.coerce.number<number>().int().min(0).max(10_000),
  prerequisite_ids: z.array(z.string()),
})
type FormValues = z.infer<typeof schema>

const CONTENT_TEMPLATE = `## Overview

Explain the key idea of this session.

## Key concepts

- Concept one
- Concept two

## Worked example

\`\`\`python
# Example code
voltage = 5.0  # V
resistance = 1_000  # Ω
current = voltage / resistance
print(f"I = {current * 1000:.1f} mA")
\`\`\`

## Summary
`

export function LessonEditorPage() {
  const { id = '' } = useParams()
  const qc = useQueryClient()
  const q = useQuery({ queryKey: ['lesson', id], queryFn: () => curriculumApi.lesson(id) })
  const lessons = useQuery({ queryKey: ['course-lessons', q.data?.course_id], queryFn: () => coursesApi.lessons(q.data!.course_id), enabled: !!q.data })
  const [resourceOpen, setResourceOpen] = React.useState(false)
  const form = useForm<FormValues>({ resolver: zodResolver(schema) })
  const l = q.data
  useBreadcrumbs([
    { label: 'Course Management', to: '/admin/courses' },
    ...(l ? [{ label: `Week ${l.week_number}`, to: `/admin/weeks/${l.week_id}` }] : []),
    { label: l?.title ?? 'Lesson' },
  ])

  React.useEffect(() => {
    if (!l) return
    form.reset({
      title: l.title,
      day_number: l.day_number,
      summary: l.summary,
      content_md: l.content_md,
      lab_md: l.lab_md,
      video_url: l.video_url ?? '',
      objectives: l.objectives,
      estimated_minutes: l.estimated_minutes,
      prerequisite_ids: l.prerequisites.map((p) => p.id),
    })
  }, [l, form])

  const refresh = () => {
    qc.invalidateQueries({ queryKey: ['lesson', id] })
    qc.invalidateQueries({ queryKey: ['week', l?.week_id] })
    qc.invalidateQueries({ queryKey: ['curriculum'] })
  }
  const onSubmit = form.handleSubmit(async (v) => {
    try {
      await curriculumApi.updateLesson(id, v)
      refresh()
      toast.success('Lesson saved')
    } catch (e) {
      handleFormError(e, form.setError)
    }
  })
  const status = useMutation({
    mutationFn: async (s: 'draft' | 'published') => {
      if (form.formState.isDirty) await curriculumApi.updateLesson(id, form.getValues())
      return curriculumApi.updateLesson(id, { status: s })
    },
    onSuccess: (_, s) => {
      refresh()
      toast.success(s === 'published' ? (l?.week_status === 'published' ? 'Lesson published, students were notified' : 'Lesson published (visible once the week is published)') : 'Lesson moved to draft')
    },
    onError: (e) => toast.error(errorMessage(e)),
  })

  if (q.isLoading) return <PageSkeleton />
  if (q.error || !l) return <ErrorState error={q.error} onRetry={() => q.refetch()} />
  const { errors, isSubmitting, isDirty } = form.formState
  const video = form.watch('video_url')
  const others = (lessons.data ?? []).filter((x) => x.id !== l.id)

  return (
    <>
    <form onSubmit={onSubmit} className="space-y-6">
      <PageHeader
        eyebrow={`Week ${l.week_number} · ${l.week_title}`}
        title={l.title}
        actions={
          <>
            <PublishBadge status={l.status} />
            <Button asChild variant="outline">
              <Link to={`/lessons/${l.id}`}>
                <Eye /> Preview
              </Link>
            </Button>
            <Button type="submit" variant={l.status === 'published' ? 'default' : 'outline'} loading={isSubmitting} disabled={!isDirty}>
              Save
            </Button>
            {l.status === 'draft' ? (
              <Button type="button" onClick={() => status.mutate('published')} loading={status.isPending}>
                <Send /> Publish
              </Button>
            ) : (
              <Button type="button" variant="ghost" onClick={() => status.mutate('draft')} loading={status.isPending}>
                Unpublish
              </Button>
            )}
          </>
        }
      />
      {l.week_status === 'draft' && <div className="rounded-lg border border-warning/30 bg-warning/10 px-4 py-2.5 text-[13px]">The week is still a draft. Students will see this lesson once both are published.</div>}

      <div className="grid gap-6 xl:grid-cols-3">
        <div className="space-y-6 xl:col-span-2">
          <Card>
            <CardHeader>
              <CardTitle>Content</CardTitle>
              <CardDescription>Markdown with headings, tables, images and fenced code blocks. Use Preview to check the rendering.</CardDescription>
            </CardHeader>
            <CardContent className="space-y-4">
              <Field label="Title" htmlFor="l-title" error={errors.title?.message} required>
                <Input id="l-title" {...form.register('title')} />
              </Field>
              <Field label="Summary" htmlFor="l-summary" error={errors.summary?.message} hint="One or two sentences shown on lesson cards.">
                <Textarea id="l-summary" rows={2} {...form.register('summary')} />
              </Field>
              <Field label="Lesson content" error={errors.content_md?.message}>
                <Controller control={form.control} name="content_md" render={({ field }) => <MarkdownEditor value={field.value ?? ''} onChange={field.onChange} minRows={18} />} />
              </Field>
              {!form.watch('content_md') && (
                <Button type="button" variant="ghost" size="sm" onClick={() => form.setValue('content_md', CONTENT_TEMPLATE, { shouldDirty: true })}>
                  Insert content template
                </Button>
              )}
            </CardContent>
          </Card>
          <Card>
            <CardHeader>
              <CardTitle>Laboratory instructions</CardTitle>
              <CardDescription>Optional practical steps, safety notes and expected measurements.</CardDescription>
            </CardHeader>
            <CardContent>
              <Controller control={form.control} name="lab_md" render={({ field }) => <MarkdownEditor value={field.value ?? ''} onChange={field.onChange} minRows={8} placeholder="1. Set the bench supply to 5 V…" />} />
            </CardContent>
          </Card>
        </div>
        <div className="space-y-6">
          <Card>
            <CardHeader>
              <CardTitle>Settings</CardTitle>
            </CardHeader>
            <CardContent className="space-y-4">
              <div className="grid grid-cols-2 gap-3">
                <Field label="Day" htmlFor="l-day" error={errors.day_number?.message}>
                  <NativeSelect id="l-day" {...form.register('day_number')}>
                    {[1, 2, 3, 4, 5, 6, 7].map((d) => (
                      <option key={d} value={d}>
                        Day {d}
                      </option>
                    ))}
                  </NativeSelect>
                </Field>
                <Field label="Minutes" htmlFor="l-min" error={errors.estimated_minutes?.message}>
                  <Input id="l-min" type="number" min={0} {...form.register('estimated_minutes')} />
                </Field>
              </div>
              <Field label="Video URL" htmlFor="l-video" error={errors.video_url?.message} hint="YouTube and Vimeo links are embedded.">
                <Input id="l-video" type="url" placeholder="https://www.youtube.com/watch?v=…" {...form.register('video_url')} />
              </Field>
              {video && /^https:\/\//.test(video) && <VideoEmbed url={video} title="Video preview" />}
              <Field label="Learning objectives">
                <Controller control={form.control} name="objectives" render={({ field }) => <ListEditor value={field.value ?? []} onChange={field.onChange} placeholder="Add an objective" />} />
              </Field>
            </CardContent>
          </Card>
          <Card>
            <CardHeader>
              <CardTitle>Prerequisites</CardTitle>
              <CardDescription>Students must complete these lessons first.</CardDescription>
            </CardHeader>
            <CardContent>
              <Controller
                control={form.control}
                name="prerequisite_ids"
                render={({ field }) => (
                  <div className="max-h-64 space-y-1 overflow-y-auto">
                    {others.length === 0 && <p className="text-sm text-muted-foreground">No other lessons yet.</p>}
                    {others.map((o) => (
                      <label key={o.id} className="flex cursor-pointer items-center gap-2 rounded-md px-2 py-1 text-sm hover:bg-accent">
                        <Checkbox
                          checked={field.value?.includes(o.id)}
                          onCheckedChange={(c) => field.onChange(c ? [...(field.value ?? []), o.id] : (field.value ?? []).filter((x) => x !== o.id))}
                        />
                        <span className="w-14 shrink-0 text-xs text-muted-foreground tabular-nums">
                          W{o.week_number} D{o.day_number}
                        </span>
                        <span className="truncate">{o.title}</span>
                      </label>
                    ))}
                  </div>
                )}
              />
            </CardContent>
          </Card>
          <Card>
            <CardHeader className="flex-row items-start justify-between gap-3">
              <div className="space-y-1">
                <CardTitle>Attachments</CardTitle>
                <CardDescription>PDFs, images, datasheets and code examples.</CardDescription>
              </div>
              <Button type="button" variant="outline" size="sm" onClick={() => setResourceOpen(true)}>
                <Plus /> Add
              </Button>
            </CardHeader>
            <CardContent>
              <ResourceList resources={l.resources} emptyText="No attachments yet." />
            </CardContent>
            <CardFooter>
              <Button asChild variant="ghost" size="sm">
                <Link to={`/admin/assignments/new?course=${l.course_id}&week=${l.week_id}&lesson=${l.id}`}>
                  <Plus /> Add exercise or assignment
                </Link>
              </Button>
            </CardFooter>
          </Card>
        </div>
      </div>
    </form>
    <ResourceDialog open={resourceOpen} onOpenChange={setResourceOpen} defaults={{ course_id: l.course_id, week_id: l.week_id, lesson_id: l.id }} />
    </>
  )
}
