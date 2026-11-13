import { useInfiniteQuery } from '@tanstack/react-query'
import { ScrollText, Search } from 'lucide-react'
import * as React from 'react'
import { miscApi } from '@/api/endpoints'
import { EmptyState, ErrorState, PageHeader, PageSkeleton } from '@/components/page'
import { Button } from '@/components/ui/button'
import { Card, CardContent, CardFooter } from '@/components/ui/card'
import { Input, NativeSelect } from '@/components/ui/input'
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from '@/components/ui/table'
import { useAuth } from '@/lib/auth'
import { useBreadcrumbs } from '@/lib/breadcrumbs'
import { AUDIT_CATEGORIES, auditActionLabel, auditDetails } from '@/lib/labels'
import { formatDateTime, relativeTime } from '@/lib/utils'

const PAGE_SIZE = 100

export function ActivityLogPage() {
  useBreadcrumbs([{ label: 'Activity Log' }])
  const { timezone } = useAuth()
  const [category, setCategory] = React.useState('')
  const [search, setSearch] = React.useState('')
  const q = useInfiniteQuery({
    queryKey: ['activity-log', category],
    queryFn: ({ pageParam }) => miscApi.auditLogs({ limit: PAGE_SIZE, action: category || undefined, before: pageParam || undefined }),
    initialPageParam: '',
    getNextPageParam: (last) => (last.length === PAGE_SIZE ? last[last.length - 1].created_at : undefined),
  })

  const entries = q.data?.pages.flat() ?? []
  const needle = search.trim().toLowerCase()
  const shown = needle
    ? entries.filter((l) => [l.actor_name, l.action, auditActionLabel(l.action), l.details, l.ip_address].some((v) => v?.toLowerCase().includes(needle)))
    : entries

  return (
    <>
      <PageHeader
        title="Activity Log"
        description="Who changed what and when: sign-ins, attendance, calendar events, courses, assignments and other administrative actions."
        actions={
          <>
            <div className="relative w-full sm:w-56">
              <Search className="pointer-events-none absolute top-1/2 left-2.5 size-4 -translate-y-1/2 text-muted-foreground" />
              <Input value={search} onChange={(e) => setSearch(e.target.value)} placeholder="Search loaded entries" className="pl-8" aria-label="Search the activity log" />
            </div>
            <NativeSelect value={category} onChange={(e) => setCategory(e.target.value)} className="w-full sm:w-56" aria-label="Filter by area">
              {AUDIT_CATEGORIES.map((c) => (
                <option key={c.value} value={c.value}>
                  {c.label}
                </option>
              ))}
            </NativeSelect>
          </>
        }
      />
      {q.isLoading ? (
        <PageSkeleton rows={1} />
      ) : q.error ? (
        <ErrorState error={q.error} onRetry={() => q.refetch()} />
      ) : shown.length === 0 ? (
        <EmptyState icon={ScrollText} title="No activity" description={needle ? 'Nothing in the loaded entries matches your search.' : 'Nothing has been recorded for this filter yet.'} />
      ) : (
        <Card>
          <CardContent className="overflow-x-auto px-0">
            <Table>
              <TableHeader>
                <TableRow>
                  <TableHead className="pl-5">When</TableHead>
                  <TableHead>Who</TableHead>
                  <TableHead>Action</TableHead>
                  <TableHead>Details</TableHead>
                  <TableHead className="pr-5">IP</TableHead>
                </TableRow>
              </TableHeader>
              <TableBody>
                {shown.map((l) => {
                  const details = auditDetails(l.details)
                  return (
                    <TableRow key={l.id}>
                      <TableCell className="pl-5 align-top whitespace-nowrap">
                        <div className="text-sm">{formatDateTime(l.created_at, timezone)}</div>
                        <div className="text-xs text-muted-foreground">{relativeTime(l.created_at)}</div>
                      </TableCell>
                      <TableCell className="align-top whitespace-nowrap">{l.actor_name ?? <span className="text-muted-foreground">System</span>}</TableCell>
                      <TableCell className="align-top">
                        <div className="text-sm font-medium whitespace-nowrap">{auditActionLabel(l.action)}</div>
                        <code className="font-mono text-[11px] text-muted-foreground">{l.action}</code>
                      </TableCell>
                      <TableCell className="max-w-md align-top text-xs text-muted-foreground">
                        {details.length ? (
                          <dl className="flex flex-wrap gap-x-3 gap-y-0.5">
                            {details.map(([k, v]) => (
                              <div key={k} className="min-w-0">
                                <dt className="inline">{k}: </dt>
                                <dd className="inline break-all text-foreground">{v.length > 80 ? `${v.slice(0, 79)}…` : v}</dd>
                              </div>
                            ))}
                          </dl>
                        ) : (
                          '·'
                        )}
                      </TableCell>
                      <TableCell className="pr-5 align-top font-mono text-xs whitespace-nowrap text-muted-foreground">{l.ip_address ?? '·'}</TableCell>
                    </TableRow>
                  )
                })}
              </TableBody>
            </Table>
          </CardContent>
          <CardFooter className="justify-between gap-3 border-t pt-4">
            <span className="text-xs text-muted-foreground">
              {needle ? `${shown.length} of ${entries.length} loaded entries` : `${entries.length} entries`}
            </span>
            {q.hasNextPage && (
              <Button variant="outline" size="sm" onClick={() => q.fetchNextPage()} loading={q.isFetchingNextPage}>
                Load older
              </Button>
            )}
          </CardFooter>
        </Card>
      )}
    </>
  )
}
