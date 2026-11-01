import { zodResolver } from '@hookform/resolvers/zod'
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query'
import { addDays, addMonths, endOfMonth, endOfWeek, format, startOfMonth, startOfWeek } from 'date-fns'
import { CalendarDays, ChevronLeft, ChevronRight, Plus, Repeat, Trash2 } from 'lucide-react'
import * as React from 'react'
import { useForm } from 'react-hook-form'
import { Link } from 'react-router'
import { toast } from 'sonner'
import { z } from 'zod'
import { calendarApi, type EventInput } from '@/api/endpoints'
import { CourseSelect } from '@/components/course-select'
import { Field } from '@/components/form'
import { EmptyState, ErrorState, PageHeader } from '@/components/page'
import { ConfirmDialog } from '@/components/ui/alert-dialog'
import { Badge, type BadgeVariant } from '@/components/ui/badge'
import { Button } from '@/components/ui/button'
import { Dialog, DialogContent, DialogDescription, DialogFooter, DialogHeader, DialogTitle } from '@/components/ui/dialog'
import { Input, NativeSelect, Textarea } from '@/components/ui/input'
import { Skeleton } from '@/components/ui/misc'
import { Tabs, TabsList, TabsTrigger } from '@/components/ui/tabs'
import { useCourses } from '@/hooks/queries'
import { useAuth } from '@/lib/auth'
import { useBreadcrumbs } from '@/lib/breadcrumbs'
import { handleFormError } from '@/lib/errors'
import { CALENDAR_KIND_LABELS, describeRecurrence, FREQ_UNIT, WEEKDAYS } from '@/lib/labels'
import { cn, formatPlainDate, formatTime, isoToWallTime, todayIn, wallTimeToIso } from '@/lib/utils'
import type { CalendarEntry, RecurrenceFreq } from '@/types/api'

const KIND_STYLE: Record<CalendarEntry['kind'], { dot: string; badge: BadgeVariant }> = {
  deadline: { dot: 'bg-destructive', badge: 'destructive' },
  session: { dot: 'bg-primary', badge: 'default' },
  lab: { dot: 'bg-chart-2', badge: 'info' },
  milestone: { dot: 'bg-warning', badge: 'warning' },
  other: { dot: 'bg-muted-foreground', badge: 'secondary' },
  course_start: { dot: 'bg-success', badge: 'success' },
  course_end: { dot: 'bg-success', badge: 'success' },
  week_start: { dot: 'bg-muted-foreground/60', badge: 'outline' },
  class: { dot: 'bg-chart-4', badge: 'secondary' },
}

function dayKey(e: CalendarEntry, tz: string) {
  return e.all_day ? e.starts_at.slice(0, 10) : isoToWallTime(e.starts_at, tz).slice(0, 10)
}

/** ISO weekday (1 = Monday) of a "YYYY-MM-DDTHH:mm" wall time. */
function isoWeekday(wall: string) {
  const [y, m, d] = wall.slice(0, 10).split('-').map(Number)
  return ((new Date(Date.UTC(y, m - 1, d)).getUTCDay() + 6) % 7) + 1
}

const eventSchema = z
  .object({
    course_id: z.string().min(1, 'Select a course'),
    title: z.string().trim().min(1, 'Title is required').max(200),
    kind: z.enum(['session', 'lab', 'milestone', 'other']),
    starts_at: z.string().min(1, 'Start time is required'),
    ends_at: z.string(),
    location: z.string().max(200),
    description: z.string().max(5000),
    repeat: z.enum(['none', 'daily', 'weekly', 'monthly']),
    interval: z.number({ message: 'Enter a number' }).int().min(1, 'At least 1').max(99, 'At most 99'),
    weekdays: z.array(z.number()),
    end_mode: z.enum(['never', 'until', 'count']),
    until: z.string(),
    count: z.number({ message: 'Enter a number' }).int().min(1, 'At least 1').max(500, 'At most 500'),
  })
  .refine((v) => !v.ends_at || v.ends_at >= v.starts_at, { message: 'End must be after start', path: ['ends_at'] })
  .refine((v) => v.repeat === 'none' || v.end_mode !== 'until' || (v.until && v.until >= v.starts_at.slice(0, 10)), {
    message: 'Pick a date on or after the start',
    path: ['until'],
  })
type EventForm = z.infer<typeof eventSchema>

const NO_REPEAT = { repeat: 'none', interval: 1, weekdays: [], end_mode: 'never', until: '', count: 10 } as const
// The dialog's hooks run while it is closed too: every field needs a value before the first reset().
const EMPTY_EVENT: EventForm = { course_id: '', title: '', kind: 'session', starts_at: '', ends_at: '', location: '', description: '', ...NO_REPEAT, weekdays: [] }

function EventDialog({ open, onOpenChange, entry, defaultDate }: { open: boolean; onOpenChange: (o: boolean) => void; entry: CalendarEntry | null; defaultDate?: string }) {
  const { timezone } = useAuth()
  const courses = useCourses()
  const qc = useQueryClient()
  const [confirm, setConfirm] = React.useState(false)
  const form = useForm<EventForm>({ resolver: zodResolver(eventSchema), defaultValues: EMPTY_EVENT })
  const series = entry?.series ?? null
  // A repeating event keeps the timezone it was created in; show its times there.
  const tz = series?.timezone ?? timezone

  React.useEffect(() => {
    if (!open) return
    const rule = series?.recurrence
    form.reset(
      entry
        ? {
            course_id: entry.course_id,
            title: entry.title,
            kind: entry.kind as EventForm['kind'],
            // For a series, edit the rule from its first occurrence.
            starts_at: isoToWallTime(series?.starts_at ?? entry.starts_at, tz),
            ends_at: isoToWallTime(series ? series.ends_at : entry.ends_at, tz),
            location: entry.location,
            description: entry.description,
            ...(rule
              ? {
                  repeat: rule.freq,
                  interval: rule.interval,
                  weekdays: rule.weekdays,
                  end_mode: rule.until ? 'until' : rule.count ? 'count' : 'never',
                  until: rule.until ?? '',
                  count: rule.count ?? 10,
                }
              : { ...NO_REPEAT, weekdays: [] }),
          }
        : {
            course_id: courses.data?.[0]?.id ?? '',
            title: '',
            kind: 'session',
            starts_at: `${defaultDate ?? todayIn(timezone)}T10:00`,
            ends_at: '',
            location: '',
            description: '',
            ...NO_REPEAT,
            weekdays: [],
          },
    )
  }, [open, entry, series, defaultDate, courses.data, timezone, tz, form])

  const repeat = form.watch('repeat')
  const endMode = form.watch('end_mode')
  const weekdays = form.watch('weekdays')
  const interval = form.watch('interval')
  const startsAt = form.watch('starts_at')
  // Weekly with no day picked: start from the start date's weekday.
  React.useEffect(() => {
    if (repeat === 'weekly' && weekdays?.length === 0 && startsAt) form.setValue('weekdays', [isoWeekday(startsAt)])
  }, [repeat, weekdays, startsAt, form])

  const done = () => {
    qc.invalidateQueries({ queryKey: ['calendar'] })
    onOpenChange(false)
  }
  const onSubmit = form.handleSubmit(async (v) => {
    const recurrence: EventInput['recurrence'] =
      v.repeat === 'none'
        ? series
          ? { freq: 'none', interval: 1, weekdays: [], until: null, count: null }
          : undefined
        : {
            freq: v.repeat,
            interval: v.interval,
            weekdays: v.repeat === 'weekly' ? [...v.weekdays].sort() : [],
            until: v.end_mode === 'until' ? v.until : null,
            count: v.end_mode === 'count' ? v.count : null,
          }
    const body: EventInput = {
      course_id: v.course_id,
      title: v.title,
      kind: v.kind,
      location: v.location,
      description: v.description,
      starts_at: wallTimeToIso(v.starts_at, tz),
      ends_at: v.ends_at ? wallTimeToIso(v.ends_at, tz) : '',
      recurrence,
      timezone: tz,
    }
    try {
      if (entry?.event_id) await calendarApi.update(entry.event_id, body)
      else await calendarApi.create(body)
      toast.success(entry ? 'Event updated' : 'Event created')
      done()
    } catch (e) {
      handleFormError(e, form.setError)
    }
  })
  const del = useMutation({
    mutationFn: async (scope: 'occurrence' | 'series') => {
      if (scope === 'occurrence') await calendarApi.skip(entry!.event_id!, series!.occurrence_date)
      else await calendarApi.remove(entry!.event_id!)
    },
    onSuccess: (_, scope) => {
      toast.success(scope === 'occurrence' ? 'Occurrence removed' : 'Event deleted')
      setConfirm(false)
      done()
    },
  })
  const { errors, isSubmitting } = form.formState
  const unit = repeat in FREQ_UNIT ? FREQ_UNIT[repeat as RecurrenceFreq][interval === 1 ? 0 : 1] : ''

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="max-h-[90dvh] overflow-y-auto">
        <DialogHeader>
          <DialogTitle>{entry ? 'Edit event' : 'New event'}</DialogTitle>
          {series && (
            <DialogDescription className="flex items-center gap-1.5">
              <Repeat className="size-3.5" /> Repeating event. Changes apply to every occurrence.
            </DialogDescription>
          )}
        </DialogHeader>
        <form onSubmit={onSubmit} className="space-y-4">
          <div className="grid gap-4 sm:grid-cols-2">
            <Field label="Course" htmlFor="ev-course" error={errors.course_id?.message}>
              <NativeSelect id="ev-course" disabled={!!entry} {...form.register('course_id')}>
                {courses.data?.map((c) => (
                  <option key={c.id} value={c.id}>
                    {c.title}
                  </option>
                ))}
              </NativeSelect>
            </Field>
            <Field label="Type" htmlFor="ev-kind">
              <NativeSelect id="ev-kind" {...form.register('kind')}>
                <option value="session">Session</option>
                <option value="lab">Laboratory</option>
                <option value="milestone">Milestone</option>
                <option value="other">Other</option>
              </NativeSelect>
            </Field>
          </div>
          <Field label="Title" htmlFor="ev-title" error={errors.title?.message} required>
            <Input id="ev-title" {...form.register('title')} />
          </Field>
          <div className="grid gap-4 sm:grid-cols-2">
            <Field label={series ? 'First occurrence starts' : 'Starts'} htmlFor="ev-start" error={errors.starts_at?.message} hint={tz} required>
              <Input id="ev-start" type="datetime-local" {...form.register('starts_at')} />
            </Field>
            <Field label="Ends" htmlFor="ev-end" error={errors.ends_at?.message}>
              <Input id="ev-end" type="datetime-local" {...form.register('ends_at')} />
            </Field>
          </div>

          <fieldset className="space-y-3 rounded-lg border p-3">
            <div className="grid gap-3 sm:grid-cols-2">
              <Field label="Repeat" htmlFor="ev-repeat">
                <NativeSelect id="ev-repeat" {...form.register('repeat')}>
                  <option value="none">Does not repeat</option>
                  <option value="daily">Daily</option>
                  <option value="weekly">Weekly</option>
                  <option value="monthly">Monthly (same day of the month)</option>
                </NativeSelect>
              </Field>
              {repeat !== 'none' && (
                <Field label="Every" htmlFor="ev-interval" error={errors.interval?.message}>
                  <div className="flex items-center gap-2">
                    <Input id="ev-interval" type="number" min={1} max={99} className="w-20" {...form.register('interval', { valueAsNumber: true })} />
                    <span className="text-sm text-muted-foreground">{unit}</span>
                  </div>
                </Field>
              )}
            </div>
            {repeat === 'weekly' && (
              <div>
                <div className="mb-1.5 text-[13px] font-medium" id="ev-days-label">
                  On
                </div>
                <div className="flex flex-wrap gap-1.5" role="group" aria-labelledby="ev-days-label">
                  {WEEKDAYS.map((d) => {
                    const on = weekdays?.includes(d.n)
                    return (
                      <button
                        key={d.n}
                        type="button"
                        aria-pressed={on}
                        aria-label={d.short}
                        title={d.short}
                        onClick={() => {
                          const next = on ? weekdays.filter((x) => x !== d.n) : [...weekdays, d.n]
                          if (next.length) form.setValue('weekdays', next, { shouldDirty: true })
                        }}
                        className={cn(
                          'flex size-8 items-center justify-center rounded-full border text-xs font-medium transition-colors focus-visible:ring-2 focus-visible:ring-ring focus-visible:outline-none',
                          on ? 'border-primary bg-primary text-primary-foreground' : 'hover:bg-accent',
                        )}
                      >
                        {d.letter}
                      </button>
                    )
                  })}
                </div>
              </div>
            )}
            {repeat !== 'none' && (
              <div className="grid gap-3 sm:grid-cols-2">
                <Field label="Repeat ends" htmlFor="ev-endmode">
                  <NativeSelect id="ev-endmode" {...form.register('end_mode')}>
                    <option value="never">Never</option>
                    <option value="until">On a date</option>
                    <option value="count">After a number of times</option>
                  </NativeSelect>
                </Field>
                {endMode === 'until' && (
                  <Field label="Last date" htmlFor="ev-until" error={errors.until?.message}>
                    <Input id="ev-until" type="date" {...form.register('until')} />
                  </Field>
                )}
                {endMode === 'count' && (
                  <Field label="Occurrences" htmlFor="ev-count" error={errors.count?.message}>
                    <Input id="ev-count" type="number" min={1} max={500} {...form.register('count', { valueAsNumber: true })} />
                  </Field>
                )}
              </div>
            )}
            {errors.root?.message && <p className="text-xs text-destructive">{errors.root.message}</p>}
          </fieldset>

          <Field label="Location" htmlFor="ev-loc">
            <Input id="ev-loc" placeholder="e.g. Electronics Lab 2" {...form.register('location')} />
          </Field>
          <Field label="Description" htmlFor="ev-desc">
            <Textarea id="ev-desc" rows={3} {...form.register('description')} />
          </Field>
          <DialogFooter className="sm:justify-between">
            {entry?.event_id ? (
              <Button type="button" variant="ghost" className="text-destructive" onClick={() => setConfirm(true)}>
                <Trash2 /> Delete
              </Button>
            ) : (
              <span />
            )}
            <div className="flex gap-2">
              <Button type="button" variant="outline" onClick={() => onOpenChange(false)}>
                Cancel
              </Button>
              <Button type="submit" loading={isSubmitting}>
                Save event
              </Button>
            </div>
          </DialogFooter>
        </form>
        {series ? (
          <Dialog open={confirm} onOpenChange={setConfirm}>
            <DialogContent className="max-w-md">
              <DialogHeader>
                <DialogTitle>Delete repeating event</DialogTitle>
                <DialogDescription>
                  Remove only the occurrence on {formatPlainDate(series.occurrence_date, { weekday: 'short' })}, or every occurrence of &ldquo;{entry?.title}&rdquo;?
                </DialogDescription>
              </DialogHeader>
              <DialogFooter>
                <Button variant="outline" onClick={() => setConfirm(false)}>
                  Cancel
                </Button>
                <Button variant="outline" loading={del.isPending && del.variables === 'occurrence'} disabled={del.isPending} onClick={() => del.mutate('occurrence')}>
                  This occurrence
                </Button>
                <Button variant="destructive" loading={del.isPending && del.variables === 'series'} disabled={del.isPending} onClick={() => del.mutate('series')}>
                  All occurrences
                </Button>
              </DialogFooter>
            </DialogContent>
          </Dialog>
        ) : (
          <ConfirmDialog open={confirm} onOpenChange={setConfirm} title="Delete this event?" destructive confirmLabel="Delete" loading={del.isPending} onConfirm={() => del.mutate('series')} />
        )}
      </DialogContent>
    </Dialog>
  )
}

function EntryChip({ e, onEdit }: { e: CalendarEntry; onEdit?: (e: CalendarEntry) => void }) {
  const { timezone } = useAuth()
  const content = (
    <>
      <span className={cn('size-1.5 shrink-0 rounded-full', KIND_STYLE[e.kind].dot)} />
      {!e.all_day && <span className="shrink-0 text-muted-foreground tabular-nums">{formatTime(e.starts_at, timezone)}</span>}
      <span className="truncate">{e.title}</span>
      {e.series && <Repeat className="size-2.5 shrink-0 text-muted-foreground" aria-label="Repeats" />}
    </>
  )
  const cls = 'flex w-full items-center gap-1.5 rounded px-1 py-0.5 text-left text-[11px] leading-4 hover:bg-accent focus-visible:bg-accent focus-visible:outline-none'
  if (e.event_id && onEdit)
    return (
      <button type="button" className={cls} onClick={() => onEdit(e)} title={e.title}>
        {content}
      </button>
    )
  if (e.link)
    return (
      <Link to={e.link} className={cls} title={e.title}>
        {content}
      </Link>
    )
  return (
    <div className={cls} title={e.title}>
      {content}
    </div>
  )
}

export function CalendarPage() {
  useBreadcrumbs([{ label: 'Calendar' }])
  const { isAdmin, timezone } = useAuth()
  const courses = useCourses()
  const [month, setMonth] = React.useState(() => startOfMonth(new Date()))
  const [view, setView] = React.useState<'month' | 'agenda'>(() => (window.innerWidth < 640 ? 'agenda' : 'month'))
  const [courseId, setCourseId] = React.useState('')
  const [editing, setEditing] = React.useState<CalendarEntry | null>(null)
  const [creating, setCreating] = React.useState<string | null>(null)

  const gridStart = startOfWeek(startOfMonth(month), { weekStartsOn: 1 })
  const gridEnd = endOfWeek(endOfMonth(month), { weekStartsOn: 1 })
  const from = view === 'month' ? addDays(gridStart, -1) : startOfMonth(month)
  const to = view === 'month' ? addDays(gridEnd, 2) : addMonths(startOfMonth(month), 1)
  const q = useQuery({
    queryKey: ['calendar', from.toISOString(), to.toISOString(), courseId],
    queryFn: () => calendarApi.entries({ from: from.toISOString(), to: to.toISOString(), course_id: courseId || undefined }),
  })

  const byDay = React.useMemo(() => {
    const m = new Map<string, CalendarEntry[]>()
    for (const e of q.data ?? []) {
      const k = dayKey(e, timezone)
      m.set(k, [...(m.get(k) ?? []), e])
    }
    return m
  }, [q.data, timezone])

  const days: Date[] = []
  for (let d = gridStart; d <= gridEnd; d = addDays(d, 1)) days.push(d)
  const today = todayIn(timezone)
  const agenda = (q.data ?? []).filter((e) => dayKey(e, timezone).slice(0, 7) === format(month, 'yyyy-MM'))

  return (
    <>
      <PageHeader
        title="Calendar"
        description={`Classes, deadlines, sessions, laboratory activities and milestones · times in ${timezone}`}
        actions={
          <>
            <CourseSelect courses={courses.data ?? []} value={courseId} onChange={setCourseId} allowAll />
            {isAdmin && (
              <Button onClick={() => setCreating(today)}>
                <Plus /> New event
              </Button>
            )}
          </>
        }
      />
      <div className="mb-4 flex flex-wrap items-center justify-between gap-3">
        <div className="flex items-center gap-2">
          <Button variant="outline" size="icon-sm" onClick={() => setMonth((m) => addMonths(m, -1))} aria-label="Previous month">
            <ChevronLeft />
          </Button>
          <Button variant="outline" size="icon-sm" onClick={() => setMonth((m) => addMonths(m, 1))} aria-label="Next month">
            <ChevronRight />
          </Button>
          <h2 className="ml-1 text-base font-semibold">{format(month, 'MMMM yyyy')}</h2>
          <Button variant="ghost" size="sm" onClick={() => setMonth(startOfMonth(new Date()))}>
            Today
          </Button>
        </div>
        <Tabs value={view} onValueChange={(v) => setView(v as 'month' | 'agenda')}>
          <TabsList variant="segmented">
            <TabsTrigger value="month">Month</TabsTrigger>
            <TabsTrigger value="agenda">Agenda</TabsTrigger>
          </TabsList>
        </Tabs>
      </div>

      {q.error ? (
        <ErrorState error={q.error} onRetry={() => q.refetch()} />
      ) : view === 'month' ? (
        <div className="overflow-hidden rounded-xl border bg-card">
          <div className="grid grid-cols-7 border-b bg-muted/30 text-xs font-medium text-muted-foreground">
            {['Mon', 'Tue', 'Wed', 'Thu', 'Fri', 'Sat', 'Sun'].map((d) => (
              <div key={d} className="px-2 py-2">
                {d}
              </div>
            ))}
          </div>
          <div className="grid grid-cols-7">
            {days.map((d) => {
              const key = format(d, 'yyyy-MM-dd')
              const items = byDay.get(key) ?? []
              const inMonth = d.getMonth() === month.getMonth()
              return (
                <div
                  key={key}
                  className={cn('group min-h-24 border-r border-b p-1 last:border-r-0 sm:min-h-28 [&:nth-child(7n)]:border-r-0', !inMonth && 'bg-muted/20 text-muted-foreground')}
                  onDoubleClick={() => isAdmin && setCreating(key)}
                >
                  <div className="mb-0.5 flex items-center justify-between px-1">
                    <span className={cn('flex size-6 items-center justify-center rounded-full text-xs tabular-nums', key === today && 'bg-primary font-semibold text-primary-foreground')}>{d.getDate()}</span>
                    {isAdmin && (
                      <button type="button" onClick={() => setCreating(key)} className="rounded p-0.5 text-muted-foreground opacity-0 group-hover:opacity-100 hover:bg-accent focus-visible:opacity-100" aria-label={`Add event on ${key}`}>
                        <Plus className="size-3" />
                      </button>
                    )}
                  </div>
                  <div className="space-y-0.5">
                    {q.isLoading ? null : items.slice(0, 3).map((e) => <EntryChip key={e.id} e={e} onEdit={isAdmin ? setEditing : undefined} />)}
                    {items.length > 3 && <div className="px-1 text-[11px] text-muted-foreground">+{items.length - 3} more</div>}
                  </div>
                </div>
              )
            })}
          </div>
        </div>
      ) : q.isLoading ? (
        <Skeleton className="h-64 rounded-xl" />
      ) : agenda.length === 0 ? (
        <EmptyState icon={CalendarDays} title="Nothing scheduled this month" />
      ) : (
        <div className="space-y-4">
          {Array.from(new Set(agenda.map((e) => dayKey(e, timezone)))).map((day) => (
            <div key={day} className="grid gap-2 sm:grid-cols-[9rem_1fr]">
              <div className={cn('pt-2 text-sm font-medium', day === today && 'text-primary')}>{formatPlainDate(day, { weekday: 'short', day: 'numeric', month: 'short' })}</div>
              <ul className="divide-y rounded-xl border bg-card">
                {agenda
                  .filter((e) => dayKey(e, timezone) === day)
                  .map((e) => (
                    <li key={e.id} className="flex items-center gap-3 px-4 py-3">
                      <span className="w-12 shrink-0 text-xs text-muted-foreground tabular-nums">{e.all_day ? 'All day' : formatTime(e.starts_at, timezone)}</span>
                      <div className="min-w-0 flex-1">
                        {e.link ? (
                          <Link to={e.link} className="truncate text-sm font-medium hover:underline">
                            {e.title}
                          </Link>
                        ) : (
                          <div className="truncate text-sm font-medium">{e.title}</div>
                        )}
                        <div className="truncate text-xs text-muted-foreground">
                          {e.course_title}
                          {e.series && ` · ${describeRecurrence(e.series.recurrence)}`}
                          {e.location && ` · ${e.location}`}
                          {e.description && ` · ${e.description}`}
                        </div>
                      </div>
                      <Badge variant={KIND_STYLE[e.kind].badge}>{CALENDAR_KIND_LABELS[e.kind]}</Badge>
                      {isAdmin && e.event_id && (
                        <Button variant="ghost" size="sm" onClick={() => setEditing(e)}>
                          Edit
                        </Button>
                      )}
                    </li>
                  ))}
              </ul>
            </div>
          ))}
        </div>
      )}
      <div className="mt-4 flex flex-wrap gap-x-4 gap-y-1 text-xs text-muted-foreground">
        {(['class', 'deadline', 'session', 'lab', 'milestone', 'week_start'] as const).map((k) => (
          <span key={k} className="flex items-center gap-1.5">
            <span className={cn('size-2 rounded-full', KIND_STYLE[k].dot)} /> {CALENDAR_KIND_LABELS[k]}
          </span>
        ))}
      </div>
      {isAdmin && (
        <EventDialog
          open={!!editing || !!creating}
          onOpenChange={(o) => {
            if (!o) {
              setEditing(null)
              setCreating(null)
            }
          }}
          entry={editing}
          defaultDate={creating ?? undefined}
        />
      )}
    </>
  )
}
