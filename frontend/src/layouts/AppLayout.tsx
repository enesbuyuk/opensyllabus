import * as React from 'react'
import { ChevronRight, LogOut, Menu, Monitor, Moon, PanelLeftClose, PanelLeftOpen, Search, Settings, Sun } from 'lucide-react'
import { Link, NavLink, Outlet, useLocation, useNavigate } from 'react-router'
import { Button } from '@/components/ui/button'
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuLabel,
  DropdownMenuSeparator,
  DropdownMenuTrigger,
} from '@/components/ui/dropdown-menu'
import { Avatar, Kbd } from '@/components/ui/misc'
import { Sheet, SheetContent } from '@/components/ui/sheet'
import { Tooltip } from '@/components/ui/tooltip'
import { usePlatformSettings } from '@/hooks/queries'
import { useUser } from '@/lib/auth'
import { BreadcrumbProvider, useBreadcrumbState } from '@/lib/breadcrumbs'
import { useTheme } from '@/lib/theme'
import { cn } from '@/lib/utils'
import { CommandSearch } from './command-search'
import { ADMIN_NAV, MAIN_NAV, type NavItem } from './nav'
import { NotificationCenter } from './notifications'

const COLLAPSE_KEY = 'opensyllabus-sidebar-collapsed'

function Logo({ collapsed }: { collapsed?: boolean }) {
  const settings = usePlatformSettings()
  return (
    <Link to="/" className="flex items-center gap-2.5 rounded-md px-1 outline-none focus-visible:ring-2 focus-visible:ring-ring/50">
      <img src="/favicon.svg" alt="" className="size-7" />
      {!collapsed && <span className="truncate text-[17px] font-bold tracking-tight">{settings.data?.platform_name ?? 'OpenSyllabus'}</span>}
    </Link>
  )
}

function NavSection({ items, collapsed, onNavigate }: { items: NavItem[]; collapsed?: boolean; onNavigate?: () => void }) {
  return (
    <ul className="space-y-0.5">
      {items.map((item) => {
        const link = (
          <NavLink
            to={item.to}
            end={item.end}
            onClick={onNavigate}
            className={({ isActive }) =>
              cn(
                'flex h-9 items-center gap-3 rounded-md px-3 text-sm font-medium text-muted-foreground transition-colors outline-none hover:bg-accent hover:text-foreground focus-visible:ring-2 focus-visible:ring-ring/50',
                isActive && 'bg-primary-soft text-foreground [&>svg]:text-primary hover:bg-primary-soft',
                collapsed && 'justify-center px-0',
              )
            }
          >
            <item.icon className="size-[18px] shrink-0" />
            {!collapsed && <span className="truncate">{item.label}</span>}
          </NavLink>
        )
        return <li key={item.to}>{collapsed ? <Tooltip content={item.label} side="right">{link}</Tooltip> : link}</li>
      })}
    </ul>
  )
}

function SidebarBody({ collapsed, onNavigate }: { collapsed?: boolean; onNavigate?: () => void }) {
  const { isAdmin } = useUser()
  return (
    <nav aria-label="Main" className="flex-1 space-y-6 overflow-y-auto px-3 py-4">
      <NavSection items={MAIN_NAV} collapsed={collapsed} onNavigate={onNavigate} />
      {isAdmin && (
        <div className="space-y-1">
          {!collapsed && <div className="px-2.5 pb-1 text-xs font-semibold tracking-wide text-muted-foreground/80 uppercase">Management</div>}
          {collapsed && <div className="mx-auto mb-2 h-px w-6 bg-border" />}
          <NavSection items={ADMIN_NAV} collapsed={collapsed} onNavigate={onNavigate} />
        </div>
      )}
    </nav>
  )
}

function Breadcrumbs() {
  const crumbs = useBreadcrumbState()
  if (crumbs.length < 2) return null
  return (
    <nav aria-label="Breadcrumb" className="mb-2 min-w-0">
      <ol className="flex flex-wrap items-center gap-1 text-[13px] text-muted-foreground">
        {crumbs.map((c, i) => (
          <li key={i} className="flex min-w-0 items-center gap-1">
            {i > 0 && <ChevronRight className="size-3.5 shrink-0 opacity-60" />}
            {c.to && i < crumbs.length - 1 ? (
              <Link to={c.to} className="truncate hover:text-foreground">
                {c.label}
              </Link>
            ) : (
              <span className={cn('truncate', i === crumbs.length - 1 && 'text-foreground')} aria-current={i === crumbs.length - 1 ? 'page' : undefined}>
                {c.label}
              </span>
            )}
          </li>
        ))}
      </ol>
    </nav>
  )
}

function ThemeMenuItems() {
  const { theme, setTheme } = useTheme()
  const options = [
    { value: 'dark', label: 'Dark', icon: Moon },
    { value: 'light', label: 'Light', icon: Sun },
    { value: 'system', label: 'System', icon: Monitor },
  ] as const
  return (
    <>
      {options.map((o) => (
        <DropdownMenuItem key={o.value} onSelect={() => setTheme(o.value)}>
          <o.icon /> {o.label}
          {theme === o.value && <span className="ml-auto text-xs text-primary">Active</span>}
        </DropdownMenuItem>
      ))}
    </>
  )
}

function ThemeToggle() {
  const { theme, setTheme } = useTheme()
  const isDark = theme === 'dark' || (theme === 'system' && document.documentElement.classList.contains('dark'))
  const options = [
    { dark: false, label: 'Light mode', icon: Sun },
    { dark: true, label: 'Dark mode', icon: Moon },
  ]
  return (
    <div role="group" aria-label="Theme" className="hidden items-center gap-0.5 rounded-full border bg-card p-1 sm:flex">
      {options.map((o) => {
        const active = o.dark === isDark
        return (
          <button
            key={o.label}
            type="button"
            onClick={() => setTheme(o.dark ? 'dark' : 'light')}
            aria-label={o.label}
            aria-pressed={active}
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

function UserMenu() {
  const { user, logout, isAdmin } = useUser()
  const navigate = useNavigate()
  return (
    <DropdownMenu>
      <DropdownMenuTrigger asChild>
        <button className="flex items-center gap-2.5 rounded-full py-0.5 pr-1 pl-0.5 outline-none hover:bg-accent focus-visible:ring-2 focus-visible:ring-ring/50 md:pr-3" aria-label="Account menu">
          <Avatar name={user.name} className="size-9" />
          <span className="hidden max-w-40 truncate text-sm font-semibold md:block">{user.name}</span>
        </button>
      </DropdownMenuTrigger>
      <DropdownMenuContent align="end" className="w-56">
        <DropdownMenuLabel className="font-normal">
          <div className="truncate text-sm font-medium text-foreground">{user.name}</div>
          <div className="truncate text-xs">{user.email}</div>
          <div className="mt-1 text-[11px] text-primary">{isAdmin ? 'Instructor' : 'Student'}</div>
        </DropdownMenuLabel>
        <DropdownMenuSeparator />
        <DropdownMenuItem onSelect={() => navigate('/settings')}>
          <Settings /> Settings
        </DropdownMenuItem>
        <DropdownMenuSeparator />
        <ThemeMenuItems />
        <DropdownMenuSeparator />
        <DropdownMenuItem
          onSelect={async () => {
            await logout()
            navigate('/login')
          }}
        >
          <LogOut /> Sign out
        </DropdownMenuItem>
      </DropdownMenuContent>
    </DropdownMenu>
  )
}

export function AppLayout() {
  const [collapsed, setCollapsed] = React.useState(() => {
    try {
      return localStorage.getItem(COLLAPSE_KEY) === '1'
    } catch {
      return false
    }
  })
  const [mobileOpen, setMobileOpen] = React.useState(false)
  const [searchOpen, setSearchOpen] = React.useState(false)
  const location = useLocation()
  const mainRef = React.useRef<HTMLElement>(null)

  React.useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if ((e.metaKey || e.ctrlKey) && e.key.toLowerCase() === 'k') {
        e.preventDefault()
        setSearchOpen((o) => !o)
      }
    }
    window.addEventListener('keydown', onKey)
    return () => window.removeEventListener('keydown', onKey)
  }, [])

  React.useEffect(() => {
    mainRef.current?.scrollTo({ top: 0 })
  }, [location.pathname])

  const toggleCollapsed = () => {
    setCollapsed((c) => {
      try {
        localStorage.setItem(COLLAPSE_KEY, c ? '0' : '1')
      } catch {
        /* ignore */
      }
      return !c
    })
  }

  return (
    <BreadcrumbProvider>
      <a href="#main" className="sr-only z-50 rounded-md bg-primary px-3 py-2 text-primary-foreground focus:not-sr-only focus:fixed focus:top-2 focus:left-2">
        Skip to content
      </a>
      <div className="flex h-dvh overflow-hidden">
        <aside className={cn('hidden shrink-0 flex-col border-r border-sidebar-border bg-sidebar transition-[width] duration-200 lg:flex', collapsed ? 'w-[64px]' : 'w-[248px]')}>
          <div className={cn('flex h-[68px] items-center border-b border-sidebar-border px-4', collapsed && 'justify-center')}>
            <Logo collapsed={collapsed} />
          </div>
          <SidebarBody collapsed={collapsed} />
          <div className={cn('border-t border-sidebar-border p-2', collapsed && 'flex justify-center')}>
            <Button variant="ghost" size={collapsed ? 'icon' : 'sm'} className={cn(!collapsed && 'w-full justify-start text-muted-foreground')} onClick={toggleCollapsed} aria-label={collapsed ? 'Expand sidebar' : 'Collapse sidebar'}>
              {collapsed ? <PanelLeftOpen /> : <PanelLeftClose />}
              {!collapsed && 'Collapse'}
            </Button>
          </div>
        </aside>

        <Sheet open={mobileOpen} onOpenChange={setMobileOpen}>
          <SheetContent>
            <div className="flex h-[68px] items-center border-b border-sidebar-border px-4">
              <Logo />
            </div>
            <SidebarBody onNavigate={() => setMobileOpen(false)} />
          </SheetContent>
        </Sheet>

        <div className="flex min-w-0 flex-1 flex-col">
          <header className="flex h-[68px] shrink-0 items-center gap-2 border-b bg-background/80 px-3 backdrop-blur sm:px-7">
            <Button variant="ghost" size="icon" className="lg:hidden" onClick={() => setMobileOpen(true)} aria-label="Open navigation">
              <Menu />
            </Button>
            <button
              type="button"
              onClick={() => setSearchOpen(true)}
              className="hidden h-11 w-full max-w-xl items-center gap-3 rounded-md border bg-card px-4 text-sm text-muted-foreground transition hover:bg-muted focus-visible:ring-2 focus-visible:ring-ring/50 focus-visible:outline-none sm:flex"
            >
              <Search className="size-[18px]" />
              <span className="flex-1 truncate text-left">Search courses, lessons, resources…</span>
              <Kbd>⌘K</Kbd>
            </button>
            <div className="ml-auto flex items-center gap-2">
              <Button variant="ghost" size="icon" className="sm:hidden" onClick={() => setSearchOpen(true)} aria-label="Search">
                <Search />
              </Button>
              <NotificationCenter />
              <ThemeToggle />
              <div className="mx-1 hidden h-7 w-px bg-border sm:block" />
              <UserMenu />
            </div>
          </header>
          <main id="main" ref={mainRef} className="flex-1 overflow-y-auto">
            <div key={location.pathname} className="mx-auto w-full max-w-7xl animate-fade-in px-4 py-6 sm:px-6 lg:px-7 lg:py-7">
              <Breadcrumbs />
              <Outlet />
            </div>
          </main>
        </div>
      </div>
      <CommandSearch open={searchOpen} onOpenChange={setSearchOpen} />
    </BreadcrumbProvider>
  )
}
