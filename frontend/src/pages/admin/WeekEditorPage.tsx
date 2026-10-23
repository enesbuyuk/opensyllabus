import { zodResolver } from '@hookform/resolvers/zod'
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query'
import { ArrowDown, ArrowUp, Copy, Eye, MoreHorizontal, Pencil, Plus, Send, Trash2 } from 'lucide-react'
import * as React from 'react'
import { Controller, useForm } from 'react-hook-form'
import { Link, useNavigate, useParams } from 'react-router'
import { toast } from 'sonner'
import { z } from 'zod'
import { coursesApi, curriculumApi } from '@/api/endpoints'
import { AssignmentRows } from '@/components/assignment-list'
import { Field } from '@/components/form'
import { ListEditor } from '@/components/list-editor'
import { MarkdownEditor } from '@/components/markdown-editor'
import { ErrorState, PageHeader, PageSkeleton } from '@/components/page'
import { ResourceList } from '@/components/resource-list'
import { PublishBadge } from '@/components/status'
import { ConfirmDialog } from '@/components/ui/alert-dialog'
import { Button } from '@/components/ui/button'
import { Card, CardContent, CardDescription, CardFooter, CardHeader, CardTitle } from '@/components/ui/card'
import { Dialog, DialogContent, DialogFooter, DialogHeader, DialogTitle } from '@/components/ui/dialog'
import { DropdownMenu, DropdownMenuContent, DropdownMenuItem, DropdownMenuSeparator, DropdownMenuTrigger } from '@/components/ui/dropdown-menu'
import { Input, NativeSelect } from '@/components/ui/input'
import { Label } from '@/components/ui/label'
import { Checkbox } from '@/components/ui/misc'
import { ResourceDialog } from '@/features/resources/ResourceDialog'
import { useBreadcrumbs } from '@/lib/breadcrumbs'
import { errorMessage, handleFormError } from '@/lib/errors'
import { formatMinutes } from '@/lib/utils'
import type { LessonListItem, WeekDetail } from '@/types/api'

const schema = z.object({
  title: z.string().trim().min(1, 'Title is required').max(200),
  description: z.string().max(20_000),
  module_id: z.string(),
  objectives: z.array(z.string()),
  instructor_notes: z.string().max(20_000),
  estimated_minutes: z.coerce.number<number>().int().min(0).max(10_000),
  available_from: z.string(),
})
type FormValues = z.infer<typeof schema>

function WeekForm({ week }: { week: WeekDetail }) {
  const qc = useQueryClient()
  const curriculum = useQuery({ queryKey: ['curriculum', week.course_id], queryFn: () => coursesApi.curriculum(week.course_id) })
  const form = useForm<FormValues>({ resolver: zodResolver(schema) })
  React.useEffect(() => {
    form.reset({
      title: week.title,
      description: week.description,
      module_id: week.module_id ?? '',
      objectives: week.objectives,
      instructor_notes: week.instructor_notes,
      estimated_minutes: week.estimated_minutes,
      available_from: week.available_from ?? '',
    })
  }, [week, form])
  const onSubmit = form.handleSubmit(async (v) => {
    try {
      await curriculumApi.updateWeek(week.id, v)
      qc.invalidateQueries({ queryKey: ['week', week.id] })
      qc.invalidateQueries({ queryKey: ['curriculum', week.course_id] })
      toast.success('Week saved')
    } catch (e) {
      handleFormError(e, form.setError)
    }
  })
  const { errors, isSubmitting, isDirty } = form.formState
  return (
    <Card>
      <form onSubmit={onSubmit}>
        <CardHeader>
          <CardTitle>Week details</CardTitle>
        </CardHeader>
        <CardContent className="space-y-4">
          <div className="grid gap-4 sm:grid-cols-2">
            <Field label="Title" htmlFor="w-title" error={errors.title?.message} required>
              <Input id="w-title" {...form.register('title')} />
            </Field>
            <Field label="Module" htmlFor="w-module">
              <NativeSelect id="w-module" {...form.register('module_id')}>
                <option value="">None</option>
                {curriculum.data?.modules.map((m) => (
                  <option key={m.id} value={m.id}>
                    {m.title}
                  </option>
                ))}
              </NativeSelect>
            </Field>
            <Field label="Estimated learning time (minutes)" htmlFor="w-min" error={errors.estimated_minutes?.message} hint="Leave 0 to sum the lesson estimates.">
              <Input id="w-min" type="number" min={0} {...form.register('estimated_minutes')} />
            </Field>
            <Field label="Available from (optional)" htmlFor="w-avail" hint="A published week stays locked for students until this date.">
              <Input id="w-avail" type="date" {...form.register('available_from')} />
            </Field>
          </div>
          <Field label="Description">
            <Controller control={form.control} name="description" render={({ field }) => <MarkdownEditor value={field.value ?? ''} onChange={field.onChange} minRows={5} />} />
          </Field>
          <Field label="Learning objectives">
            <Controller control={form.control} name="objectives" render={({ field }) => <ListEditor value={field.value ?? []} onChange={field.onChange} placeholder="Add an objective for this week" />} />
          </Field>
          <Field label="Instructor notes" hint="Shown to students on the week page (e.g. preparation, what to bring to the lab).">
            <Controller control={form.control} name="instructor_notes" render={({ field }) => <MarkdownEditor value={field.value ?? ''} onChange={field.onChange} minRows={4} />} />
          </Field>
        </CardContent>
        <CardFooter className="justify-end">
          <Button type="submit" loading={isSubmitting} disabled={!isDirty}>
            Save week
          </Button>
        </CardFooter>
      </form>
    </Card>
  )
}

function LessonsCard({ week }: { week: WeekDetail }) {
  const qc = useQueryClient()
  const navigate = useNavigate()
  const curriculum = useQuery({ queryKey: ['curriculum', week.course_id], queryFn: () => coursesApi.curriculum(week.course_id) })
  const [deleting, setDeleting] = React.useState<LessonListItem | null>(null)
  const [dupFor, setDupFor] = React.useState<{ lesson: LessonListItem; target: string } | null>(null)
  const refresh = () => {
    qc.invalidateQueries({ queryKey: ['week', week.id] })
    qc.invalidateQueries({ queryKey: ['curriculum', week.course_id] })
  }
  const onErr = (e: unknown) => toast.error(errorMessage(e))
  const add = useMutation({
    mutationFn: (day: number) => curriculumApi.createLesson(week.id, { title: `Day ${day} lesson`, day_number: day }),
    onSuccess: (l) => navigate(`/admin/lessons/${l.id}`),
    onError: onErr,
  })
  const patch = useMutation({
    mutationFn: ({ id, status }: { id: string; status: 'draft' | 'published' }) => curriculumApi.updateLesson(id, { status }),
    onSuccess: (_, v) => {
      refresh()
      toast.success(v.status === 'published' ? 'Lesson published' : 'Lesson unpublished')
    },
    onError: onErr,
  })
  const reorder = useMutation({ mutationFn: (ids: string[]) => curriculumApi.reorderLessons(week.id, ids), onSuccess: refresh, onError: onErr })
  const del = useMutation({
    mutationFn: (id: string) => curriculumApi.deleteLesson(id),
    onSuccess: () => {
      setDeleting(null)
      refresh()
      toast.success('Lesson deleted')
    },
    onError: onErr,
  })
  const dup = useMutation({
    mutationFn: ({ id, target }: { id: string; target: string }) => curriculumApi.duplicateLesson(id, target),
    onSuccess: (r) => {
      setDupFor(null)
      refresh()
      toast.success('Lesson duplicated as a draft')
      navigate(`/admin/lessons/${r.id}`)
    },
    onError: onErr,
  })

  const ordered = [...week.lessons].sort((a, b) => a.day_number - b.day_number || a.position - b.position)
  const move = (idx: number, dir: -1 | 1) => {
    const ids = ordered.map((l) => l.id)
    const j = idx + dir
    if (j < 0 || j >= ids.length) return
    ;[ids[idx], ids[j]] = [ids[j], ids[idx]]
    reorder.mutate(ids)
  }
  const days = Array.from(new Set([1, 2, 3, ...ordered.map((l) => l.day_number)])).sort((a, b) => a - b)

  return (
    <Card>
      <CardHeader className="flex-row items-start justify-between gap-3">
        <div className="space-y-1">
          <CardTitle>Lessons</CardTitle>
          <CardDescription>Publish lessons independently. Reorder with the arrows.</CardDescription>
        </div>
        <DropdownMenu>
          <DropdownMenuTrigger asChild>
            <Button size="sm" loading={add.isPending}>
              <Plus /> Add lesson
            </Button>
          </DropdownMenuTrigger>
          <DropdownMenuContent align="end">
            {[1, 2, 3, 4, 5, 6, 7].map((d) => (
              <DropdownMenuItem key={d} onSelect={() => add.mutate(d)}>
                Day {d}
              </DropdownMenuItem>
            ))}
          </DropdownMenuContent>
        </DropdownMenu>
      </CardHeader>
      <CardContent className="space-y-4">
        {days.map((day) => {
          const items = ordered.filter((l) => l.day_number === day)
          return (
            <div key={day}>
              <div className="mb-1.5 text-xs font-semibold text-muted-foreground uppercase">Day {day}</div>
              {items.length === 0 ? (
                <button type="button" onClick={() => add.mutate(day)} className="w-full rounded-lg border border-dashed px-3 py-3 text-left text-[13px] text-muted-foreground hover:bg-accent">
                  + Add a lesson for day {day}
                </button>
              ) : (
                <ul className="divide-y rounded-lg border">
                  {items.map((l) => {
                    const idx = ordered.indexOf(l)
                    return (
                      <li key={l.id} className="flex items-center gap-2 px-3 py-2">
                        <div className="flex flex-col">
                          <button type="button" className="rounded p-0.5 text-muted-foreground hover:bg-accent disabled:opacity-30" disabled={idx === 0 || reorder.isPending} onClick={() => move(idx, -1)} aria-label="Move up">
                            <ArrowUp className="size-3.5" />
                          </button>
                          <button type="button" className="rounded p-0.5 text-muted-foreground hover:bg-accent disabled:opacity-30" disabled={idx === ordered.length - 1 || reorder.isPending} onClick={() => move(idx, 1)} aria-label="Move down">
                            <ArrowDown className="size-3.5" />
                          </button>
                        </div>
                        <div className="min-w-0 flex-1">
                          <Link to={`/admin/lessons/${l.id}`} className="block truncate text-sm font-medium hover:underline">
                            {l.title}
                          </Link>
                          <div className="truncate text-xs text-muted-foreground">
                            {formatMinutes(l.estimated_minutes)}
                            {l.summary ? ` · ${l.summary}` : ''}
                          </div>
                        </div>
                        <PublishBadge status={l.status} />
                        <DropdownMenu>
                          <DropdownMenuTrigger asChild>
                            <Button variant="ghost" size="icon-sm" aria-label={`Actions for ${l.title}`}>
                              <MoreHorizontal />
                            </Button>
                          </DropdownMenuTrigger>
                          <DropdownMenuContent align="end">
                            <DropdownMenuItem onSelect={() => navigate(`/admin/lessons/${l.id}`)}>
                              <Pencil /> Edit
                            </DropdownMenuItem>
                            <DropdownMenuItem onSelect={() => navigate(`/lessons/${l.id}`)}>
                              <Eye /> Preview
                            </DropdownMenuItem>
                            <DropdownMenuItem onSelect={() => patch.mutate({ id: l.id, status: l.status === 'published' ? 'draft' : 'published' })}>
                              <Send /> {l.status === 'published' ? 'Unpublish' : 'Publish'}
                            </DropdownMenuItem>
                            <DropdownMenuItem onSelect={() => setDupFor({ lesson: l, target: week.id })}>
                              <Copy /> Duplicate…
                            </DropdownMenuItem>
                            <DropdownMenuSeparator />
                            <DropdownMenuItem destructive onSelect={() => setDeleting(l)}>
                              <Trash2 /> Delete
                            </DropdownMenuItem>
                          </DropdownMenuContent>
                        </DropdownMenu>
                      </li>
                    )
                  })}
                </ul>
              )}
            </div>
          )
        })}
      </CardContent>
      <ConfirmDialog
        open={!!deleting}
        onOpenChange={(o) => !o && setDeleting(null)}
        title={`Delete “${deleting?.title}”?`}
        description="Student completion records for this lesson are removed as well."
        destructive
        confirmLabel="Delete lesson"
        loading={del.isPending}
        onConfirm={() => deleting && del.mutate(deleting.id)}
      />
      <Dialog open={!!dupFor} onOpenChange={(o) => !o && setDupFor(null)}>
        <DialogContent>
          <DialogHeader>
            <DialogTitle>Duplicate lesson</DialogTitle>
          </DialogHeader>
          <Field label="Copy into week" htmlFor="dup-week">
            <NativeSelect id="dup-week" value={dupFor?.target ?? ''} onChange={(e) => setDupFor((d) => (d ? { ...d, target: e.target.value } : d))}>
              {curriculum.data?.weeks.map((w) => (
                <option key={w.id} value={w.id}>
                  Week {w.week_number}: {w.title}
                </option>
              ))}
            </NativeSelect>
          </Field>
          <DialogFooter>
            <Button variant="outline" onClick={() => setDupFor(null)}>
              Cancel
            </Button>
            <Button onClick={() => dupFor && dup.mutate({ id: dupFor.lesson.id, target: dupFor.target })} loading={dup.isPending}>
              Duplicate
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
    </Card>
  )
}

export function WeekEditorPage() {
  const { id = '' } = useParams()
  const qc = useQueryClient()
  const q = useQuery({ queryKey: ['week', id], queryFn: () => curriculumApi.week(id) })
  const [publishOpen, setPublishOpen] = React.useState(false)
  const [cascade, setCascade] = React.useState(true)
  const [resourceOpen, setResourceOpen] = React.useState(false)
  useBreadcrumbs([
    { label: 'Course Management', to: '/admin/courses' },
    ...(q.data ? [{ label: 'Curriculum', to: `/admin/courses/${q.data.course_id}?tab=curriculum` }] : []),
    { label: q.data ? `Week ${q.data.week_number}` : 'Week' },
  ])
  const status = useMutation({
    mutationFn: (s: 'draft' | 'published') => curriculumApi.updateWeek(id, { status: s, cascade_lessons: s === 'published' ? cascade : false }),
    onSuccess: (_, s) => {
      setPublishOpen(false)
      qc.invalidateQueries({ queryKey: ['week', id] })
      qc.invalidateQueries({ queryKey: ['curriculum'] })
      toast.success(s === 'published' ? 'Week published' : 'Week moved to draft')
    },
    onError: (e) => toast.error(errorMessage(e)),
  })
  if (q.isLoading) return <PageSkeleton />
  if (q.error || !q.data) return <ErrorState error={q.error} onRetry={() => q.refetch()} />
  const w = q.data
  const drafts = w.lessons.filter((l) => l.status === 'draft').length

  return (
    <div className="space-y-6">
      <PageHeader
        eyebrow={`Week ${w.week_number}${w.module ? ` · ${w.module.title}` : ''}`}
        title={w.title}
        actions={
          <>
            <PublishBadge status={w.status} />
            <Button asChild variant="outline">
              <Link to={`/weeks/${w.id}`}>
                <Eye /> Preview
              </Link>
            </Button>
            {w.status === 'draft' ? (
              <Button onClick={() => setPublishOpen(true)}>
                <Send /> Publish week
              </Button>
            ) : (
              <Button variant="outline" onClick={() => status.mutate('draft')} loading={status.isPending}>
                Unpublish
              </Button>
            )}
          </>
        }
      />
      <div className="grid gap-6 xl:grid-cols-5">
        <div className="xl:col-span-3">
          <WeekForm week={w} />
        </div>
        <div className="space-y-6 xl:col-span-2">
          <LessonsCard week={w} />
          <Card>
            <CardHeader className="flex-row items-start justify-between gap-3">
              <CardTitle>Assignments</CardTitle>
              <Button asChild variant="outline" size="sm">
                <Link to={`/admin/assignments/new?course=${w.course_id}&week=${w.id}`}>
                  <Plus /> New
                </Link>
              </Button>
            </CardHeader>
            <CardContent>
              <AssignmentRows items={w.assignments} emptyText="No assignments in this week." />
            </CardContent>
          </Card>
          <Card>
            <CardHeader className="flex-row items-start justify-between gap-3">
              <CardTitle>Week resources</CardTitle>
              <Button variant="outline" size="sm" onClick={() => setResourceOpen(true)}>
                <Plus /> Add
              </Button>
            </CardHeader>
            <CardContent>
              <ResourceList resources={w.resources} emptyText="No resources attached." />
            </CardContent>
          </Card>
        </div>
      </div>
      <Dialog open={publishOpen} onOpenChange={setPublishOpen}>
        <DialogContent>
          <DialogHeader>
            <DialogTitle>Publish week {w.week_number}?</DialogTitle>
          </DialogHeader>
          <p className="text-sm text-muted-foreground">
            Enrolled students will be able to open this week{w.available_from ? ` from ${w.available_from}` : ' immediately'} and will be notified about published lessons.
          </p>
          {drafts > 0 && (
            <Label className="font-normal">
              <Checkbox checked={cascade} onCheckedChange={(c) => setCascade(c === true)} />
              Also publish the {drafts} draft lesson{drafts === 1 ? '' : 's'} in this week
            </Label>
          )}
          <DialogFooter>
            <Button variant="outline" onClick={() => setPublishOpen(false)}>
              Cancel
            </Button>
            <Button onClick={() => status.mutate('published')} loading={status.isPending}>
              Publish
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
      <ResourceDialog open={resourceOpen} onOpenChange={setResourceOpen} defaults={{ course_id: w.course_id, week_id: w.id }} />
    </div>
  )
}
