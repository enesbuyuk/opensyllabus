import { ExternalLink, PlayCircle } from 'lucide-react'

/** Converts YouTube/Vimeo URLs to privacy-friendly embed URLs; other links are shown as links. */
export function embedUrl(url: string): string | null {
  try {
    const u = new URL(url)
    const host = u.hostname.replace(/^www\./, '')
    if (host === 'youtube.com' || host === 'm.youtube.com') {
      const id = u.searchParams.get('v') ?? (u.pathname.startsWith('/embed/') ? u.pathname.split('/')[2] : null)
      return id ? `https://www.youtube-nocookie.com/embed/${encodeURIComponent(id)}` : null
    }
    if (host === 'youtu.be') return `https://www.youtube-nocookie.com/embed/${encodeURIComponent(u.pathname.slice(1))}`
    if (host === 'vimeo.com') {
      const id = u.pathname.split('/').filter(Boolean)[0]
      return id && /^\d+$/.test(id) ? `https://player.vimeo.com/video/${id}` : null
    }
  } catch {
    return null
  }
  return null
}

export function VideoEmbed({ url, title }: { url: string; title: string }) {
  const src = embedUrl(url)
  if (!src) {
    return (
      <a href={url} target="_blank" rel="noopener noreferrer" className="flex items-center gap-3 rounded-xl border bg-muted/30 p-4 text-sm hover:bg-muted/60">
        <PlayCircle className="size-5 text-primary" />
        <span className="flex-1 truncate">Watch video</span>
        <ExternalLink className="size-4 text-muted-foreground" />
      </a>
    )
  }
  return (
    <div className="aspect-video overflow-hidden rounded-xl border bg-black">
      <iframe
        src={src}
        title={title}
        className="size-full"
        allow="accelerometer; clipboard-write; encrypted-media; gyroscope; picture-in-picture; fullscreen"
        referrerPolicy="strict-origin-when-cross-origin"
        allowFullScreen
      />
    </div>
  )
}
