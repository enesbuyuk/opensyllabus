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
import { useT } from '@/lib/i18n'
import { AuthShell } from './AuthShell'

// Mirrors passwordSchema in lib/validation with localized messages.
const makeSchema = (t: ReturnType<typeof useT>) =>
  z
    .object({
      password: z.string().min(10, t('password.min')).max(128, t('password.max')).regex(/[^A-Za-z]/, t('password.symbol')),
      confirm: z.string(),
    })
    .refine((v) => v.password === v.confirm, { message: t('reset.mismatch'), path: ['confirm'] })
type FormValues = z.infer<ReturnType<typeof makeSchema>>

export function ResetPasswordPage() {
  const [params] = useSearchParams()
  const token = params.get('token') ?? ''
  const setup = params.get('setup') === '1'
  const [done, setDone] = React.useState(false)
  const t = useT()
  const schema = React.useMemo(() => makeSchema(t), [t])
  const form = useForm<FormValues>({ resolver: zodResolver(schema), defaultValues: { password: '', confirm: '' } })
  const { errors, isSubmitting } = form.formState

  const onSubmit = form.handleSubmit(async (v) => {
    try {
      await authApi.resetPassword(token, v.password)
      setDone(true)
    } catch (e) {
      form.setError('root', { message: errorMessage(e) })
    }
  })

  const title = setup ? t('reset.title.setup') : t('reset.title.reset')
  if (!token) {
    return (
      <AuthShell title={title} description={t('reset.incomplete')}>
        <p className="text-sm text-muted-foreground">{t('reset.askNewLink')}</p>
      </AuthShell>
    )
  }
  if (done) {
    return (
      <AuthShell title={t('reset.done.title')} description={t('reset.done.description')}>
        <div className="flex flex-col items-center gap-4 text-center">
          <CheckCircle2 className="size-8 text-success" />
          <Button asChild className="w-full">
            <Link to="/login">{t('reset.done.continue')}</Link>
          </Button>
        </div>
      </AuthShell>
    )
  }
  return (
    <AuthShell title={title} description={t('reset.description')}>
      <form onSubmit={onSubmit} className="space-y-4" noValidate>
        {errors.root && (
          <div role="alert" className="rounded-md border border-destructive/30 bg-destructive/10 px-3 py-2 text-[13px] text-destructive">
            {errors.root.message}
          </div>
        )}
        <Field label={t('reset.new')} htmlFor="password" error={errors.password?.message}>
          <Input id="password" type="password" autoComplete="new-password" autoFocus aria-invalid={!!errors.password} {...form.register('password')} />
        </Field>
        <Field label={t('reset.confirm')} htmlFor="confirm" error={errors.confirm?.message}>
          <Input id="confirm" type="password" autoComplete="new-password" aria-invalid={!!errors.confirm} {...form.register('confirm')} />
        </Field>
        <Button type="submit" className="w-full" loading={isSubmitting}>
          {setup ? t('reset.submit.setup') : t('reset.submit.reset')}
        </Button>
      </form>
    </AuthShell>
  )
}
