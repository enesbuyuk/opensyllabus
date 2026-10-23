import { zodResolver } from '@hookform/resolvers/zod'
import { useForm } from 'react-hook-form'
import { Navigate, useLocation, useNavigate } from 'react-router'
import { z } from 'zod'
import { Field } from '@/components/form'
import { Button } from '@/components/ui/button'
import { Input } from '@/components/ui/input'
import { useAuth } from '@/lib/auth'
import { errorMessage } from '@/lib/errors'
import { AuthShell } from './AuthShell'

const schema = z.object({
  email: z.string().trim().min(1, 'Enter your email address').email('Enter a valid email address'),
  password: z.string().min(1, 'Enter your password'),
})
type FormValues = z.infer<typeof schema>

export function LoginPage() {
  const { user, login, loading } = useAuth()
  const navigate = useNavigate()
  const location = useLocation()
  const from = (location.state as { from?: string } | null)?.from ?? '/'
  const form = useForm<FormValues>({ resolver: zodResolver(schema), defaultValues: { email: '', password: '' } })

  if (!loading && user) return <Navigate to={from} replace />

  const onSubmit = form.handleSubmit(async (v) => {
    try {
      await login(v.email, v.password)
      navigate(from, { replace: true })
    } catch (e) {
      form.setError('root', { message: errorMessage(e) })
    }
  })

  const { errors, isSubmitting } = form.formState
  return (
    <AuthShell title="Sign in" description="Welcome back. Sign in to continue learning.">
      <form onSubmit={onSubmit} className="space-y-4" noValidate>
        {errors.root && (
          <div role="alert" className="rounded-md border border-destructive/30 bg-destructive/10 px-3 py-2 text-[13px] text-destructive">
            {errors.root.message}
          </div>
        )}
        <Field label="Email" htmlFor="email" error={errors.email?.message}>
          <Input id="email" type="email" autoComplete="username" autoFocus aria-invalid={!!errors.email} {...form.register('email')} />
        </Field>
        <Field label="Password" htmlFor="password" error={errors.password?.message}>
          <Input id="password" type="password" autoComplete="current-password" aria-invalid={!!errors.password} {...form.register('password')} />
        </Field>
        <Button type="submit" className="w-full" loading={isSubmitting}>
          Sign in
        </Button>
        <p className="text-center text-xs text-muted-foreground">Forgot your password? Ask your instructor for a reset link.</p>
      </form>
    </AuthShell>
  )
}
