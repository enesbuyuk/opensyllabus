import { zodResolver } from '@hookform/resolvers/zod'
import { useQueryClient } from '@tanstack/react-query'
import { ChevronRight, GraduationCap, Plus } from 'lucide-react'
import * as React from 'react'
import { Controller, useForm } from 'react-hook-form'
import { Link, useNavigate } from 'react-router'
import { toast } from 'sonner'
import { z } from 'zod'
import { coursesApi } from '@/api/endpoints'
import { Field } from '@/components/form'
import { EmptyState, ErrorState, PageHeader, PageSkeleton } from '@/components/page'
import { PublishBadge } from '@/components/status'
import { Button } from '@/components/ui/button'
import { Dialog, DialogContent, DialogDescription, DialogFooter, DialogHeader, DialogTitle } from '@/components/ui/dialog'
import { Input, NativeSelect, Textarea } from '@/components/ui/input'
import { Label } from '@/components/ui/label'
import { Checkbox } from '@/components/ui/misc'
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from '@/components/ui/table'
import { useCourses } from '@/hooks/queries'
import { useBreadcrumbs } from '@/lib/breadcrumbs'
import { handleFormError } from '@/lib/errors'
import { formatPlainDate } from '@/lib/utils'

const schema = z.object({
  title: z.string().trim().min(3, 'At least 3 characters').max(200),
  description: z.string().max(20_000),
  difficulty: z.enum(['beginner', 'intermediate', 'advanced']),
  duration_weeks: z.coerce.number<number>().int().min(1).max(104),
  days_per_week: z.coerce.number<number>().int().min(1).max(7),
  start_date: z.string(),
  generate_weeks: z.boolean(),
})
type FormValues = z.infer<typeof schema>

function NewCourseDialog({ open, onOpenChange }: { open: boolean; onOpenChange: (o: boolean) => void }) {
  const qc = useQueryClient()
  const navigate = useNavigate()
  const form = useForm<FormValues>({
    resolver: zodResolver(schema),
    defaultValues: { title: '', description: '', difficulty: 'intermediate', duration_weeks: 12, days_per_week: 3, start_date: '', generate_weeks: true },
  })
  const onSubmit = form.handleSubmit(async (v) => {
    try {
      const r = await coursesApi.create(v)
      qc.invalidateQueries({ queryKey: ['courses'] })
      toast.success('Course created as a draft')
      onOpenChange(false)
      navigate(`/admin/courses/${r.id}`)
    } catch (e) {
      handleFormError(e, form.setError)
    }
  })
  const { errors, isSubmitting } = form.formState
  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent>
        <DialogHeader>
          <DialogTitle>New course</DialogTitle>
          <DialogDescription>Courses start as drafts; publish when ready.</DialogDescription>
        </DialogHeader>
        <form onSubmit={onSubmit} className="space-y-4">
          <Field label="Title" htmlFor="c-title" error={errors.title?.message} required>
            <Input id="c-title" {...form.register('title')} />
          </Field>
          <Field label="Short description" htmlFor="c-desc">
            <Textarea id="c-desc" rows={3} {...form.register('description')} />
          </Field>
          <div className="grid grid-cols-3 gap-3">
            <Field label="Weeks" htmlFor="c-weeks" error={errors.duration_weeks?.message}>
              <Input id="c-weeks" type="number" min={1} max={104} {...form.register('duration_weeks')} />
            </Field>
            <Field label="Days / week" htmlFor="c-days" error={errors.days_per_week?.message}>
              <Input id="c-days" type="number" min={1} max={7} {...form.register('days_per_week')} />
            </Field>
            <Field label="Difficulty" htmlFor="c-diff">
              <NativeSelect id="c-diff" {...form.register('difficulty')}>
                <option value="beginner">Beginner</option>
                <option value="intermediate">Intermediate</option>
                <option value="advanced">Advanced</option>
              </NativeSelect>
            </Field>
          </div>
          <Field label="Start date" htmlFor="c-start" hint="Used to determine the current week.">
            <Input id="c-start" type="date" {...form.register('start_date')} />
          </Field>
          <Controller
            control={form.control}
            name="generate_weeks"
            render={({ field }) => (
              <Label className="font-normal">
                <Checkbox checked={field.value} onCheckedChange={(c) => field.onChange(c === true)} />
                Create empty draft weeks with one lesson per instructional day
              </Label>
            )}
          />
          <DialogFooter>
            <Button type="button" variant="outline" onClick={() => onOpenChange(false)}>
              Cancel
            </Button>
            <Button type="submit" loading={isSubmitting}>
              Create course
            </Button>
          </DialogFooter>
        </form>
      </DialogContent>
    </Dialog>
  )
}

export function CoursesAdminPage() {
  useBreadcrumbs([{ label: 'Course Management' }])
  const q = useCourses()
  const [open, setOpen] = React.useState(false)
  if (q.isLoading) return <PageSkeleton rows={1} />
  if (q.error) return <ErrorState error={q.error} onRetry={() => q.refetch()} />
  return (
    <>
      <PageHeader
        title="Course Management"
        description="Create courses, structure the curriculum and manage enrollment."
        actions={
          <Button onClick={() => setOpen(true)}>
            <Plus /> New course
          </Button>
        }
      />
      {q.data?.length ? (
        <div className="overflow-hidden rounded-xl border bg-card">
          <Table>
            <TableHeader>
              <TableRow>
                <TableHead className="pl-5">Course</TableHead>
                <TableHead>Status</TableHead>
                <TableHead className="hidden md:table-cell">Weeks</TableHead>
                <TableHead className="hidden md:table-cell">Students</TableHead>
                <TableHead className="hidden lg:table-cell">Dates</TableHead>
                <TableHead className="pr-5" />
              </TableRow>
            </TableHeader>
            <TableBody>
              {q.data.map((c) => (
                <TableRow key={c.id}>
                  <TableCell className="pl-5">
                    <Link to={`/admin/courses/${c.id}`} className="font-medium hover:underline">
                      {c.title}
                    </Link>
                    <div className="text-xs text-muted-foreground">/{c.slug}</div>
                  </TableCell>
                  <TableCell>
                    <PublishBadge status={c.status} />
                  </TableCell>
                  <TableCell className="hidden tabular-nums md:table-cell">
                    {c.published_weeks}/{c.total_weeks} published
                  </TableCell>
                  <TableCell className="hidden tabular-nums md:table-cell">{c.enrolled_count}</TableCell>
                  <TableCell className="hidden text-muted-foreground lg:table-cell">
                    {c.start_date ? `${formatPlainDate(c.start_date)} – ${formatPlainDate(c.end_date)}` : 'Not scheduled'}
                  </TableCell>
                  <TableCell className="pr-5 text-right">
                    <Button asChild variant="ghost" size="sm">
                      <Link to={`/admin/courses/${c.id}`}>
                        Manage <ChevronRight />
                      </Link>
                    </Button>
                  </TableCell>
                </TableRow>
              ))}
            </TableBody>
          </Table>
        </div>
      ) : (
        <EmptyState icon={GraduationCap} title="No courses yet" description="Create your first course to start building a curriculum." action={<Button onClick={() => setOpen(true)}>New course</Button>} />
      )}
      <NewCourseDialog open={open} onOpenChange={setOpen} />
    </>
  )
}
