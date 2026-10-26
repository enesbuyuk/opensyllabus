import { zodResolver } from '@hookform/resolvers/zod'
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query'
import { Archive, Copy, Eye, ImagePlus, MoreHorizontal, Pencil, Plus, Send, Trash2, UserMinus, UserPlus, Wand2 } from 'lucide-react'
import * as React from 'react'
import { Controller, useForm } from 'react-hook-form'
import { Link, useNavigate, useParams, useSearchParams } from 'react-router'
import { toast } from 'sonner'
import { z } from 'zod'
import { coursesApi, curriculumApi, usersApi } from '@/api/endpoints'
import { Field } from '@/components/form'
import { ListEditor } from '@/components/list-editor'
import { MarkdownEditor } from '@/components/markdown-editor'
import { EmptyState, ErrorState, PageHeader, PageSkeleton } from '@/components/page'
import { ScheduleEditor } from '@/features/courses/ScheduleEditor'
import { PublishBadge } from '@/components/status'
import { ConfirmDialog } from '@/components/ui/alert-dialog'
import { Button } from '@/components/ui/button'
import { Card, CardContent, CardDescription, CardFooter, CardHeader, CardTitle } from '@/components/ui/card'
import { Dialog, DialogContent, DialogFooter, DialogHeader, DialogTitle } from '@/components/ui/dialog'
import { DropdownMenu, DropdownMenuContent, DropdownMenuItem, DropdownMenuSeparator, DropdownMenuTrigger } from '@/components/ui/dropdown-menu'
import { Input, NativeSelect } from '@/components/ui/input'
import { Checkbox } from '@/components/ui/misc'
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from '@/components/ui/table'
import { Tabs, TabsContent, TabsList, TabsTrigger } from '@/components/ui/tabs'
import { useBreadcrumbs } from '@/lib/breadcrumbs'
import { errorMessage, handleFormError } from '@/lib/errors'
import { formatDate, formatPlainDate } from '@/lib/utils'
import type { CourseListItem, Module, WeekSummary } from '@/types/api'
import { CourseCover } from '../CoursesPage'

const detailsSchema = z
  .object({
    title: z.string().trim().min(3).max(200),
    slug: z.string().trim().regex(/^[a-z0-9]+(?:-[a-z0-9]+)*$/, 'Use lowercase letters, numbers and hyphens'),
    description: z.string().max(20_000),
    difficulty: z.enum(['beginner', 'intermediate', 'advanced']),
    duration_weeks: z.coerce.number<number>().int().min(1).max(104),
    days_per_week: z.coerce.number<number>().int().min(1).max(7),
    start_date: z.string(),
    end_date: z.string(),
    objectives: z.array(z.string()),
  })
  .refine((v) => !v.start_date || !v.end_date || v.end_date >= v.start_date, { message: 'End date must be after the start date', path: ['end_date'] })
type DetailsForm = z.infer<typeof detailsSchema>

function invalidateCourse(qc: ReturnType<typeof useQueryClient>, id: string) {
  qc.invalidateQueries({ queryKey: ['course', id] })
  qc.invalidateQueries({ queryKey: ['courses'] })
  qc.invalidateQueries({ queryKey: ['curriculum', id] })
}

function DetailsTab({ course }: { course: CourseListItem }) {
  const qc = useQueryClient()
  const navigate = useNavigate()
  const coverRef = React.useRef<HTMLInputElement>(null)
  const [confirm, setConfirm] = React.useState<null | 'archive' | 'delete'>(null)
  const form = useForm<DetailsForm>({ resolver: zodResolver(detailsSchema) })
  React.useEffect(() => {
    form.reset({
      title: course.title,
      slug: course.slug,
      description: course.description,
      difficulty: course.difficulty,
      duration_weeks: course.duration_weeks,
      days_per_week: course.days_per_week,
      start_date: course.start_date ?? '',
      end_date: course.end_date ?? '',
      objectives: course.objectives,
    })
  }, [course, form])

  const onSubmit = form.handleSubmit(async (v) => {
    try {
      await coursesApi.update(course.id, v)
      invalidateCourse(qc, course.id)
      toast.success('Course saved')
    } catch (e) {
      handleFormError(e, form.setError)
    }
  })
  const setStatus = useMutation({
    mutationFn: (status: 'draft' | 'published' | 'archived') => coursesApi.update(course.id, { status }),
    onSuccess: (_, status) => {
      invalidateCourse(qc, course.id)
      setConfirm(null)
      toast.success(status === 'published' ? 'Course published' : status === 'archived' ? 'Course archived' : 'Course moved to draft')
    },
    onError: (e) => toast.error(errorMessage(e)),
  })
  const del = useMutation({
    mutationFn: () => coursesApi.remove(course.id),
    onSuccess: () => {
      qc.invalidateQueries({ queryKey: ['courses'] })
      toast.success('Course deleted')
      navigate('/admin/courses')
    },
    onError: (e) => {
      setConfirm(null)
      toast.error(errorMessage(e))
    },
  })
  const cover = useMutation({
    mutationFn: (f: File) => coursesApi.uploadCover(course.id, f),
    onSuccess: () => {
      invalidateCourse(qc, course.id)
      toast.success('Cover image updated')
    },
    onError: (e) => toast.error(errorMessage(e)),
  })
  const { errors, isSubmitting, isDirty } = form.formState

  return (
    <div className="grid gap-6 lg:grid-cols-3">
      <Card className="lg:col-span-2">
        <form onSubmit={onSubmit}>
          <CardHeader>
            <CardTitle>Course details</CardTitle>
          </CardHeader>
          <CardContent className="space-y-4">
            <div className="grid gap-4 sm:grid-cols-2">
              <Field label="Title" htmlFor="cd-title" error={errors.title?.message} required>
                <Input id="cd-title" {...form.register('title')} />
              </Field>
              <Field label="Slug" htmlFor="cd-slug" error={errors.slug?.message}>
                <Input id="cd-slug" {...form.register('slug')} />
              </Field>
            </div>
            <Field label="Description" error={errors.description?.message}>
              <Controller control={form.control} name="description" render={({ field }) => <MarkdownEditor value={field.value ?? ''} onChange={field.onChange} minRows={6} />} />
            </Field>
            <div className="grid gap-4 sm:grid-cols-3">
              <Field label="Duration (weeks)" htmlFor="cd-weeks" error={errors.duration_weeks?.message}>
                <Input id="cd-weeks" type="number" {...form.register('duration_weeks')} />
              </Field>
              <Field label="Days per week" htmlFor="cd-days" error={errors.days_per_week?.message}>
                <Input id="cd-days" type="number" {...form.register('days_per_week')} />
              </Field>
              <Field label="Difficulty" htmlFor="cd-diff">
                <NativeSelect id="cd-diff" {...form.register('difficulty')}>
                  <option value="beginner">Beginner</option>
                  <option value="intermediate">Intermediate</option>
                  <option value="advanced">Advanced</option>
                </NativeSelect>
              </Field>
              <Field label="Start date" htmlFor="cd-start" hint="Week 1 begins on this date.">
                <Input id="cd-start" type="date" {...form.register('start_date')} />
              </Field>
              <Field label="End date" htmlFor="cd-end" error={errors.end_date?.message}>
                <Input id="cd-end" type="date" {...form.register('end_date')} />
              </Field>
            </div>
            <Field label="Learning objectives">
              <Controller control={form.control} name="objectives" render={({ field }) => <ListEditor value={field.value ?? []} onChange={field.onChange} placeholder="Add a learning objective" max={30} />} />
            </Field>
          </CardContent>
          <CardFooter className="justify-end">
            <Button type="submit" loading={isSubmitting} disabled={!isDirty}>
              Save changes
            </Button>
          </CardFooter>
        </form>
      </Card>
      <div className="space-y-6">
        <Card>
          <CardHeader>
            <CardTitle>Publishing</CardTitle>
            <CardDescription>Students only see published courses they are enrolled in.</CardDescription>
          </CardHeader>
          <CardContent className="space-y-3">
            <div className="flex items-center justify-between text-sm">
              Status <PublishBadge status={course.status} />
            </div>
            {course.status !== 'published' && (
              <Button className="w-full" onClick={() => setStatus.mutate('published')} loading={setStatus.isPending}>
                <Send /> Publish course
              </Button>
            )}
            {course.status === 'published' && (
              <Button variant="outline" className="w-full" onClick={() => setStatus.mutate('draft')} loading={setStatus.isPending}>
                Unpublish (back to draft)
              </Button>
            )}
            {course.status !== 'archived' && (
              <Button variant="ghost" className="w-full" onClick={() => setConfirm('archive')}>
                <Archive /> Archive
              </Button>
            )}
            {course.status === 'draft' && (
              <Button variant="ghost" className="w-full text-destructive" onClick={() => setConfirm('delete')}>
                <Trash2 /> Delete course
              </Button>
            )}
          </CardContent>
        </Card>
        <Card className="overflow-hidden">
          <CourseCover course={course} />
          <CardContent className="pt-4">
            <input ref={coverRef} type="file" accept=".png,.jpg,.jpeg,.webp" className="sr-only" onChange={(e) => e.target.files?.[0] && cover.mutate(e.target.files[0])} />
            <Button variant="outline" className="w-full" onClick={() => coverRef.current?.click()} loading={cover.isPending}>
              <ImagePlus /> {course.cover_file_id ? 'Replace cover image' : 'Upload cover image'}
            </Button>
            <p className="mt-2 text-xs text-muted-foreground">PNG, JPEG or WebP, up to 5 MB.</p>
          </CardContent>
        </Card>
      </div>
      <ConfirmDialog
        open={confirm === 'archive'}
        onOpenChange={(o) => !o && setConfirm(null)}
        title="Archive this course?"
        description="Archived courses remain readable for enrolled students but are no longer active."
        confirmLabel="Archive"
        loading={setStatus.isPending}
        onConfirm={() => setStatus.mutate('archived')}
      />
      <ConfirmDialog
        open={confirm === 'delete'}
        onOpenChange={(o) => !o && setConfirm(null)}
        title="Delete this course permanently?"
        description="All weeks, lessons, assignments and resources of this draft course will be deleted. This cannot be undone."
        destructive
        confirmLabel="Delete course"
        loading={del.isPending}
        onConfirm={() => del.mutate()}
      />
    </div>
  )
}

function ModuleDialog({ open, onOpenChange, courseId, module }: { open: boolean; onOpenChange: (o: boolean) => void; courseId: string; module: Module | null }) {
  const qc = useQueryClient()
  const [title, setTitle] = React.useState('')
  const [description, setDescription] = React.useState('')
  React.useEffect(() => {
    if (open) {
      setTitle(module?.title ?? '')
      setDescription(module?.description ?? '')
    }
  }, [open, module])
  const save = useMutation({
    mutationFn: () => (module ? curriculumApi.updateModule(module.id, { title, description }) : curriculumApi.createModule(courseId, { title, description })),
    onSuccess: () => {
      qc.invalidateQueries({ queryKey: ['curriculum', courseId] })
      toast.success('Module saved')
      onOpenChange(false)
    },
    onError: (e) => toast.error(errorMessage(e)),
  })
  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent>
        <DialogHeader>
          <DialogTitle>{module ? 'Edit module' : 'New module'}</DialogTitle>
        </DialogHeader>
        <Field label="Title" htmlFor="m-title" required>
          <Input id="m-title" value={title} onChange={(e) => setTitle(e.target.value)} />
        </Field>
        <Field label="Description" htmlFor="m-desc">
          <Input id="m-desc" value={description} onChange={(e) => setDescription(e.target.value)} />
        </Field>
        <DialogFooter>
          <Button variant="outline" onClick={() => onOpenChange(false)}>
            Cancel
          </Button>
          <Button onClick={() => save.mutate()} loading={save.isPending} disabled={!title.trim()}>
            Save module
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  )
}

function CurriculumTab({ course }: { course: CourseListItem }) {
  const qc = useQueryClient()
  const navigate = useNavigate()
  const q = useQuery({ queryKey: ['curriculum', course.id], queryFn: () => coursesApi.curriculum(course.id) })
  const [moduleDialog, setModuleDialog] = React.useState<{ open: boolean; module: Module | null }>({ open: false, module: null })
  const [deleteModule, setDeleteModule] = React.useState<Module | null>(null)
  const [deleteWeek, setDeleteWeek] = React.useState<WeekSummary | null>(null)
  const [duplicate, setDuplicate] = React.useState<{ week: WeekSummary; target: string } | null>(null)
  const refresh = () => invalidateCourse(qc, course.id)
  const onErr = (e: unknown) => toast.error(errorMessage(e))

  const generate = useMutation({
    mutationFn: () => coursesApi.generateWeeks(course.id, true),
    onSuccess: (r) => {
      refresh()
      toast.success(r.count ? `Created ${r.count} draft weeks` : 'All weeks already exist')
    },
    onError: onErr,
  })
  const addWeek = useMutation({
    mutationFn: () => curriculumApi.createWeek(course.id, { title: 'New week', create_day_lessons: true }),
    onSuccess: (r) => {
      refresh()
      navigate(`/admin/weeks/${r.id}`)
    },
    onError: onErr,
  })
  const patchWeek = useMutation({
    mutationFn: ({ id, body }: { id: string; body: Parameters<typeof curriculumApi.updateWeek>[1] }) => curriculumApi.updateWeek(id, body),
    onSuccess: () => refresh(),
    onError: onErr,
  })
  const delWeek = useMutation({
    mutationFn: (id: string) => curriculumApi.deleteWeek(id),
    onSuccess: () => {
      setDeleteWeek(null)
      refresh()
      toast.success('Week deleted')
    },
    onError: (e) => {
      setDeleteWeek(null)
      onErr(e)
    },
  })
  const delModule = useMutation({
    mutationFn: (id: string) => curriculumApi.deleteModule(id),
    onSuccess: () => {
      setDeleteModule(null)
      refresh()
    },
    onError: onErr,
  })
  const dup = useMutation({
    mutationFn: ({ id, n }: { id: string; n: number }) => curriculumApi.duplicateWeek(id, n),
    onSuccess: (r) => {
      setDuplicate(null)
      refresh()
      toast.success('Week duplicated as a draft')
      navigate(`/admin/weeks/${r.id}`)
    },
    onError: onErr,
  })

  if (q.isLoading) return <PageSkeleton rows={1} />
  if (q.error || !q.data) return <ErrorState error={q.error} onRetry={() => q.refetch()} />
  const { modules, weeks } = q.data
  const missing = course.duration_weeks - weeks.filter((w) => w.week_number <= course.duration_weeks).length

  return (
    <div className="space-y-6">
      <Card>
        <CardHeader className="flex-row items-start justify-between gap-3">
          <div className="space-y-1">
            <CardTitle>Modules</CardTitle>
            <CardDescription>Group weeks into thematic modules.</CardDescription>
          </div>
          <Button variant="outline" size="sm" onClick={() => setModuleDialog({ open: true, module: null })}>
            <Plus /> Add module
          </Button>
        </CardHeader>
        <CardContent>
          {modules.length ? (
            <ul className="divide-y rounded-lg border">
              {modules.map((m, i) => (
                <li key={m.id} className="flex items-center gap-3 px-3 py-2">
                  <span className="w-6 text-xs text-muted-foreground tabular-nums">{i + 1}</span>
                  <div className="min-w-0 flex-1">
                    <div className="truncate text-sm font-medium">{m.title}</div>
                    <div className="truncate text-xs text-muted-foreground">
                      {weeks.filter((w) => w.module_id === m.id).length} weeks{m.description ? ` · ${m.description}` : ''}
                    </div>
                  </div>
                  <Button variant="ghost" size="icon-sm" onClick={() => setModuleDialog({ open: true, module: m })} aria-label={`Edit ${m.title}`}>
                    <Pencil />
                  </Button>
                  <Button variant="ghost" size="icon-sm" onClick={() => setDeleteModule(m)} aria-label={`Delete ${m.title}`}>
                    <Trash2 />
                  </Button>
                </li>
              ))}
            </ul>
          ) : (
            <p className="text-sm text-muted-foreground">No modules yet.</p>
          )}
        </CardContent>
      </Card>

      <Card>
        <CardHeader className="flex-row flex-wrap items-start justify-between gap-3">
          <div className="space-y-1">
            <CardTitle>Weeks</CardTitle>
            <CardDescription>
              {weeks.filter((w) => w.status === 'published').length} of {weeks.length} published · each week has {course.days_per_week} instructional days
            </CardDescription>
          </div>
          <div className="flex flex-wrap gap-2">
            {missing > 0 && (
              <Button variant="outline" size="sm" onClick={() => generate.mutate()} loading={generate.isPending}>
                <Wand2 /> Generate {missing} missing weeks
              </Button>
            )}
            <Button size="sm" onClick={() => addWeek.mutate()} loading={addWeek.isPending}>
              <Plus /> Add week
            </Button>
          </div>
        </CardHeader>
        <CardContent className="px-0">
          {weeks.length === 0 ? (
            <div className="px-5">
              <EmptyState icon={Wand2} title="No weeks yet" description="Generate the full week structure for this course in one step." action={<Button onClick={() => generate.mutate()}>Generate weeks</Button>} />
            </div>
          ) : (
            <Table>
              <TableHeader>
                <TableRow>
                  <TableHead className="w-16 pl-5">Week</TableHead>
                  <TableHead>Title</TableHead>
                  <TableHead className="hidden md:table-cell">Module</TableHead>
                  <TableHead className="hidden sm:table-cell">Lessons</TableHead>
                  <TableHead>Status</TableHead>
                  <TableHead className="pr-5" />
                </TableRow>
              </TableHeader>
              <TableBody>
                {weeks.map((w) => (
                  <TableRow key={w.id}>
                    <TableCell className="pl-5 font-medium tabular-nums">{w.week_number}</TableCell>
                    <TableCell>
                      <Link to={`/admin/weeks/${w.id}`} className="font-medium hover:underline">
                        {w.title}
                      </Link>
                      {w.starts_on && <div className="text-xs text-muted-foreground">Starts {formatPlainDate(w.starts_on)}</div>}
                    </TableCell>
                    <TableCell className="hidden md:table-cell">
                      <NativeSelect
                        className="h-8 w-48 text-xs"
                        value={w.module_id ?? ''}
                        onChange={(e) => patchWeek.mutate({ id: w.id, body: { module_id: e.target.value } })}
                        aria-label={`Module of week ${w.week_number}`}
                      >
                        <option value="">None</option>
                        {modules.map((m) => (
                          <option key={m.id} value={m.id}>
                            {m.title}
                          </option>
                        ))}
                      </NativeSelect>
                    </TableCell>
                    <TableCell className="hidden text-muted-foreground tabular-nums sm:table-cell">
                      {w.published_lesson_count}/{w.lesson_count}
                    </TableCell>
                    <TableCell>
                      <PublishBadge status={w.status} />
                    </TableCell>
                    <TableCell className="pr-5 text-right">
                      <DropdownMenu>
                        <DropdownMenuTrigger asChild>
                          <Button variant="ghost" size="icon-sm" aria-label={`Actions for week ${w.week_number}`}>
                            <MoreHorizontal />
                          </Button>
                        </DropdownMenuTrigger>
                        <DropdownMenuContent align="end">
                          <DropdownMenuItem onSelect={() => navigate(`/admin/weeks/${w.id}`)}>
                            <Pencil /> Edit week
                          </DropdownMenuItem>
                          <DropdownMenuItem onSelect={() => navigate(`/weeks/${w.id}`)}>
                            <Eye /> Preview
                          </DropdownMenuItem>
                          {w.status === 'draft' ? (
                            <DropdownMenuItem onSelect={() => patchWeek.mutate({ id: w.id, body: { status: 'published', cascade_lessons: true } })}>
                              <Send /> Publish week and lessons
                            </DropdownMenuItem>
                          ) : (
                            <DropdownMenuItem onSelect={() => patchWeek.mutate({ id: w.id, body: { status: 'draft' } })}>Unpublish week</DropdownMenuItem>
                          )}
                          <DropdownMenuItem onSelect={() => setDuplicate({ week: w, target: String(Math.max(...weeks.map((x) => x.week_number)) + 1) })}>
                            <Copy /> Duplicate…
                          </DropdownMenuItem>
                          <DropdownMenuSeparator />
                          <DropdownMenuItem destructive onSelect={() => setDeleteWeek(w)}>
                            <Trash2 /> Delete
                          </DropdownMenuItem>
                        </DropdownMenuContent>
                      </DropdownMenu>
                    </TableCell>
                  </TableRow>
                ))}
              </TableBody>
            </Table>
          )}
        </CardContent>
      </Card>

      <ModuleDialog open={moduleDialog.open} onOpenChange={(o) => setModuleDialog((d) => ({ ...d, open: o }))} courseId={course.id} module={moduleDialog.module} />
      <ConfirmDialog
        open={!!deleteModule}
        onOpenChange={(o) => !o && setDeleteModule(null)}
        title={`Delete module “${deleteModule?.title}”?`}
        description="Its weeks are kept and become unassigned."
        destructive
        confirmLabel="Delete module"
        loading={delModule.isPending}
        onConfirm={() => deleteModule && delModule.mutate(deleteModule.id)}
      />
      <ConfirmDialog
        open={!!deleteWeek}
        onOpenChange={(o) => !o && setDeleteWeek(null)}
        title={`Delete week ${deleteWeek?.week_number}?`}
        description="Its lessons and lesson progress are deleted. Weeks with student submissions cannot be deleted."
        destructive
        confirmLabel="Delete week"
        loading={delWeek.isPending}
        onConfirm={() => deleteWeek && delWeek.mutate(deleteWeek.id)}
      />
      <Dialog open={!!duplicate} onOpenChange={(o) => !o && setDuplicate(null)}>
        <DialogContent>
          <DialogHeader>
            <DialogTitle>Duplicate week {duplicate?.week.week_number}</DialogTitle>
          </DialogHeader>
          <p className="text-sm text-muted-foreground">Lessons, resources and assignments are copied as drafts. Assignment dates are cleared.</p>
          <Field label="New week number" htmlFor="dup-n">
            <Input id="dup-n" type="number" min={1} max={104} value={duplicate?.target ?? ''} onChange={(e) => setDuplicate((d) => (d ? { ...d, target: e.target.value } : d))} />
          </Field>
          <DialogFooter>
            <Button variant="outline" onClick={() => setDuplicate(null)}>
              Cancel
            </Button>
            <Button onClick={() => duplicate && dup.mutate({ id: duplicate.week.id, n: Number(duplicate.target) })} loading={dup.isPending}>
              Duplicate
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
    </div>
  )
}

function EnrollmentTab({ course }: { course: CourseListItem }) {
  const qc = useQueryClient()
  const enrolled = useQuery({ queryKey: ['enrollments', course.id], queryFn: () => coursesApi.enrollments(course.id) })
  const students = useQuery({ queryKey: ['students'], queryFn: () => usersApi.list({ role: 'student' }) })
  const [selected, setSelected] = React.useState<string[]>([])
  const [removing, setRemoving] = React.useState<string | null>(null)
  const refresh = () => {
    qc.invalidateQueries({ queryKey: ['enrollments', course.id] })
    qc.invalidateQueries({ queryKey: ['students'] })
    invalidateCourse(qc, course.id)
  }
  const enroll = useMutation({
    mutationFn: () => coursesApi.enroll(course.id, selected),
    onSuccess: () => {
      toast.success(`Enrolled ${selected.length} student${selected.length === 1 ? '' : 's'}`)
      setSelected([])
      refresh()
    },
    onError: (e) => toast.error(errorMessage(e)),
  })
  const unenroll = useMutation({
    mutationFn: (uid: string) => coursesApi.unenroll(course.id, uid),
    onSuccess: () => {
      setRemoving(null)
      refresh()
      toast.success('Student removed from course')
    },
  })
  const enrolledIds = new Set(enrolled.data?.map((e) => e.user_id))
  const available = (students.data ?? []).filter((s) => !enrolledIds.has(s.id) && s.is_active)

  return (
    <div className="grid gap-6 lg:grid-cols-3">
      <Card className="lg:col-span-2">
        <CardHeader>
          <CardTitle>Enrolled students</CardTitle>
          <CardDescription>{enrolled.data?.length ?? 0} students</CardDescription>
        </CardHeader>
        <CardContent className="px-0">
          {enrolled.data?.length ? (
            <Table>
              <TableHeader>
                <TableRow>
                  <TableHead className="pl-5">Name</TableHead>
                  <TableHead>Email</TableHead>
                  <TableHead className="hidden sm:table-cell">Enrolled</TableHead>
                  <TableHead className="pr-5" />
                </TableRow>
              </TableHeader>
              <TableBody>
                {enrolled.data.map((e) => (
                  <TableRow key={e.user_id}>
                    <TableCell className="pl-5 font-medium">
                      {e.name} {!e.is_active && <span className="text-xs text-muted-foreground">(inactive)</span>}
                    </TableCell>
                    <TableCell className="text-muted-foreground">{e.email}</TableCell>
                    <TableCell className="hidden text-muted-foreground sm:table-cell">{formatDate(e.enrolled_at)}</TableCell>
                    <TableCell className="pr-5 text-right">
                      <Button variant="ghost" size="icon-sm" onClick={() => setRemoving(e.user_id)} aria-label={`Remove ${e.name}`}>
                        <UserMinus />
                      </Button>
                    </TableCell>
                  </TableRow>
                ))}
              </TableBody>
            </Table>
          ) : (
            <p className="px-5 text-sm text-muted-foreground">No students enrolled yet.</p>
          )}
        </CardContent>
      </Card>
      <Card>
        <CardHeader>
          <CardTitle>Add students</CardTitle>
          <CardDescription>
            Create accounts in <Link to="/admin/students" className="text-primary hover:underline">Student Management</Link>.
          </CardDescription>
        </CardHeader>
        <CardContent className="space-y-2">
          {available.length ? (
            available.map((s) => (
              <label key={s.id} className="flex cursor-pointer items-center gap-3 rounded-md px-2 py-1.5 hover:bg-accent">
                <Checkbox checked={selected.includes(s.id)} onCheckedChange={(c) => setSelected((sel) => (c ? [...sel, s.id] : sel.filter((x) => x !== s.id)))} />
                <span className="min-w-0">
                  <span className="block truncate text-sm">{s.name}</span>
                  <span className="block truncate text-xs text-muted-foreground">{s.email}</span>
                </span>
              </label>
            ))
          ) : (
            <p className="text-sm text-muted-foreground">All active students are enrolled.</p>
          )}
        </CardContent>
        {available.length > 0 && (
          <CardFooter>
            <Button className="w-full" disabled={!selected.length} onClick={() => enroll.mutate()} loading={enroll.isPending}>
              <UserPlus /> Enroll {selected.length || ''} selected
            </Button>
          </CardFooter>
        )}
      </Card>
      <ConfirmDialog
        open={!!removing}
        onOpenChange={(o) => !o && setRemoving(null)}
        title="Remove student from course?"
        description="Their submissions and progress are kept and restored if they are enrolled again."
        destructive
        confirmLabel="Remove"
        loading={unenroll.isPending}
        onConfirm={() => removing && unenroll.mutate(removing)}
      />
    </div>
  )
}

export function CourseEditorPage() {
  const { id = '' } = useParams()
  const [params, setParams] = useSearchParams()
  const q = useQuery({ queryKey: ['course', id], queryFn: () => coursesApi.get(id) })
  useBreadcrumbs([{ label: 'Course Management', to: '/admin/courses' }, { label: q.data?.title ?? 'Course' }])
  if (q.isLoading) return <PageSkeleton />
  if (q.error || !q.data) return <ErrorState error={q.error} onRetry={() => q.refetch()} />
  const tab = params.get('tab') ?? 'details'
  return (
    <>
      <PageHeader
        eyebrow="Course Management"
        title={q.data.title}
        actions={
          <>
            <PublishBadge status={q.data.status} />
            <Button asChild variant="outline">
              <Link to={`/courses/${q.data.id}`}>
                <Eye /> View as student
              </Link>
            </Button>
          </>
        }
      />
      <Tabs value={tab} onValueChange={(t) => setParams({ tab: t }, { replace: true })} className="space-y-6">
        <TabsList>
          <TabsTrigger value="details">Details</TabsTrigger>
          <TabsTrigger value="curriculum">Curriculum</TabsTrigger>
          <TabsTrigger value="schedule">Schedule</TabsTrigger>
          <TabsTrigger value="enrollment">Enrollment</TabsTrigger>
        </TabsList>
        <TabsContent value="details">
          <DetailsTab course={q.data} />
        </TabsContent>
        <TabsContent value="curriculum">
          <CurriculumTab course={q.data} />
        </TabsContent>
        <TabsContent value="schedule">
          <ScheduleEditor course={q.data} />
        </TabsContent>
        <TabsContent value="enrollment">
          <EnrollmentTab course={q.data} />
        </TabsContent>
      </Tabs>
    </>
  )
}
