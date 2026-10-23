import { useQuery } from '@tanstack/react-query'
import { Command } from 'cmdk'
import { BookOpen, CalendarRange, FileText, FolderOpen, GraduationCap, ListChecks, Megaphone, Search, User } from 'lucide-react'
import * as React from 'react'
import { useNavigate } from 'react-router'
import { miscApi } from '@/api/endpoints'
import { Dialog, DialogContent, DialogTitle } from '@/components/ui/dialog'
import { useDebounced } from '@/hooks/queries'
import { useAuth } from '@/lib/auth'
import { ADMIN_NAV, MAIN_NAV } from './nav'
import type { SearchResult } from '@/types/api'

const KIND_ICONS: Record<SearchResult['kind'], typeof Search> = {
  course: GraduationCap,
  week: CalendarRange,
  lesson: BookOpen,
  assignment: ListChecks,
  resource: FolderOpen,
  announcement: Megaphone,
  student: User,
}

/** Global search (⌘K / Ctrl+K) across pages and content the user can access. */
export function CommandSearch({ open, onOpenChange }: { open: boolean; onOpenChange: (o: boolean) => void }) {
  const [q, setQ] = React.useState('')
  const term = useDebounced(q.trim(), 200)
  const navigate = useNavigate()
  const { isAdmin } = useAuth()
  const results = useQuery({ queryKey: ['search', term], queryFn: () => miscApi.search(term), enabled: open && term.length >= 2 })
  const pages = [...MAIN_NAV, ...(isAdmin ? ADMIN_NAV : [])]

  const go = (to: string) => {
    onOpenChange(false)
    setQ('')
    navigate(to)
  }

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="top-[15vh] translate-y-0 gap-0 overflow-hidden p-0 [&>button:last-child]:hidden">
        <DialogTitle className="sr-only">Search</DialogTitle>
        <Command shouldFilter={false} className="flex flex-col" label="Global search">
          <div className="flex items-center gap-2 border-b px-3">
            <Search className="size-4 text-muted-foreground" />
            <Command.Input
              value={q}
              onValueChange={setQ}
              placeholder="Search lessons, assignments, resources…"
              className="h-12 flex-1 bg-transparent text-sm outline-none placeholder:text-muted-foreground"
            />
          </div>
          <Command.List className="max-h-[60vh] overflow-y-auto p-2">
            {term.length >= 2 && results.isFetching && <div className="px-3 py-6 text-center text-sm text-muted-foreground">Searching…</div>}
            {term.length >= 2 && !results.isFetching && results.data?.length === 0 && (
              <Command.Empty className="px-3 py-6 text-center text-sm text-muted-foreground">No results for “{term}”.</Command.Empty>
            )}
            {term.length < 2 && (
              <Command.Group heading="Go to" className="text-xs text-muted-foreground [&_[cmdk-group-heading]]:px-2 [&_[cmdk-group-heading]]:py-1.5">
                {pages.map((p) => (
                  <Command.Item key={p.to} value={p.to} onSelect={() => go(p.to)} className="flex cursor-pointer items-center gap-2 rounded-md px-2 py-2 text-sm text-foreground data-[selected=true]:bg-accent">
                    <p.icon className="size-4 text-muted-foreground" />
                    {p.label}
                  </Command.Item>
                ))}
              </Command.Group>
            )}
            {results.data?.map((r) => {
              const Icon = KIND_ICONS[r.kind] ?? FileText
              return (
                <Command.Item key={`${r.kind}-${r.id}`} value={`${r.kind}-${r.id}`} onSelect={() => go(r.link)} className="flex cursor-pointer items-center gap-3 rounded-md px-2 py-2 data-[selected=true]:bg-accent">
                  <Icon className="size-4 shrink-0 text-muted-foreground" />
                  <div className="min-w-0">
                    <div className="truncate text-sm">{r.title}</div>
                    <div className="truncate text-xs text-muted-foreground">{r.subtitle}</div>
                  </div>
                </Command.Item>
              )
            })}
          </Command.List>
        </Command>
      </DialogContent>
    </Dialog>
  )
}
