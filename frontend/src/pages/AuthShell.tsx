import * as React from 'react'
import { BookOpen, ClipboardCheck, TrendingUp, type LucideIcon } from 'lucide-react'
import { LanguageSwitcher, ThemeToggle } from '@/components/preferences'
import { usePlatformSettings } from '@/hooks/queries'
import { useT, type MessageKey } from '@/lib/i18n'

const HIGHLIGHTS: { icon: LucideIcon; title: MessageKey; text: MessageKey }[] = [
  { icon: BookOpen, title: 'auth.brand.h1.title', text: 'auth.brand.h1.text' },
  { icon: ClipboardCheck, title: 'auth.brand.h2.title', text: 'auth.brand.h2.text' },
  { icon: TrendingUp, title: 'auth.brand.h3.title', text: 'auth.brand.h3.text' },
]

/** Split layout for unauthenticated pages: brand panel on the left, form on the right. */
export function AuthShell({ title, description, children }: { title: string; description: string; children: React.ReactNode }) {
  const settings = usePlatformSettings()
  const t = useT()
  const name = settings.data?.platform_name ?? 'OpenSyllabus'
  React.useEffect(() => {
    document.title = `${title} · ${name}`
  }, [title, name])

  return (
    <div className="grid min-h-dvh lg:grid-cols-[1.1fr_1fr]">
      {/* Brand panel (large screens) */}
      <aside className="relative hidden overflow-hidden border-r bg-sidebar lg:flex lg:flex-col lg:justify-between lg:p-12 xl:p-16">
        <div
          aria-hidden
          className="pointer-events-none absolute inset-0 opacity-40 [background-image:linear-gradient(to_right,var(--border)_1px,transparent_1px),linear-gradient(to_bottom,var(--border)_1px,transparent_1px)] [background-size:44px_44px] [mask-image:radial-gradient(ellipse_at_top_left,black_10%,transparent_70%)]"
        />
        <div
          aria-hidden
          className="pointer-events-none absolute -bottom-40 -left-32 size-[34rem] rounded-full bg-primary/15 blur-3xl"
        />

        <div className="relative flex items-center gap-3">
          <img src="/favicon.svg" alt="" className="size-9 rounded-lg shadow-sm" />
          <span className="text-lg font-semibold tracking-tight">{name}</span>
        </div>

        <div className="relative max-w-md animate-slide-up">
          <h2 className="text-4xl leading-tight font-semibold tracking-tight text-balance xl:text-5xl">
            {t('auth.brand.heading')} <span className="text-primary">{t('auth.brand.headingAccent')}</span>
          </h2>
          <p className="mt-4 text-base text-muted-foreground">{t('auth.brand.subtitle')}</p>
          <ul className="mt-10 space-y-6">
            {HIGHLIGHTS.map(({ icon: Icon, title, text }) => (
              <li key={title} className="flex gap-4">
                <span className="flex size-10 shrink-0 items-center justify-center rounded-lg border bg-card text-primary shadow-sm">
                  <Icon className="size-5" aria-hidden />
                </span>
                <div>
                  <div className="font-medium">{t(title)}</div>
                  <div className="text-sm text-muted-foreground">{t(text)}</div>
                </div>
              </li>
            ))}
          </ul>
        </div>

        <p className="relative text-xs text-muted-foreground">
          {name === 'OpenSyllabus' ? t('auth.brand.footer') : t('auth.brand.poweredBy')}
        </p>
      </aside>

      {/* Form panel */}
      <main className="relative flex items-center justify-center px-4 pt-24 pb-10 sm:px-8 lg:pt-10">
        <div className="absolute top-5 right-4 z-10 flex items-center gap-2 sm:top-6 sm:right-8">
          <LanguageSwitcher />
          <ThemeToggle />
        </div>
        <div
          aria-hidden
          className="pointer-events-none absolute inset-0 opacity-[0.35] [background-image:linear-gradient(to_right,var(--border)_1px,transparent_1px),linear-gradient(to_bottom,var(--border)_1px,transparent_1px)] [background-size:44px_44px] [mask-image:radial-gradient(ellipse_at_center,black_20%,transparent_70%)] lg:hidden"
        />
        <div className="relative w-full max-w-sm animate-slide-up">
          <div className="mb-8 flex items-center gap-3 lg:hidden">
            <img src="/favicon.svg" alt="" className="size-9 rounded-lg shadow-sm" />
            <span className="text-lg font-semibold tracking-tight">{name}</span>
          </div>
          <div className="mb-6">
            <h1 className="text-2xl font-semibold tracking-tight">{title}</h1>
            <p className="mt-1.5 text-sm text-muted-foreground">{description}</p>
          </div>
          <div className="rounded-xl border bg-card p-6 shadow-sm">{children}</div>
          {settings.data?.support_email && (
            <p className="mt-6 text-center text-xs text-muted-foreground">
              {t('auth.help')}{' '}
              <a className="text-foreground hover:underline" href={`mailto:${settings.data.support_email}`}>
                {settings.data.support_email}
              </a>
            </p>
          )}
        </div>
      </main>
    </div>
  )
}
