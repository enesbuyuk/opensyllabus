import * as React from 'react'
import { usePlatformSettings } from '@/hooks/queries'

/** Centered layout for unauthenticated pages. */
export function AuthShell({ title, description, children }: { title: string; description: string; children: React.ReactNode }) {
  const settings = usePlatformSettings()
  React.useEffect(() => {
    document.title = `${title} · OpenSyllabus`
  }, [title])
  return (
    <div className="relative flex min-h-dvh items-center justify-center overflow-hidden px-4 py-10">
      <div
        aria-hidden
        className="pointer-events-none absolute inset-0 opacity-[0.35] [background-image:linear-gradient(to_right,var(--border)_1px,transparent_1px),linear-gradient(to_bottom,var(--border)_1px,transparent_1px)] [background-size:44px_44px] [mask-image:radial-gradient(ellipse_at_center,black_20%,transparent_70%)]"
      />
      <div className="relative w-full max-w-sm animate-slide-up">
        <div className="mb-8 flex flex-col items-center text-center">
          <img src="/favicon.svg" alt="" className="mb-4 size-10 rounded-lg shadow-sm" />
          <div className="text-xs font-medium tracking-wide text-muted-foreground uppercase">{settings.data?.platform_name ?? 'OpenSyllabus'}</div>
          <h1 className="mt-2 text-xl font-semibold tracking-tight">{title}</h1>
          <p className="mt-1 text-sm text-muted-foreground">{description}</p>
        </div>
        <div className="rounded-xl border bg-card p-6 shadow-sm">{children}</div>
        {settings.data?.support_email && (
          <p className="mt-6 text-center text-xs text-muted-foreground">
            Need help? Contact{' '}
            <a className="text-foreground hover:underline" href={`mailto:${settings.data.support_email}`}>
              {settings.data.support_email}
            </a>
          </p>
        )}
      </div>
    </div>
  )
}
