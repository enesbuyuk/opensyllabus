import { zodResolver } from '@hookform/resolvers/zod'
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query'
import { Megaphone, MoreHorizontal, Pencil, Pin, PinOff, Plus, Send, Trash2 } from 'lucide-react'
import * as React from 'react'
import { Controller, useForm } from 'react-hook-form'
import { useLocation } from 'react-router'
import { toast } from 'sonner'
import { z } from 'zod'
import { announcementsApi } from '@/api/endpoints'
import { CourseSelect } from '@/components/course-select'
import { Field } from '@/components/form'
import { Markdown } from '@/components/markdown'
import { MarkdownEditor } from '@/components/markdown-editor'
import { EmptyState, ErrorState, PageHeader } from '@/components/page'
import { PublishBadge } from '@/components/status'
import { ConfirmDialog } from '@/components/ui/alert-dialog'
import { Badge } from '@/components/ui/badge'
import { Button } from '@/components/ui/button'
import { Dialog, DialogContent, DialogFooter, DialogHeader, DialogTitle } from '@/components/ui/dialog'
import { DropdownMenu, DropdownMenuContent, DropdownMenuItem, DropdownMenuTrigger } from '@/components/ui/dropdown-menu'
import { Input, NativeSelect } from '@/components/ui/input'
import { Avatar, Skeleton, Switch } from '@/components/ui/misc'
import { Label } from '@/components/ui/label'
import { useCourses } from '@/hooks/queries'
import { useAuth } from '@/lib/auth'
import { useBreadcrumbs } from '@/lib/breadcrumbs'
import { errorMessage, handleFormError } from '@/lib/errors'
import { cn, formatDateTime, relativeTime } from '@/lib/utils'
import type { Announcement } from '@/types/api'

const schema = z.object({
  course_id: z.string().min(1, 'Select a course'),
  title: z.string().trim().min(1, 'Title is required').max(200),
  body_md: z.string().max(50_000),
  pinned: z.boolean(),
})
type FormValues = z.infer<typeof schema>

function AnnouncementDialog({ open, onOpenChange, item, defaultCourse }: { open: boolean; onOpenChange: (o: boolean) => void; item: Announcement | null; defaultCourse?: string }) {
  const courses = useCourses()
  const qc = useQueryClient()
  const form = useForm<FormValues>({ resolver: zodResolver(schema) })
  React.useEffect(() => {
    if (open) form.reset({ course_id: item?.course_id ?? defaultCourse ?? courses.data?.[0]?.id ?? '', title: item?.title ?? '', body_md: item?.body_md ?? '', pinned: item?.pinned ?? false })
  }, [open, item, defaultCourse, courses.data, form])

  const save = (status: 'draft' | 'published') =>
    form.handleSubmit(async (v) => {
      try {
        if (item) await announcementsApi.update(item.id, { ...v, status: item.status === 'published' ? 'published' : status })
        else await announcementsApi.create({ ...v, status })
        toast.success(status === 'published' && item?.status !== 'published' ? 'Announcement published' : 'Announcement saved')
        qc.invalidateQueries({ queryKey: ['announcements'] })
        qc.invalidateQueries({ queryKey: ['dashboard'] })
        onOpenChange(false)
      } catch (e) {
        handleFormError(e, form.setError)
      }
    })()
  const { errors, isSubmitting } = form.formState

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent wide>
        <DialogHeader>
          <DialogTitle>{item ? 'Edit announcement' : 'New announcement'}</DialogTitle>
        </DialogHeader>
        <form onSubmit={(e) => e.preventDefault()} className="space-y-4">
          <div className="grid gap-4 sm:grid-cols-[1fr_16rem]">
            <Field label="Title" htmlFor="an-title" error={errors.title?.message} required>
              <Input id="an-title" {...form.register('title')} />
            </Field>
            <Field label="Course" htmlFor="an-course" error={errors.course_id?.message}>
              <NativeSelect id="an-course" disabled={!!item} {...form.register('course_id')}>
                {courses.data?.map((c) => (
                  <option key={c.id} value={c.id}>
                    {c.title}
                  </option>
                ))}
              </NativeSelect>
            </Field>
          </div>
          <Field label="Message">
            <Controller control={form.control} name="body_md" render={({ field }) => <MarkdownEditor value={field.value} onChange={field.onChange} minRows={10} />} />
          </Field>
          <Controller
            control={form.control}
            name="pinned"
            render={({ field }) => (
              <Label className="font-normal">
                <Switch checked={field.value} onCheckedChange={field.onChange} /> Pin to the top of the list
              </Label>
            )}
          />
          <DialogFooter>
            <Button type="button" variant="outline" onClick={() => onOpenChange(false)}>
              Cancel
            </Button>
            {(!item || item.status === 'draft') && (
              <Button type="button" variant="secondary" onClick={() => save('draft')} disabled={isSubmitting}>
                Save draft
              </Button>
            )}
            <Button type="button" onClick={() => save('published')} loading={isSubmitting}>
              <Send /> {item?.status === 'published' ? 'Save changes' : 'Publish'}
            </Button>
          </DialogFooter>
        </form>
      </DialogContent>
    </Dialog>
  )
}

export function AnnouncementsPage() {
  useBreadcrumbs([{ label: 'Announcements' }])
  const { isAdmin, timezone } = useAuth()
  const location = useLocation()
  const courses = useCourses()
  const qc = useQueryClient()
  const [courseId, setCourseId] = React.useState('')
  const [dialog, setDialog] = React.useState<{ open: boolean; item: Announcement | null }>({ open: false, item: null })
  const [deleting, setDeleting] = React.useState<Announcement | null>(null)
  const q = useQuery({ queryKey: ['announcements', courseId], queryFn: () => announcementsApi.list({ course_id: courseId || undefined }) })
  const invalidate = () => qc.invalidateQueries({ queryKey: ['announcements'] })
  const patch = useMutation({
    mutationFn: ({ id, body }: { id: string; body: Parameters<typeof announcementsApi.update>[1] }) => announcementsApi.update(id, body),
    onSuccess: invalidate,
    onError: (e) => toast.error(errorMessage(e)),
  })
  const del = useMutation({
    mutationFn: (id: string) => announcementsApi.remove(id),
    onSuccess: () => {
      setDeleting(null)
      toast.success('Announcement deleted')
      invalidate()
    },
  })

  React.useEffect(() => {
    if (location.hash && q.data) document.getElementById(location.hash.slice(1))?.scrollIntoView({ block: 'start' })
  }, [location.hash, q.data])

  return (
    <>
      <PageHeader
        title="Announcements"
        description="Course news and updates from your instructor."
        actions={
          <>
            <CourseSelect courses={courses.data ?? []} value={courseId} onChange={setCourseId} allowAll />
            {isAdmin && (
              <Button onClick={() => setDialog({ open: true, item: null })}>
                <Plus /> New announcement
              </Button>
            )}
          </>
        }
      />
      {q.isLoading ? (
        <div className="space-y-4">
          <Skeleton className="h-40 rounded-xl" />
          <Skeleton className="h-40 rounded-xl" />
        </div>
      ) : q.error ? (
        <ErrorState error={q.error} onRetry={() => q.refetch()} />
      ) : !q.data?.length ? (
        <EmptyState icon={Megaphone} title="No announcements yet" description={isAdmin ? 'Publish an announcement to notify enrolled students.' : 'Announcements from your instructor will appear here.'} />
      ) : (
        <div className="mx-auto max-w-3xl space-y-4">
          {q.data.map((a) => (
            <article key={a.id} id={a.id} className={cn('scroll-mt-20 rounded-xl border bg-card p-5 shadow-xs', a.pinned && 'border-primary/30', location.hash === `#${a.id}` && 'ring-2 ring-primary/30')}>
              <header className="mb-3 flex items-start gap-3">
                <Avatar name={a.author_name ?? 'Instructor'} />
                <div className="min-w-0 flex-1">
                  <div className="flex flex-wrap items-center gap-2">
                    <h2 className="font-semibold">{a.title}</h2>
                    {a.pinned && (
                      <Badge>
                        <Pin /> Pinned
                      </Badge>
                    )}
                    {isAdmin && <PublishBadge status={a.status} />}
                  </div>
                  <div className="text-xs text-muted-foreground">
                    {a.author_name ?? 'Instructor'} · {a.course_title} ·{' '}
                    <time dateTime={a.published_at ?? a.created_at} title={formatDateTime(a.published_at ?? a.created_at, timezone)}>
                      {relativeTime(a.published_at ?? a.created_at)}
                    </time>
                  </div>
                </div>
                {isAdmin && (
                  <DropdownMenu>
                    <DropdownMenuTrigger asChild>
                      <Button variant="ghost" size="icon-sm" aria-label="Announcement actions">
                        <MoreHorizontal />
                      </Button>
                    </DropdownMenuTrigger>
                    <DropdownMenuContent align="end">
                      <DropdownMenuItem onSelect={() => setDialog({ open: true, item: a })}>
                        <Pencil /> Edit
                      </DropdownMenuItem>
                      <DropdownMenuItem onSelect={() => patch.mutate({ id: a.id, body: { pinned: !a.pinned } })}>
                        {a.pinned ? <PinOff /> : <Pin />} {a.pinned ? 'Unpin' : 'Pin'}
                      </DropdownMenuItem>
                      {a.status === 'draft' && (
                        <DropdownMenuItem onSelect={() => patch.mutate({ id: a.id, body: { status: 'published' } })}>
                          <Send /> Publish
                        </DropdownMenuItem>
                      )}
                      <DropdownMenuItem destructive onSelect={() => setDeleting(a)}>
                        <Trash2 /> Delete
                      </DropdownMenuItem>
                    </DropdownMenuContent>
                  </DropdownMenu>
                )}
              </header>
              <Markdown>{a.body_md}</Markdown>
            </article>
          ))}
        </div>
      )}
      {isAdmin && (
        <>
          <AnnouncementDialog open={dialog.open} onOpenChange={(o) => setDialog((d) => ({ ...d, open: o }))} item={dialog.item} defaultCourse={courseId || undefined} />
          <ConfirmDialog open={!!deleting} onOpenChange={(o) => !o && setDeleting(null)} title="Delete this announcement?" destructive confirmLabel="Delete" loading={del.isPending} onConfirm={() => deleting && del.mutate(deleting.id)} />
        </>
      )}
    </>
  )
}
