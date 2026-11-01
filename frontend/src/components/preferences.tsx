import { Check, Globe, Moon, Sun } from 'lucide-react'
import { DropdownMenu, DropdownMenuContent, DropdownMenuItem, DropdownMenuTrigger } from '@/components/ui/dropdown-menu'
import { LOCALES, useLocale } from '@/lib/i18n'
import { useTheme } from '@/lib/theme'
import { cn } from '@/lib/utils'

/** Sun/moon pill that switches between light and dark mode. */
export function ThemeToggle({ className }: { className?: string }) {
  const { theme, setTheme } = useTheme()
  const { t } = useLocale()
  const isDark = theme === 'dark' || (theme === 'system' && document.documentElement.classList.contains('dark'))
  const options = [
    { dark: false, label: t('prefs.light'), icon: Sun },
    { dark: true, label: t('prefs.dark'), icon: Moon },
  ]
  return (
    <div role="group" aria-label={t('prefs.theme')} className={cn('flex items-center gap-0.5 rounded-full border bg-card p-1', className)}>
      {options.map((o) => {
        const active = o.dark === isDark
        return (
          <button
            key={String(o.dark)}
            type="button"
            onClick={() => setTheme(o.dark ? 'dark' : 'light')}
            aria-label={o.label}
            aria-pressed={active}
            title={o.label}
            className={cn(
              'flex size-8 items-center justify-center rounded-full transition-colors outline-none focus-visible:ring-2 focus-visible:ring-ring/50 [&_svg]:size-4',
              active ? 'bg-primary text-primary-foreground' : 'text-muted-foreground hover:text-foreground',
            )}
          >
            <o.icon />
          </button>
        )
      })}
    </div>
  )
}

/** Globe button with a menu of available interface languages. */
export function LanguageSwitcher({ className }: { className?: string }) {
  const { locale, setLocale, t } = useLocale()
  const current = LOCALES.find((l) => l.code === locale) ?? LOCALES[0]
  return (
    <DropdownMenu>
      <DropdownMenuTrigger asChild>
        <button
          type="button"
          aria-label={`${t('prefs.language')}: ${current.label}`}
          className={cn(
            'flex h-10 items-center gap-2 rounded-full border bg-card px-3.5 text-sm font-semibold text-muted-foreground transition-colors outline-none hover:text-foreground focus-visible:ring-2 focus-visible:ring-ring/50',
            className,
          )}
        >
          <Globe className="size-4" />
          {current.short}
        </button>
      </DropdownMenuTrigger>
      <DropdownMenuContent align="end" className="w-40">
        {LOCALES.map((l) => (
          <DropdownMenuItem key={l.code} onSelect={() => setLocale(l.code)} lang={l.code}>
            {l.label}
            {l.code === locale && <Check className="ml-auto text-primary" />}
          </DropdownMenuItem>
        ))}
      </DropdownMenuContent>
    </DropdownMenu>
  )
}
