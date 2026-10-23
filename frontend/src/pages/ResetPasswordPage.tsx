import { zodResolver } from '@hookform/resolvers/zod'
import { CheckCircle2 } from 'lucide-react'
import * as React from 'react'
import { useForm } from 'react-hook-form'
import { Link, useSearchParams } from 'react-router'
import { z } from 'zod'
import { authApi } from '@/api/endpoints'
import { Field } from '@/components/form'
import { Button } from '@/components/ui/button'
import { Input } from '@/components/ui/input'
import { errorMessage } from '@/lib/errors'
import { passwordSchema } from '@/lib/validation'
import { AuthShell } from './AuthShell'

const schema = z
  .object({ password: passwordSchema, confirm: z.string() })
  .refine((v) => v.password === v.confirm, { message: 'Passwords do not match', path: ['confirm'] })

export function ResetPasswordPage() {
  const [params] = useSearchParams()
  const token = params.get('token') ?? ''
  const setup = params.get('setup') === '1'
  const [done, setDone] = React.useState(false)
  const form = useForm<z.infer<typeof schema>>({ resolver: zodResolver(schema), defaultValues: { password: '', confirm: '' } })
  const { errors, isSubmitting } = form.formState

  const onSubmit = form.handleSubmit(async (v) => {
    try {
      await authApi.resetPassword(token, v.password)
      setDone(true)
    } catch (e) {
      form.setError('root', { message: errorMessage(e) })
    }
  })

  const title = setup ? 'Set your password' : 'Reset your password'
  if (!token) {
    return (
      <AuthShell title={title} description="This link is incomplete.">
        <p className="text-sm text-muted-foreground">Ask your instructor for a new link.</p>
      </AuthShell>
    )
  }
  if (done) {
    return (
      <AuthShell title="Password updated" description="You can now sign in with your new password.">
        <div className="flex flex-col items-center gap-4 text-center">
          <CheckCircle2 className="size-8 text-success" />
          <Button asChild className="w-full">
            <Link to="/login">Continue to sign in</Link>
          </Button>
        </div>
      </AuthShell>
    )
  }
  return (
    <AuthShell title={title} description="Choose a strong password of at least 10 characters.">
      <form onSubmit={onSubmit} className="space-y-4" noValidate>
        {errors.root && (
          <div role="alert" className="rounded-md border border-destructive/30 bg-destructive/10 px-3 py-2 text-[13px] text-destructive">
            {errors.root.message}
          </div>
        )}
        <Field label="New password" htmlFor="password" error={errors.password?.message}>
          <Input id="password" type="password" autoComplete="new-password" autoFocus aria-invalid={!!errors.password} {...form.register('password')} />
        </Field>
        <Field label="Confirm password" htmlFor="confirm" error={errors.confirm?.message}>
          <Input id="confirm" type="password" autoComplete="new-password" aria-invalid={!!errors.confirm} {...form.register('confirm')} />
        </Field>
        <Button type="submit" className="w-full" loading={isSubmitting}>
          {setup ? 'Set password' : 'Update password'}
        </Button>
      </form>
    </AuthShell>
  )
}
