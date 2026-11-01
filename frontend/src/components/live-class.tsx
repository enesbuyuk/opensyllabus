import { CalendarClock, ExternalLink, Video } from 'lucide-react'
import { Link } from 'react-router'
import { Badge } from '@/components/ui/badge'
import { Button } from '@/components/ui/button'
import { cn, formatDateTime, formatTime } from '@/lib/utils'

/** Minutes before the start from which a class counts as "starting soon / live". */
const JOIN_EARLY_MS = 15 * 60_000
/** Classes without an end time are treated as lasting this long. */
const DEFAULT_LENGTH_MS = 2 * 60 * 60_000

export function classPhase(startsAt: string | null, endsAt: string | null, now = Date.now()): 'upcoming' | 'live' | 'ended' | 'unscheduled' {
  if (!startsAt) return 'unscheduled'
  const start = new Date(startsAt).getTime()
  const end = endsAt ? new Date(endsAt).getTime() : start + DEFAULT_LENGTH_MS
  if (now > end) return 'ended'
  if (now >= start - JOIN_EARLY_MS) return 'live'
  return 'upcoming'
}

/** True for links the app opens: https only, so a stored value can never run script. */
export const isSafeMeetingUrl = (url: string | null | undefined): url is string => !!url && /^https:\/\/\S+$/i.test(url)

/** Hostname shown next to the join button, e.g. "zoom.us" or "meet.google.com". */
function host(url: string) {
  try {
    return new URL(url).hostname.replace(/^www\./, '')
  } catch {
    return ''
  }
}

export function JoinClassButton({ url, size = 'default', className }: { url: string; size?: 'default' | 'sm'; className?: string }) {
  return (
    <Button asChild size={size} className={className}>
      <a href={url} target="_blank" rel="noopener noreferrer">
        <Video /> Join class <ExternalLink className="opacity-70" />
      </a>
    </Button>
  )
}

/** Live class banner: when the class meets and a button to join it online. */
export function LiveClassCard({
  url,
  startsAt,
  endsAt,
  timezone,
  title,
  lessonLink,
  className,
}: {
  url: string | null
  startsAt: string | null
  endsAt: string | null
  timezone: string
  title?: string
  lessonLink?: string
  className?: string
}) {
  const phase = classPhase(startsAt, endsAt)
  if (!isSafeMeetingUrl(url) && phase === 'unscheduled') return null
  const when = startsAt ? `${formatDateTime(startsAt, timezone, { weekday: 'short' })}${endsAt ? `–${formatTime(endsAt, timezone)}` : ''}` : null
  return (
    <div className={cn('flex flex-col gap-4 rounded-xl border bg-card p-5 shadow-xs sm:flex-row sm:items-center', phase === 'live' && 'border-primary/50 bg-primary-soft', className)}>
      <span className="flex size-11 shrink-0 items-center justify-center rounded-full bg-primary-soft text-primary">
        <CalendarClock className="size-5" />
      </span>
      <div className="min-w-0 flex-1">
        <div className="flex flex-wrap items-center gap-2">
          <span className="text-base font-semibold">{title ?? 'Live class'}</span>
          {phase === 'live' && <Badge>Live now</Badge>}
          {phase === 'ended' && <Badge variant="neutral">Ended</Badge>}
        </div>
        <div className="mt-0.5 text-[13px] text-muted-foreground">
          {when ?? 'Time not scheduled yet'}
          {isSafeMeetingUrl(url) && host(url) && <> · {host(url)}</>}
          {lessonLink && (
            <>
              {' · '}
              <Link to={lessonLink} className="hover:text-foreground hover:underline">
                Open lesson
              </Link>
            </>
          )}
        </div>
      </div>
      {isSafeMeetingUrl(url) ? (
        <JoinClassButton url={url} className="shrink-0" />
      ) : (
        <span className="text-[13px] text-muted-foreground">No class link yet</span>
      )}
    </div>
  )
}
