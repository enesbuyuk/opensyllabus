import { zodResolver } from '@hookform/resolvers/zod'
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query'
import { Laptop, LogOut, Monitor, Moon, ShieldCheck, Sun } from 'lucide-react'
import * as React from 'react'
import { useForm } from 'react-hook-form'
import { toast } from 'sonner'
import { z } from 'zod'
import { authApi, miscApi } from '@/api/endpoints'
import { Field } from '@/components/form'
import { PageHeader } from '@/components/page'
import { Badge } from '@/components/ui/badge'
import { Button } from '@/components/ui/button'
import { Card, CardContent, CardDescription, CardFooter, CardHeader, CardTitle } from '@/components/ui/card'
import { Input, NativeSelect, Textarea } from '@/components/ui/input'
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from '@/components/ui/table'
import { Tabs, TabsContent, TabsList, TabsTrigger } from '@/components/ui/tabs'
import { usePlatformSettings } from '@/hooks/queries'
import { useUser } from '@/lib/auth'
import { useBreadcrumbs } from '@/lib/breadcrumbs'
import { errorMessage, handleFormError } from '@/lib/errors'
import { useTheme } from '@/lib/theme'
import { cn, formatDateTime, relativeTime, TIMEZONES } from '@/lib/utils'
import { passwordSchema } from '@/lib/validation'

const profileSchema = z.object({ name: z.string().trim().min(1, 'Name is required').max(120), bio: z.string().max(1000), timezone: z.string().min(1) })

function ProfileCard() {
  const { user, refresh } = useUser()
  const form = useForm<z.infer<typeof profileSchema>>({ resolver: zodResolver(profileSchema), defaultValues: { name: user.name, bio: user.bio, timezone: user.timezone } })
  const onSubmit = form.handleSubmit(async (v) => {
    try {
      await authApi.updateProfile(v)
      await refresh()
      toast.success('Profile updated')
      form.reset(v)
    } catch (e) {
      handleFormError(e, form.setError)
    }
  })
  const { errors, isSubmitting, isDirty } = form.formState
  return (
    <Card>
      <form onSubmit={onSubmit}>
        <CardHeader>
          <CardTitle>Profile</CardTitle>
          <CardDescription>Your name is visible to your instructor and on submissions.</CardDescription>
        </CardHeader>
        <CardContent className="space-y-4">
          <div className="grid gap-4 sm:grid-cols-2">
            <Field label="Name" htmlFor="p-name" error={errors.name?.message}>
              <Input id="p-name" {...form.register('name')} />
            </Field>
            <Field label="Email" htmlFor="p-email" hint="Contact your instructor to change your email.">
              <Input id="p-email" value={user.email} disabled />
            </Field>
          </div>
          <Field label="Timezone" htmlFor="p-tz" hint="Deadlines and times are displayed in this timezone.">
            <NativeSelect id="p-tz" {...form.register('timezone')}>
              {TIMEZONES.map((tz) => (
                <option key={tz} value={tz}>
                  {tz.replace(/_/g, ' ')}
                </option>
              ))}
            </NativeSelect>
          </Field>
          <Field label="Bio" htmlFor="p-bio" error={errors.bio?.message}>
            <Textarea id="p-bio" rows={3} {...form.register('bio')} />
          </Field>
        </CardContent>
        <CardFooter className="justify-end">
          <Button type="submit" loading={isSubmitting} disabled={!isDirty}>
            Save profile
          </Button>
        </CardFooter>
      </form>
    </Card>
  )
}

const pwSchema = z
  .object({ current_password: z.string().min(1, 'Enter your current password'), new_password: passwordSchema, confirm: z.string() })
  .refine((v) => v.new_password === v.confirm, { message: 'Passwords do not match', path: ['confirm'] })

function PasswordCard() {
  const form = useForm<z.infer<typeof pwSchema>>({ resolver: zodResolver(pwSchema), defaultValues: { current_password: '', new_password: '', confirm: '' } })
  const onSubmit = form.handleSubmit(async (v) => {
    try {
      await authApi.changePassword(v.current_password, v.new_password)
      toast.success('Password changed. Other devices were signed out.')
      form.reset()
    } catch (e) {
      handleFormError(e, form.setError)
    }
  })
  const { errors, isSubmitting } = form.formState
  return (
    <Card>
      <form onSubmit={onSubmit}>
        <CardHeader>
          <CardTitle>Password</CardTitle>
          <CardDescription>At least 10 characters, including a number or symbol.</CardDescription>
        </CardHeader>
        <CardContent className="grid gap-4 sm:grid-cols-3">
          <Field label="Current password" htmlFor="pw-cur" error={errors.current_password?.message}>
            <Input id="pw-cur" type="password" autoComplete="current-password" {...form.register('current_password')} />
          </Field>
          <Field label="New password" htmlFor="pw-new" error={errors.new_password?.message}>
            <Input id="pw-new" type="password" autoComplete="new-password" {...form.register('new_password')} />
          </Field>
          <Field label="Confirm new password" htmlFor="pw-conf" error={errors.confirm?.message}>
            <Input id="pw-conf" type="password" autoComplete="new-password" {...form.register('confirm')} />
          </Field>
        </CardContent>
        <CardFooter className="justify-end">
          <Button type="submit" loading={isSubmitting}>
            Change password
          </Button>
        </CardFooter>
      </form>
    </Card>
  )
}

function SessionsCard() {
  const { timezone } = useUser()
  const qc = useQueryClient()
  const q = useQuery({ queryKey: ['sessions'], queryFn: authApi.sessions })
  const revoke = useMutation({ mutationFn: authApi.revokeSession, onSuccess: () => qc.invalidateQueries({ queryKey: ['sessions'] }) })
  const others = useMutation({
    mutationFn: authApi.revokeOthers,
    onSuccess: () => {
      toast.success('Signed out of all other sessions')
      qc.invalidateQueries({ queryKey: ['sessions'] })
    },
  })
  return (
    <Card>
      <CardHeader className="flex-row items-start justify-between gap-3">
        <div className="space-y-1">
          <CardTitle>Active sessions</CardTitle>
          <CardDescription>Devices currently signed in to your account.</CardDescription>
        </div>
        <Button variant="outline" size="sm" onClick={() => others.mutate()} loading={others.isPending} disabled={(q.data?.length ?? 0) < 2}>
          <LogOut /> Sign out others
        </Button>
      </CardHeader>
      <CardContent className="space-y-2">
        {q.data?.map((s) => (
          <div key={s.id} className="flex items-center gap-3 rounded-lg border px-3 py-2.5">
            <Laptop className="size-4 shrink-0 text-muted-foreground" />
            <div className="min-w-0 flex-1">
              <div className="truncate text-sm">{s.user_agent ?? 'Unknown device'}</div>
              <div className="text-xs text-muted-foreground">
                {s.ip_address ?? 'Unknown IP'} · signed in {formatDateTime(s.created_at, timezone)} · active {relativeTime(s.last_seen_at)}
              </div>
            </div>
            {s.current ? (
              <Badge variant="success">This device</Badge>
            ) : (
              <Button variant="ghost" size="sm" onClick={() => revoke.mutate(s.id)}>
                Revoke
              </Button>
            )}
          </div>
        ))}
      </CardContent>
    </Card>
  )
}

function AppearanceCard() {
  const { theme, setTheme } = useTheme()
  const options = [
    { value: 'dark', label: 'Dark', icon: Moon },
    { value: 'light', label: 'Light', icon: Sun },
    { value: 'system', label: 'System', icon: Monitor },
  ] as const
  return (
    <Card>
      <CardHeader>
        <CardTitle>Appearance</CardTitle>
        <CardDescription>Dark mode is the default. Your choice is saved on this device.</CardDescription>
      </CardHeader>
      <CardContent className="grid grid-cols-3 gap-3">
        {options.map((o) => (
          <button
            key={o.value}
            type="button"
            onClick={() => setTheme(o.value)}
            aria-pressed={theme === o.value}
            className={cn('flex flex-col items-center gap-2 rounded-lg border p-4 text-sm transition hover:bg-accent focus-visible:ring-2 focus-visible:ring-ring/50 focus-visible:outline-none', theme === o.value && 'border-primary bg-primary/5')}
          >
            <o.icon className="size-5" />
            {o.label}
          </button>
        ))}
      </CardContent>
    </Card>
  )
}

const platformSchema = z.object({ platform_name: z.string().trim().min(1).max(80), default_timezone: z.string(), support_email: z.string().trim().refine((v) => !v || /.+@.+\..+/.test(v), 'Enter a valid email') })

function PlatformCard() {
  const settings = usePlatformSettings()
  const qc = useQueryClient()
  const form = useForm<z.infer<typeof platformSchema>>({ resolver: zodResolver(platformSchema) })
  React.useEffect(() => {
    if (settings.data) form.reset({ platform_name: settings.data.platform_name, default_timezone: settings.data.default_timezone, support_email: settings.data.support_email })
  }, [settings.data, form])
  const onSubmit = form.handleSubmit(async (v) => {
    try {
      await miscApi.updateSettings(v)
      qc.invalidateQueries({ queryKey: ['settings'] })
      toast.success('Platform settings saved')
    } catch (e) {
      handleFormError(e, form.setError)
    }
  })
  const { errors, isSubmitting } = form.formState
  return (
    <Card>
      <form onSubmit={onSubmit}>
        <CardHeader>
          <CardTitle>Platform</CardTitle>
          <CardDescription>The default timezone determines when weeks open and is used for new accounts.</CardDescription>
        </CardHeader>
        <CardContent className="grid gap-4 sm:grid-cols-3">
          <Field label="Platform name" htmlFor="pl-name" error={errors.platform_name?.message}>
            <Input id="pl-name" {...form.register('platform_name')} />
          </Field>
          <Field label="Default timezone" htmlFor="pl-tz">
            <NativeSelect id="pl-tz" {...form.register('default_timezone')}>
              {TIMEZONES.map((tz) => (
                <option key={tz} value={tz}>
                  {tz}
                </option>
              ))}
            </NativeSelect>
          </Field>
          <Field label="Support email" htmlFor="pl-email" error={errors.support_email?.message} hint="Shown on the sign-in page.">
            <Input id="pl-email" type="email" {...form.register('support_email')} />
          </Field>
        </CardContent>
        <CardFooter className="justify-between">
          <span className="text-xs text-muted-foreground">Email notifications are not configured; all notifications are in-app.</span>
          <Button type="submit" loading={isSubmitting}>
            Save settings
          </Button>
        </CardFooter>
      </form>
    </Card>
  )
}

function AuditCard() {
  const { timezone } = useUser()
  const q = useQuery({ queryKey: ['audit'], queryFn: miscApi.auditLogs })
  return (
    <Card>
      <CardHeader>
        <CardTitle className="flex items-center gap-2">
          <ShieldCheck className="size-4 text-primary" /> Audit log
        </CardTitle>
        <CardDescription>Administrative and security-relevant operations (latest 200).</CardDescription>
      </CardHeader>
      <CardContent className="px-0">
        {q.error ? (
          <p className="px-5 text-sm text-destructive">{errorMessage(q.error)}</p>
        ) : (
          <div className="max-h-[32rem] overflow-y-auto">
            <Table>
              <TableHeader>
                <TableRow>
                  <TableHead className="pl-5">When</TableHead>
                  <TableHead>Actor</TableHead>
                  <TableHead>Action</TableHead>
                  <TableHead className="pr-5">IP</TableHead>
                </TableRow>
              </TableHeader>
              <TableBody>
                {q.data?.map((l) => (
                  <TableRow key={l.id}>
                    <TableCell className="pl-5 whitespace-nowrap text-muted-foreground">{formatDateTime(l.created_at, timezone)}</TableCell>
                    <TableCell>{l.actor_name ?? '—'}</TableCell>
                    <TableCell>
                      <code className="rounded bg-muted px-1.5 py-0.5 font-mono text-xs">{l.action}</code>
                    </TableCell>
                    <TableCell className="pr-5 font-mono text-xs text-muted-foreground">{l.ip_address ?? '—'}</TableCell>
                  </TableRow>
                ))}
              </TableBody>
            </Table>
          </div>
        )}
      </CardContent>
    </Card>
  )
}

export function SettingsPage() {
  useBreadcrumbs([{ label: 'Settings' }])
  const { isAdmin } = useUser()
  return (
    <>
      <PageHeader title="Settings" description="Manage your profile, security and preferences." />
      <Tabs defaultValue="account" className="space-y-6">
        <TabsList>
          <TabsTrigger value="account">Account</TabsTrigger>
          <TabsTrigger value="security">Security</TabsTrigger>
          <TabsTrigger value="appearance">Appearance</TabsTrigger>
          {isAdmin && <TabsTrigger value="platform">Platform</TabsTrigger>}
        </TabsList>
        <TabsContent value="account" className="space-y-6">
          <ProfileCard />
        </TabsContent>
        <TabsContent value="security" className="space-y-6">
          <PasswordCard />
          <SessionsCard />
        </TabsContent>
        <TabsContent value="appearance">
          <AppearanceCard />
        </TabsContent>
        {isAdmin && (
          <TabsContent value="platform" className="space-y-6">
            <PlatformCard />
            <AuditCard />
          </TabsContent>
        )}
      </Tabs>
    </>
  )
}
