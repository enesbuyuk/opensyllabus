import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query'
import { FolderOpen, MoreHorizontal, Pencil, Plus, Search, Trash2 } from 'lucide-react'
import * as React from 'react'
import { useSearchParams } from 'react-router'
import { toast } from 'sonner'
import { coursesApi, resourcesApi } from '@/api/endpoints'
import { CourseSelect } from '@/components/course-select'
import { EmptyState, ErrorState, PageHeader } from '@/components/page'
import { RESOURCE_ICONS, ResourceActions, ResourceViewers } from '@/components/resource-list'
import { ConfirmDialog } from '@/components/ui/alert-dialog'
import { Badge } from '@/components/ui/badge'
import { Button } from '@/components/ui/button'
import { DropdownMenu, DropdownMenuContent, DropdownMenuItem, DropdownMenuTrigger } from '@/components/ui/dropdown-menu'
import { Input, NativeSelect } from '@/components/ui/input'
import { Skeleton } from '@/components/ui/misc'
import { ResourceDialog } from '@/features/resources/ResourceDialog'
import { useDebounced, useSelectedCourse } from '@/hooks/queries'
import { useAuth } from '@/lib/auth'
import { useBreadcrumbs } from '@/lib/breadcrumbs'
import { errorMessage } from '@/lib/errors'
import { RESOURCE_KIND_LABELS } from '@/lib/labels'
import { cn, formatBytes, formatDate } from '@/lib/utils'
import type { FileMeta, Resource, ResourceKind } from '@/types/api'

export function ResourcesPage() {
  useBreadcrumbs([{ label: 'Resources' }])
  const { isAdmin, timezone } = useAuth()
  const qc = useQueryClient()
  const [params] = useSearchParams()
  const highlight = params.get('highlight')
  const { courses, course, select } = useSelectedCourse()
  const [search, setSearch] = React.useState('')
  const [kind, setKind] = React.useState('')
  const [weekId, setWeekId] = React.useState('')
  const [moduleId, setModuleId] = React.useState('')
  const [sort, setSort] = React.useState('newest')
  const q = useDebounced(search)
  const curriculum = useQuery({ queryKey: ['curriculum', course?.id], queryFn: () => coursesApi.curriculum(course!.id), enabled: !!course })
  const list = useQuery({
    queryKey: ['resources', course?.id, q, kind, weekId, moduleId, sort],
    queryFn: () => resourcesApi.list({ course_id: course?.id, q, kind, week_id: weekId, module_id: moduleId, sort }),
    enabled: !!course,
  })
  const [dialog, setDialog] = React.useState<{ open: boolean; resource: Resource | null }>({ open: false, resource: null })
  const [deleting, setDeleting] = React.useState<Resource | null>(null)
  const [preview, setPreview] = React.useState<FileMeta | null>(null)
  const [reading, setReading] = React.useState<Resource | null>(null)
  const del = useMutation({
    mutationFn: (id: string) => resourcesApi.remove(id),
    onSuccess: () => {
      toast.success('Resource deleted')
      setDeleting(null)
      qc.invalidateQueries({ queryKey: ['resources'] })
    },
    onError: (e) => toast.error(errorMessage(e)),
  })

  React.useEffect(() => {
    if (highlight && list.data) document.getElementById(`resource-${highlight}`)?.scrollIntoView({ block: 'center' })
  }, [highlight, list.data])

  if (!courses.isLoading && !course) return <EmptyState icon={FolderOpen} title="No courses available" description="Resources appear once you are enrolled in a course." />
  const items = list.data ?? []

  return (
    <>
      <PageHeader
        eyebrow={course?.title}
        title="Resources"
        description="Datasheets, lab instructions, notes, code examples, videos and links."
        actions={
          <>
            <CourseSelect courses={courses.data ?? []} value={course?.id} onChange={select} />
            {isAdmin && (
              <Button onClick={() => setDialog({ open: true, resource: null })}>
                <Plus /> Add resource
              </Button>
            )}
          </>
        }
      />
      <div className="mb-4 grid gap-2 sm:grid-cols-2 lg:grid-cols-[1fr_repeat(4,minmax(0,11rem))]">
        <div className="relative">
          <Search className="pointer-events-none absolute top-1/2 left-2.5 size-4 -translate-y-1/2 text-muted-foreground" />
          <Input value={search} onChange={(e) => setSearch(e.target.value)} placeholder="Search resources…" className="pl-8" aria-label="Search resources" />
        </div>
        <NativeSelect value={kind} onChange={(e) => setKind(e.target.value)} aria-label="Type">
          <option value="">All types</option>
          {(Object.keys(RESOURCE_KIND_LABELS) as ResourceKind[]).map((k) => (
            <option key={k} value={k}>
              {RESOURCE_KIND_LABELS[k]}
            </option>
          ))}
        </NativeSelect>
        <NativeSelect value={moduleId} onChange={(e) => setModuleId(e.target.value)} aria-label="Module">
          <option value="">All modules</option>
          {curriculum.data?.modules.map((m) => (
            <option key={m.id} value={m.id}>
              {m.title}
            </option>
          ))}
        </NativeSelect>
        <NativeSelect value={weekId} onChange={(e) => setWeekId(e.target.value)} aria-label="Week">
          <option value="">All weeks</option>
          {curriculum.data?.weeks
            .filter((w) => isAdmin || w.state !== 'locked')
            .map((w) => (
              <option key={w.id} value={w.id}>
                Week {w.week_number}
              </option>
            ))}
        </NativeSelect>
        <NativeSelect value={sort} onChange={(e) => setSort(e.target.value)} aria-label="Sort">
          <option value="newest">Newest first</option>
          <option value="oldest">Oldest first</option>
          <option value="title">Title A–Z</option>
          <option value="week">By week</option>
        </NativeSelect>
      </div>

      {list.isLoading || courses.isLoading ? (
        <Skeleton className="h-72 rounded-xl" />
      ) : list.error ? (
        <ErrorState error={list.error} onRetry={() => list.refetch()} />
      ) : items.length === 0 ? (
        <EmptyState icon={FolderOpen} title="No resources found" description={search || kind || weekId || moduleId ? 'Try clearing the filters.' : 'Resources added by your instructor will appear here.'} />
      ) : (
        <ul className="divide-y overflow-hidden rounded-xl border bg-card">
          {items.map((r) => {
            const Icon = RESOURCE_ICONS[r.kind]
            return (
              <li key={r.id} id={`resource-${r.id}`} className={cn('flex items-center gap-3 px-4 py-3', highlight === r.id && 'bg-primary/5')}>
                <div className="flex size-9 shrink-0 items-center justify-center rounded-lg border bg-muted/40">
                  <Icon className="size-4 text-muted-foreground" />
                </div>
                <div className="min-w-0 flex-1">
                  <div className="flex flex-wrap items-center gap-2">
                    <span className="truncate text-sm font-medium">{r.title}</span>
                    <Badge variant="outline">{RESOURCE_KIND_LABELS[r.kind]}</Badge>
                    {r.visibility === 'instructor' && <Badge variant="warning">Instructor only</Badge>}
                  </div>
                  <div className="mt-0.5 flex flex-wrap gap-x-3 text-xs text-muted-foreground">
                    {r.module_title && <span>{r.module_title}</span>}
                    {r.week_number && <span>Week {r.week_number}</span>}
                    {r.lesson_title && <span>{r.lesson_title}</span>}
                    {r.file_size != null && <span>{formatBytes(r.file_size)}</span>}
                    <span>Added {formatDate(r.created_at, timezone)}</span>
                  </div>
                  {r.description && <p className="mt-1 line-clamp-1 text-[13px] text-muted-foreground">{r.description}</p>}
                </div>
                <ResourceActions r={r} onPreview={setPreview} onRead={setReading} />
                {isAdmin && (
                  <DropdownMenu>
                    <DropdownMenuTrigger asChild>
                      <Button variant="ghost" size="icon-sm" aria-label={`Manage ${r.title}`}>
                        <MoreHorizontal />
                      </Button>
                    </DropdownMenuTrigger>
                    <DropdownMenuContent align="end">
                      <DropdownMenuItem onSelect={() => setDialog({ open: true, resource: r })}>
                        <Pencil /> Edit
                      </DropdownMenuItem>
                      <DropdownMenuItem destructive onSelect={() => setDeleting(r)}>
                        <Trash2 /> Delete
                      </DropdownMenuItem>
                    </DropdownMenuContent>
                  </DropdownMenu>
                )}
              </li>
            )
          })}
        </ul>
      )}
      <ResourceViewers preview={preview} setPreview={setPreview} reading={reading} setReading={setReading} />
      {isAdmin && (
        <>
          <ResourceDialog open={dialog.open} onOpenChange={(o) => setDialog((d) => ({ ...d, open: o }))} resource={dialog.resource} defaults={{ course_id: course?.id }} />
          <ConfirmDialog
            open={!!deleting}
            onOpenChange={(o) => !o && setDeleting(null)}
            title="Delete this resource?"
            description={`“${deleting?.title}” will be removed for all students.`}
            destructive
            confirmLabel="Delete"
            loading={del.isPending}
            onConfirm={() => deleting && del.mutate(deleting.id)}
          />
        </>
      )}
    </>
  )
}
