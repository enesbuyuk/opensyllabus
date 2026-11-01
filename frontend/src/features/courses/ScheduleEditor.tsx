import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query'
import { Link } from 'react-router'
import * as React from 'react'
import { toast } from 'sonner'
import { ApiError } from '@/api/client'
import { coursesApi } from '@/api/endpoints'
import { Field } from '@/components/form'
import { ErrorState } from '@/components/page'
import { Button } from '@/components/ui/button'
import { Card, CardContent, CardDescription, CardFooter, CardHeader, CardTitle } from '@/components/ui/card'
import { Input, NativeSelect } from '@/components/ui/input'
import { Checkbox, Skeleton } from '@/components/ui/misc'
import { useAuth } from '@/lib/auth'
import { errorMessage } from '@/lib/errors'
import { WEEKDAYS } from '@/lib/labels'
import { formatPlainDate, TIMEZONES } from '@/lib/utils'
import type { CourseListItem, CourseSchedule, ScheduleSlot } from '@/types/api'

type Row = { enabled: boolean; weekday: number; start_time: string; end_time: string; location: string; meeting_url: string }

/** Default weekday for course day N: spread over Mon/Wed/Fri/... like a typical timetable. */
const DEFAULT_WEEKDAYS = [1, 3, 5, 2, 4, 6, 7]

function rowsFrom(days: number, slots: ScheduleSlot[]): Row[] {
  return Array.from({ length: days }, (_, i) => {
    const s = slots.find((x) => x.day_number === i + 1)
    return s
      ? { enabled: true, weekday: s.weekday, start_time: s.start_time, end_time: s.end_time ?? '', location: s.location, meeting_url: s.meeting_url ?? '' }
      : { enabled: false, weekday: DEFAULT_WEEKDAYS[i], start_time: '10:00', end_time: '12:00', location: '', meeting_url: '' }
  })
}

function rowError(r: Row): string | null {
  if (!r.enabled) return null
  if (!r.start_time) return 'Enter a start time'
  if (r.end_time && r.end_time <= r.start_time) return 'End must be after the start'
  if (r.meeting_url.trim() && !/^https:\/\/\S+$/.test(r.meeting_url.trim())) return 'Class link must start with https://'
  return null
}

/** Weekly class times: course day N (the "Day N" lessons of every week) meets on a weekday at a time. */
export function ScheduleEditor({ course }: { course: CourseListItem }) {
  const { timezone: userTz } = useAuth()
  const qc = useQueryClient()
  const q = useQuery({ queryKey: ['schedule', course.id], queryFn: () => coursesApi.schedule(course.id) })
  const [rows, setRows] = React.useState<Row[]>([])
  const [tz, setTz] = React.useState(userTz)
  const [serverErrors, setServerErrors] = React.useState<Record<string, string>>({})
  const [dirty, setDirty] = React.useState(false)

  React.useEffect(() => {
    if (!q.data) return
    setRows(rowsFrom(course.days_per_week, q.data.slots))
    setTz(q.data.slots.length ? q.data.timezone : userTz)
    setDirty(false)
  }, [q.data, course.days_per_week, userTz])

  const save = useMutation({
    mutationFn: (body: CourseSchedule) => coursesApi.updateSchedule(course.id, body),
    onSuccess: (data) => {
      qc.setQueryData(['schedule', course.id], data)
      qc.invalidateQueries({ queryKey: ['calendar'] })
      setServerErrors({})
      toast.success(data.slots.length ? 'Class schedule saved' : 'Class schedule cleared')
    },
    onError: (e) => {
      if (e instanceof ApiError) setServerErrors(e.fields)
      toast.error(errorMessage(e))
    },
  })

  const update = (i: number, patch: Partial<Row>) => {
    setRows((rs) => rs.map((r, j) => (j === i ? { ...r, ...patch } : r)))
    setDirty(true)
  }
  const invalid = rows.some((r) => rowError(r))
  const onSave = () =>
    save.mutate({
      timezone: tz,
      slots: rows.flatMap((r, i) =>
        r.enabled ? [{ day_number: i + 1, weekday: r.weekday, start_time: r.start_time, end_time: r.end_time || null, location: r.location.trim(), meeting_url: r.meeting_url.trim() || null }] : [],
      ),
    })

  if (q.error) return <ErrorState error={q.error} onRetry={() => q.refetch()} />
  return (
    <Card>
      <CardHeader>
        <CardTitle>Class schedule</CardTitle>
        <CardDescription>
          When each course day meets. The &ldquo;Day N&rdquo; lessons of every week appear on the calendar at these times, starting{' '}
          {course.start_date ? formatPlainDate(course.start_date, { weekday: 'short' }) : 'from the course start date'}.
        </CardDescription>
      </CardHeader>
      <CardContent className="space-y-4">
        {!course.start_date && (
          <p className="rounded-md border border-warning/30 bg-warning/10 px-3 py-2 text-[13px]">
            Set a start date in <Link to="?tab=details" className="font-medium underline">Details</Link> to place classes on the calendar.
          </p>
        )}
        {q.isLoading ? (
          <Skeleton className="h-48" />
        ) : (
          <div className="space-y-3">
            {rows.map((r, i) => {
              const day = i + 1
              const err = rowError(r) ?? serverErrors[`slots.${day}`]
              return (
                <fieldset key={day} className="grid items-start gap-3 rounded-lg border p-3 sm:grid-cols-[7rem_9rem_1fr_1fr_2fr]" aria-label={`Day ${day}`}>
                  <label className="flex h-9 items-center gap-2 text-sm font-medium">
                    <Checkbox checked={r.enabled} onCheckedChange={(v) => update(i, { enabled: v === true })} aria-label={`Schedule day ${day}`} />
                    Day {day}
                  </label>
                  <Field label="Weekday" htmlFor={`sd-wd-${day}`}>
                    <NativeSelect id={`sd-wd-${day}`} value={r.weekday} disabled={!r.enabled} onChange={(e) => update(i, { weekday: Number(e.target.value) })}>
                      {WEEKDAYS.map((w) => (
                        <option key={w.n} value={w.n}>
                          {w.long}
                        </option>
                      ))}
                    </NativeSelect>
                  </Field>
                  <Field label="Starts" htmlFor={`sd-start-${day}`}>
                    <Input id={`sd-start-${day}`} type="time" value={r.start_time} disabled={!r.enabled} onChange={(e) => update(i, { start_time: e.target.value })} />
                  </Field>
                  <Field label="Ends" htmlFor={`sd-end-${day}`}>
                    <Input id={`sd-end-${day}`} type="time" value={r.end_time} disabled={!r.enabled} onChange={(e) => update(i, { end_time: e.target.value })} />
                  </Field>
                  <Field label="Location" htmlFor={`sd-loc-${day}`}>
                    <Input id={`sd-loc-${day}`} value={r.location} disabled={!r.enabled} placeholder="e.g. Electronics Lab 2" maxLength={200} onChange={(e) => update(i, { location: e.target.value })} />
                  </Field>
                  <Field
                    label="Class link"
                    htmlFor={`sd-url-${day}`}
                    hint="Zoom, Meet or Teams link students use to join this class. A lesson can override it."
                    className="sm:col-span-4 sm:col-start-2"
                  >
                    <Input
                      id={`sd-url-${day}`}
                      type="url"
                      inputMode="url"
                      value={r.meeting_url}
                      disabled={!r.enabled}
                      placeholder="Paste a Zoom, Meet or Teams link"
                      maxLength={2048}
                      onChange={(e) => update(i, { meeting_url: e.target.value })}
                    />
                  </Field>
                  {err && (
                    <p role="alert" className="text-xs text-destructive sm:col-span-5">
                      {err}
                    </p>
                  )}
                </fieldset>
              )
            })}
          </div>
        )}
        <Field label="Timezone" htmlFor="sd-tz" hint="Classes keep these local times, also across daylight-saving changes." className="max-w-sm">
          <NativeSelect
            id="sd-tz"
            value={tz}
            onChange={(e) => {
              setTz(e.target.value)
              setDirty(true)
            }}
          >
            {TIMEZONES.map((t) => (
              <option key={t} value={t}>
                {t.replace(/_/g, ' ')}
              </option>
            ))}
          </NativeSelect>
        </Field>
      </CardContent>
      <CardFooter className="justify-end">
        <Button onClick={onSave} loading={save.isPending} disabled={!dirty || invalid}>
          Save schedule
        </Button>
      </CardFooter>
    </Card>
  )
}
