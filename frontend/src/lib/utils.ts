import { clsx, type ClassValue } from 'clsx'
import { twMerge } from 'tailwind-merge'

export function cn(...inputs: ClassValue[]) {
  return twMerge(clsx(inputs))
}

export const DEFAULT_TIMEZONE = 'UTC'

function safeDate(iso: string | null | undefined): Date | null {
  if (!iso) return null
  const d = new Date(iso)
  return Number.isNaN(d.getTime()) ? null : d
}

/** Formats a UTC timestamp in the user's timezone. */
export function formatDateTime(iso: string | null | undefined, tz = DEFAULT_TIMEZONE, opts: Intl.DateTimeFormatOptions = {}) {
  const d = safeDate(iso)
  if (!d) return '—'
  return new Intl.DateTimeFormat('en-GB', {
    timeZone: tz,
    day: 'numeric',
    month: 'short',
    year: 'numeric',
    hour: '2-digit',
    minute: '2-digit',
    ...opts,
  }).format(d)
}

export function formatDate(iso: string | null | undefined, tz = DEFAULT_TIMEZONE) {
  const d = safeDate(iso)
  if (!d) return '—'
  return new Intl.DateTimeFormat('en-GB', { timeZone: tz, day: 'numeric', month: 'short', year: 'numeric' }).format(d)
}

/** Formats a calendar date (YYYY-MM-DD) without timezone shifts. */
export function formatPlainDate(date: string | null | undefined, opts: Intl.DateTimeFormatOptions = {}) {
  if (!date) return '—'
  const [y, m, d] = date.split('-').map(Number)
  return new Intl.DateTimeFormat('en-GB', { timeZone: 'UTC', day: 'numeric', month: 'short', year: 'numeric', ...opts }).format(
    new Date(Date.UTC(y, m - 1, d)),
  )
}

export function formatTime(iso: string, tz = DEFAULT_TIMEZONE) {
  const d = safeDate(iso)
  if (!d) return ''
  return new Intl.DateTimeFormat('en-GB', { timeZone: tz, hour: '2-digit', minute: '2-digit' }).format(d)
}

export function relativeTime(iso: string | null | undefined, now = Date.now()) {
  const d = safeDate(iso)
  if (!d) return '—'
  const diff = d.getTime() - now
  const abs = Math.abs(diff)
  const rtf = new Intl.RelativeTimeFormat('en', { numeric: 'auto' })
  const units: [Intl.RelativeTimeFormatUnit, number][] = [
    ['year', 365 * 864e5],
    ['month', 30 * 864e5],
    ['week', 7 * 864e5],
    ['day', 864e5],
    ['hour', 36e5],
    ['minute', 6e4],
  ]
  for (const [unit, ms] of units) {
    if (abs >= ms) return rtf.format(Math.round(diff / ms), unit)
  }
  return 'just now'
}

/** Date parts of an instant as seen in a timezone. */
export function zonedParts(date: Date, tz: string) {
  const parts = new Intl.DateTimeFormat('en-US', {
    timeZone: tz,
    hourCycle: 'h23',
    year: 'numeric',
    month: '2-digit',
    day: '2-digit',
    hour: '2-digit',
    minute: '2-digit',
    second: '2-digit',
  }).formatToParts(date)
  const get = (t: string) => Number(parts.find((p) => p.type === t)?.value ?? 0)
  return { year: get('year'), month: get('month'), day: get('day'), hour: get('hour') % 24, minute: get('minute'), second: get('second') }
}

function tzOffsetMs(date: Date, tz: string) {
  const p = zonedParts(date, tz)
  const asUtc = Date.UTC(p.year, p.month - 1, p.day, p.hour, p.minute, p.second)
  return asUtc - Math.floor(date.getTime() / 1000) * 1000
}

/** Converts a wall-clock value ("YYYY-MM-DDTHH:mm") in `tz` into a UTC ISO string. */
export function wallTimeToIso(wall: string, tz = DEFAULT_TIMEZONE): string {
  if (!wall) return ''
  const [date, time = '00:00'] = wall.split('T')
  const [y, m, d] = date.split('-').map(Number)
  const [hh, mm] = time.split(':').map(Number)
  const guess = Date.UTC(y, m - 1, d, hh, mm)
  let ts = guess - tzOffsetMs(new Date(guess), tz)
  // Second pass handles DST transitions.
  ts = guess - tzOffsetMs(new Date(ts), tz)
  return new Date(ts).toISOString()
}

/** Converts a UTC ISO timestamp to a wall-clock "YYYY-MM-DDTHH:mm" value in `tz` (for datetime-local inputs). */
export function isoToWallTime(iso: string | null | undefined, tz = DEFAULT_TIMEZONE): string {
  const d = safeDate(iso)
  if (!d) return ''
  const p = zonedParts(d, tz)
  const pad = (n: number) => String(n).padStart(2, '0')
  return `${p.year}-${pad(p.month)}-${pad(p.day)}T${pad(p.hour)}:${pad(p.minute)}`
}

/** Today's date (YYYY-MM-DD) in a timezone. */
export function todayIn(tz = DEFAULT_TIMEZONE) {
  const p = zonedParts(new Date(), tz)
  return `${p.year}-${String(p.month).padStart(2, '0')}-${String(p.day).padStart(2, '0')}`
}

export function formatBytes(bytes: number | null | undefined) {
  if (bytes == null) return ''
  if (bytes < 1024) return `${bytes} B`
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`
  return `${(bytes / (1024 * 1024)).toFixed(1)} MB`
}

export function formatMinutes(minutes: number) {
  if (!minutes) return '—'
  const h = Math.floor(minutes / 60)
  const m = minutes % 60
  if (!h) return `${m} min`
  return m ? `${h} h ${m} min` : `${h} h`
}

export function formatScore(score: number | null | undefined, max?: number) {
  if (score == null) return '—'
  const s = Number.isInteger(score) ? String(score) : score.toFixed(1)
  return max != null ? `${s} / ${Number.isInteger(max) ? max : max.toFixed(1)}` : s
}

export function initials(name: string) {
  return name
    .split(/\s+/)
    .filter(Boolean)
    .slice(0, 2)
    .map((p) => p[0]?.toUpperCase())
    .join('')
}

export function isPast(iso: string | null | undefined) {
  const d = safeDate(iso)
  return d ? d.getTime() < Date.now() : false
}

export const TIMEZONES: string[] = (() => {
  try {
    const fn = (Intl as unknown as { supportedValuesOf?: (k: string) => string[] }).supportedValuesOf
    if (fn) return fn('timeZone')
  } catch {
    /* fall through */
  }
  return ['UTC', 'Europe/Istanbul', 'Europe/London', 'Europe/Berlin', 'America/New_York', 'Asia/Tokyo']
})()
