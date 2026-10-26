import { zodResolver } from '@hookform/resolvers/zod'
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query'
import { Check, Copy, KeyRound, LogOut, MoreHorizontal, Pencil, Search, TrendingUp, UserCheck, UserPlus, Users, UserX } from 'lucide-react'
import * as React from 'react'
import { Controller, useForm } from 'react-hook-form'
import { useNavigate, useSearchParams } from 'react-router'
import { toast } from 'sonner'
import { z } from 'zod'
import { usersApi } from '@/api/endpoints'
import { Field } from '@/components/form'
import { EmptyState, ErrorState, PageHeader } from '@/components/page'
import { ConfirmDialog } from '@/components/ui/alert-dialog'
import { Badge } from '@/components/ui/badge'
import { Button } from '@/components/ui/button'
import { Dialog, DialogContent, DialogDescription, DialogFooter, DialogHeader, DialogTitle } from '@/components/ui/dialog'
import { DropdownMenu, DropdownMenuContent, DropdownMenuItem, DropdownMenuSeparator, DropdownMenuTrigger } from '@/components/ui/dropdown-menu'
import { Input } from '@/components/ui/input'
import { Label } from '@/components/ui/label'
import { Avatar, Checkbox, Skeleton } from '@/components/ui/misc'
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from '@/components/ui/table'
import { useCourses, useDebounced } from '@/hooks/queries'
import { useBreadcrumbs } from '@/lib/breadcrumbs'
import { errorMessage, handleFormError } from '@/lib/errors'
import { cn, formatDateTime, relativeTime } from '@/lib/utils'
import type { StudentListItem } from '@/types/api'
import { passwordSchema, usernameSchema } from '@/lib/validation'

function CopyLink({ link, expires }: { link: string; expires?: string }) {
  const [copied, setCopied] = React.useState(false)
  return (
    <div className="space-y-2">
      <div className="flex gap-2">
        <Input readOnly value={link} className="font-mono text-xs" onFocus={(e) => e.target.select()} aria-label="One-time link" />
        <Button
          type="button"
          variant="outline"
          size="icon"
          aria-label="Copy link"
          onClick={async () => {
            try {
              await navigator.clipboard.writeText(link)
              setCopied(true)
              setTimeout(() => setCopied(false), 1500)
            } catch {
              toast.error('Copy failed. Select the link and copy it manually')
            }
          }}
        >
          {copied ? <Check /> : <Copy />}
        </Button>
      </div>
      <p className="text-xs text-muted-foreground">
        Share this one-time link privately with the student{expires ? `. It expires ${formatDateTime(expires)}` : ''}. It is shown only once.
      </p>
    </div>
  )
}

const createSchema = z.object({
  name: z.string().trim().min(1, 'Name is required').max(120),
  email: z.string().trim().email('Enter a valid email address'),
  username: usernameSchema,
  mode: z.enum(['link', 'password']),
  password: z.string(),
  course_ids: z.array(z.string()),
})

function CreateStudentDialog({ open, onOpenChange }: { open: boolean; onOpenChange: (o: boolean) => void }) {
  const qc = useQueryClient()
  const courses = useCourses()
  const [created, setCreated] = React.useState<{ name: string; link: string | null } | null>(null)
  const form = useForm<z.infer<typeof createSchema>>({
    resolver: zodResolver(createSchema.superRefine((v, ctx) => {
      if (v.mode === 'password') {
        const r = passwordSchema.safeParse(v.password)
        if (!r.success) ctx.addIssue({ code: 'custom', path: ['password'], message: r.error.issues[0].message })
      }
    })),
    defaultValues: { name: '', email: '', username: '', mode: 'link', password: '', course_ids: [] },
  })
  React.useEffect(() => {
    if (open) {
      setCreated(null)
      form.reset({ name: '', email: '', username: '', mode: 'link', password: '', course_ids: courses.data?.filter((c) => c.status === 'published').map((c) => c.id).slice(0, 1) ?? [] })
    }
  }, [open, courses.data, form])
  const mode = form.watch('mode')
  const onSubmit = form.handleSubmit(async (v) => {
    try {
      const r = await usersApi.create({ name: v.name, email: v.email, username: v.username || undefined, password: v.mode === 'password' ? v.password : undefined, course_ids: v.course_ids })
      qc.invalidateQueries({ queryKey: ['students'] })
      qc.invalidateQueries({ queryKey: ['courses'] })
      setCreated({ name: r.user.name, link: r.setup_link })
    } catch (e) {
      handleFormError(e, form.setError)
    }
  })
  const { errors, isSubmitting } = form.formState

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent>
        {created ? (
          <>
            <DialogHeader>
              <DialogTitle>Student account created</DialogTitle>
              <DialogDescription>{created.link ? `Send ${created.name} this link to set a password.` : `${created.name} can now sign in with the password you set.`}</DialogDescription>
            </DialogHeader>
            {created.link && <CopyLink link={created.link} />}
            <DialogFooter>
              <Button variant="outline" onClick={() => {
                  form.reset({ ...form.getValues(), name: '', email: '', username: '', password: '' })
                  setCreated(null)
                }}>
                Add another
              </Button>
              <Button onClick={() => onOpenChange(false)}>Done</Button>
            </DialogFooter>
          </>
        ) : (
          <form onSubmit={onSubmit} className="space-y-4">
            <DialogHeader>
              <DialogTitle>Add student</DialogTitle>
              <DialogDescription>Accounts are created by the instructor; there is no public registration.</DialogDescription>
            </DialogHeader>
            <div className="grid gap-4 sm:grid-cols-2">
              <Field label="Full name" htmlFor="s-name" error={errors.name?.message} required>
                <Input id="s-name" autoComplete="off" {...form.register('name')} />
              </Field>
              <Field label="Email" htmlFor="s-email" error={errors.email?.message} required>
                <Input id="s-email" type="email" autoComplete="off" {...form.register('email')} />
              </Field>
            </div>
            <Field label="Username" htmlFor="s-username" error={errors.username?.message} hint="Optional. The student can sign in with it instead of the email.">
              <Input id="s-username" autoComplete="off" autoCapitalize="none" spellCheck={false} {...form.register('username')} />
            </Field>
            <Controller
              control={form.control}
              name="mode"
              render={({ field }) => (
                <fieldset className="space-y-2">
                  <legend className="mb-1.5 text-[13px] font-medium">Initial password</legend>
                  {[
                    { v: 'link', label: 'Generate a one-time setup link (recommended)' },
                    { v: 'password', label: 'Set a password now' },
                  ].map((o) => (
                    <label key={o.v} className="flex cursor-pointer items-center gap-2 text-sm">
                      <input type="radio" className="accent-[var(--primary)]" checked={field.value === o.v} onChange={() => field.onChange(o.v)} />
                      {o.label}
                    </label>
                  ))}
                </fieldset>
              )}
            />
            {mode === 'password' && (
              <Field label="Password" htmlFor="s-pw" error={errors.password?.message}>
                <Input id="s-pw" type="password" autoComplete="new-password" {...form.register('password')} />
              </Field>
            )}
            <Controller
              control={form.control}
              name="course_ids"
              render={({ field }) => (
                <fieldset className="space-y-1.5">
                  <legend className="mb-1.5 text-[13px] font-medium">Enroll in</legend>
                  {courses.data?.map((c) => (
                    <Label key={c.id} className="font-normal">
                      <Checkbox checked={field.value.includes(c.id)} onCheckedChange={(on) => field.onChange(on ? [...field.value, c.id] : field.value.filter((x) => x !== c.id))} />
                      {c.title}
                      {c.status !== 'published' && <Badge variant="secondary">{c.status}</Badge>}
                    </Label>
                  ))}
                </fieldset>
              )}
            />
            <DialogFooter>
              <Button type="button" variant="outline" onClick={() => onOpenChange(false)}>
                Cancel
              </Button>
              <Button type="submit" loading={isSubmitting}>
                Create account
              </Button>
            </DialogFooter>
          </form>
        )}
      </DialogContent>
    </Dialog>
  )
}

const editSchema = z.object({ name: z.string().trim().min(1).max(120), email: z.string().trim().email('Enter a valid email address'), username: usernameSchema })

function EditStudentDialog({ student, onOpenChange }: { student: StudentListItem | null; onOpenChange: (o: boolean) => void }) {
  const qc = useQueryClient()
  const form = useForm<z.infer<typeof editSchema>>({ resolver: zodResolver(editSchema) })
  React.useEffect(() => {
    if (student) form.reset({ name: student.name, email: student.email, username: student.username ?? '' })
  }, [student, form])
  const onSubmit = form.handleSubmit(async (v) => {
    try {
      await usersApi.update(student!.id, v)
      qc.invalidateQueries({ queryKey: ['students'] })
      toast.success('Student updated')
      onOpenChange(false)
    } catch (e) {
      handleFormError(e, form.setError)
    }
  })
  const { errors, isSubmitting } = form.formState
  return (
    <Dialog open={!!student} onOpenChange={onOpenChange}>
      <DialogContent>
        <form onSubmit={onSubmit} className="space-y-4">
          <DialogHeader>
            <DialogTitle>Edit student</DialogTitle>
          </DialogHeader>
          <Field label="Full name" htmlFor="e-name" error={errors.name?.message}>
            <Input id="e-name" {...form.register('name')} />
          </Field>
          <Field label="Email" htmlFor="e-email" error={errors.email?.message}>
            <Input id="e-email" type="email" {...form.register('email')} />
          </Field>
          <Field label="Username" htmlFor="e-username" error={errors.username?.message} hint="Leave empty to remove it.">
            <Input id="e-username" autoComplete="off" autoCapitalize="none" spellCheck={false} {...form.register('username')} />
          </Field>
          <DialogFooter>
            <Button type="button" variant="outline" onClick={() => onOpenChange(false)}>
              Cancel
            </Button>
            <Button type="submit" loading={isSubmitting}>
              Save
            </Button>
          </DialogFooter>
        </form>
      </DialogContent>
    </Dialog>
  )
}

export function StudentsPage() {
  useBreadcrumbs([{ label: 'Student Management' }])
  const qc = useQueryClient()
  const navigate = useNavigate()
  const [params] = useSearchParams()
  const highlight = params.get('highlight')
  const [search, setSearch] = React.useState('')
  const q = useDebounced(search)
  const list = useQuery({ queryKey: ['students', q], queryFn: () => usersApi.list({ role: 'student', q }) })
  const [creating, setCreating] = React.useState(false)
  const [editing, setEditing] = React.useState<StudentListItem | null>(null)
  const [toggle, setToggle] = React.useState<StudentListItem | null>(null)
  const [resetLink, setResetLink] = React.useState<{ name: string; link: string; expires_at: string } | null>(null)
  const onErr = (e: unknown) => toast.error(errorMessage(e))

  const setActive = useMutation({
    mutationFn: (s: StudentListItem) => usersApi.update(s.id, { is_active: !s.is_active }),
    onSuccess: (u) => {
      setToggle(null)
      qc.invalidateQueries({ queryKey: ['students'] })
      toast.success(u.is_active ? 'Account reactivated' : 'Account deactivated and signed out')
    },
    onError: onErr,
  })
  const reset = useMutation({
    mutationFn: (s: StudentListItem) => usersApi.resetLink(s.id).then((r) => ({ ...r, name: s.name })),
    onSuccess: (r) => setResetLink(r),
    onError: onErr,
  })
  const revoke = useMutation({
    mutationFn: (s: StudentListItem) => usersApi.revokeSessions(s.id),
    onSuccess: (r) => toast.success(`Revoked ${r.count} session${r.count === 1 ? '' : 's'}`),
    onError: onErr,
  })

  const students = list.data ?? []
  return (
    <>
      <PageHeader
        title="Student Management"
        description="Create accounts, manage enrollment and help students regain access."
        actions={
          <Button onClick={() => setCreating(true)}>
            <UserPlus /> Add student
          </Button>
        }
      />
      <div className="relative mb-4 max-w-sm">
        <Search className="pointer-events-none absolute top-1/2 left-2.5 size-4 -translate-y-1/2 text-muted-foreground" />
        <Input value={search} onChange={(e) => setSearch(e.target.value)} placeholder="Search by name or email" className="pl-8" aria-label="Search students" />
      </div>
      {list.isLoading ? (
        <Skeleton className="h-64 rounded-xl" />
      ) : list.error ? (
        <ErrorState error={list.error} onRetry={() => list.refetch()} />
      ) : students.length === 0 ? (
        <EmptyState icon={Users} title={search ? 'No matching students' : 'No students yet'} description={search ? undefined : 'Add your first student to get started.'} action={!search && <Button onClick={() => setCreating(true)}>Add student</Button>} />
      ) : (
        <div className="overflow-hidden rounded-xl border bg-card">
          <Table>
            <TableHeader>
              <TableRow>
                <TableHead className="pl-5">Student</TableHead>
                <TableHead className="hidden md:table-cell">Courses</TableHead>
                <TableHead>Status</TableHead>
                <TableHead className="hidden sm:table-cell">Last sign-in</TableHead>
                <TableHead className="pr-5" />
              </TableRow>
            </TableHeader>
            <TableBody>
              {students.map((s) => (
                <TableRow key={s.id} className={cn(highlight === s.id && 'bg-primary/5', !s.is_active && 'opacity-60')}>
                  <TableCell className="pl-5">
                    <div className="flex items-center gap-3">
                      <Avatar name={s.name} />
                      <div className="min-w-0">
                        <div className="truncate font-medium">{s.name}</div>
                        <div className="truncate text-xs text-muted-foreground">
                          {s.email}
                          {s.username && <span> · @{s.username}</span>}
                        </div>
                      </div>
                    </div>
                  </TableCell>
                  <TableCell className="hidden md:table-cell">
                    <div className="flex flex-wrap gap-1">
                      {s.courses.length ? (
                        s.courses.map((c) => (
                          <Badge key={c.id} variant="outline" className="max-w-48 truncate">
                            {c.title}
                          </Badge>
                        ))
                      ) : (
                        <span className="text-xs text-muted-foreground">Not enrolled</span>
                      )}
                    </div>
                  </TableCell>
                  <TableCell>
                    {!s.is_active ? <Badge variant="secondary">Inactive</Badge> : s.must_change_password ? <Badge variant="warning">Setup pending</Badge> : <Badge variant="success">Active</Badge>}
                  </TableCell>
                  <TableCell className="hidden text-muted-foreground sm:table-cell">{s.last_login_at ? relativeTime(s.last_login_at) : 'Never'}</TableCell>
                  <TableCell className="pr-5 text-right">
                    <DropdownMenu>
                      <DropdownMenuTrigger asChild>
                        <Button variant="ghost" size="icon-sm" aria-label={`Actions for ${s.name}`}>
                          <MoreHorizontal />
                        </Button>
                      </DropdownMenuTrigger>
                      <DropdownMenuContent align="end">
                        <DropdownMenuItem onSelect={() => setEditing(s)}>
                          <Pencil /> Edit details
                        </DropdownMenuItem>
                        {s.courses[0] && (
                          <DropdownMenuItem onSelect={() => navigate(`/progress?student=${s.id}&course=${s.courses[0].id}`)}>
                            <TrendingUp /> View progress
                          </DropdownMenuItem>
                        )}
                        <DropdownMenuItem onSelect={() => reset.mutate(s)}>
                          <KeyRound /> Create password reset link
                        </DropdownMenuItem>
                        <DropdownMenuItem onSelect={() => revoke.mutate(s)}>
                          <LogOut /> Sign out everywhere
                        </DropdownMenuItem>
                        <DropdownMenuSeparator />
                        <DropdownMenuItem destructive={s.is_active} onSelect={() => setToggle(s)}>
                          {s.is_active ? <UserX /> : <UserCheck />} {s.is_active ? 'Deactivate' : 'Reactivate'}
                        </DropdownMenuItem>
                      </DropdownMenuContent>
                    </DropdownMenu>
                  </TableCell>
                </TableRow>
              ))}
            </TableBody>
          </Table>
        </div>
      )}
      <CreateStudentDialog open={creating} onOpenChange={setCreating} />
      <EditStudentDialog student={editing} onOpenChange={(o) => !o && setEditing(null)} />
      <ConfirmDialog
        open={!!toggle}
        onOpenChange={(o) => !o && setToggle(null)}
        title={toggle?.is_active ? `Deactivate ${toggle?.name}?` : `Reactivate ${toggle?.name}?`}
        description={toggle?.is_active ? 'They will be signed out and cannot sign in. Their work and grades are kept.' : 'They will be able to sign in again.'}
        destructive={toggle?.is_active}
        confirmLabel={toggle?.is_active ? 'Deactivate' : 'Reactivate'}
        loading={setActive.isPending}
        onConfirm={() => toggle && setActive.mutate(toggle)}
      />
      <Dialog open={!!resetLink} onOpenChange={(o) => !o && setResetLink(null)}>
        <DialogContent>
          <DialogHeader>
            <DialogTitle>Password reset link for {resetLink?.name}</DialogTitle>
            <DialogDescription>Any earlier unused links for this account were invalidated.</DialogDescription>
          </DialogHeader>
          {resetLink && <CopyLink link={resetLink.link} expires={resetLink.expires_at} />}
          <DialogFooter>
            <Button onClick={() => setResetLink(null)}>Done</Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
    </>
  )
}
