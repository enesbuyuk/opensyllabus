import { formatScore, isoToWallTime, wallTimeToIso } from '@/lib/utils'
import { embedUrl } from '@/components/video-embed'

describe('timezone conversion', () => {
  it('converts Istanbul wall time to UTC and back', () => {
    const iso = wallTimeToIso('2026-03-01T12:00', 'Europe/Istanbul')
    expect(iso).toBe('2026-03-01T09:00:00.000Z')
    expect(isoToWallTime(iso, 'Europe/Istanbul')).toBe('2026-03-01T12:00')
  })

  it('handles daylight saving time zones', () => {
    expect(wallTimeToIso('2026-07-01T12:00', 'Europe/Berlin')).toBe('2026-07-01T10:00:00.000Z')
    expect(wallTimeToIso('2026-01-15T12:00', 'Europe/Berlin')).toBe('2026-01-15T11:00:00.000Z')
  })
})

describe('formatting', () => {
  it('formats scores', () => {
    expect(formatScore(9, 10)).toBe('9 / 10')
    expect(formatScore(7.25)).toBe('7.3')
    expect(formatScore(null)).toBe('—')
  })
})

describe('video embeds', () => {
  it('uses privacy-friendly embeds for known hosts only', () => {
    expect(embedUrl('https://www.youtube.com/watch?v=abc123')).toBe('https://www.youtube-nocookie.com/embed/abc123')
    expect(embedUrl('https://youtu.be/xyz')).toBe('https://www.youtube-nocookie.com/embed/xyz')
    expect(embedUrl('https://vimeo.com/12345')).toBe('https://player.vimeo.com/video/12345')
    expect(embedUrl('https://evil.example/video')).toBeNull()
  })
})
