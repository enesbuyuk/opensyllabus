import * as React from 'react'
import { BookOpen, ClipboardCheck, TrendingUp, type LucideIcon } from 'lucide-react'
import { usePlatformSettings } from '@/hooks/queries'

const HIGHLIGHTS: { icon: LucideIcon; title: string; text: string }[] = [
  { icon: BookOpen, title: 'Week-by-week curriculum', text: 'Lessons, labs and resources laid out in the order you learn them.' },
  { icon: ClipboardCheck, title: 'Submissions and feedback', text: 'Hand in your work and get graded reviews from your instructor.' },
  { icon: TrendingUp, title: 'Clear progress', text: 'See what is done, what is next and how you are doing.' },
]

/** Split layout for unauthenticated pages: brand panel on the left, form on the right. */
export function AuthShell({ title, description, children }: { title: string; description: string; children: React.ReactNode }) {
  const settings = usePlatformSettings()
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
            Learn step by step, <span className="text-primary">week by week.</span>
          </h2>
          <p className="mt-4 text-base text-muted-foreground">
            Your course, assignments, feedback and progress in one place.
          </p>
          <ul className="mt-10 space-y-6">
            {HIGHLIGHTS.map(({ icon: Icon, title: t, text }) => (
              <li key={t} className="flex gap-4">
                <span className="flex size-10 shrink-0 items-center justify-center rounded-lg border bg-card text-primary shadow-sm">
                  <Icon className="size-5" aria-hidden />
                </span>
                <div>
                  <div className="font-medium">{t}</div>
                  <div className="text-sm text-muted-foreground">{text}</div>
                </div>
              </li>
            ))}
          </ul>
        </div>

        <p className="relative text-xs text-muted-foreground">
          {name === 'OpenSyllabus' ? 'Open-source learning platform' : 'Powered by OpenSyllabus'}
        </p>
      </aside>

      {/* Form panel */}
      <main className="relative flex items-center justify-center px-4 py-10 sm:px-8">
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
              Need help? Contact{' '}
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
