import ReactMarkdown, { type Components } from 'react-markdown'
import rehypeHighlight from 'rehype-highlight'
import rehypeSanitize from 'rehype-sanitize'
import remarkGfm from 'remark-gfm'
import { cn } from '@/lib/utils'

const components: Components = {
  a: ({ href, children, ...props }) => {
    const external = !!href && /^https?:\/\//.test(href)
    return (
      <a href={href} {...props} {...(external ? { target: '_blank', rel: 'noopener noreferrer nofollow' } : {})}>
        {children}
      </a>
    )
  },
}

/**
 * Safe Markdown renderer: raw HTML is never rendered, output is sanitized,
 * then code blocks are syntax-highlighted.
 */
export function Markdown({ children, className }: { children: string; className?: string }) {
  if (!children?.trim()) return null
  return (
    <div className={cn('prose-opensyllabus', className)}>
      <ReactMarkdown remarkPlugins={[remarkGfm]} rehypePlugins={[rehypeSanitize, [rehypeHighlight, { detect: true, ignoreMissing: true }]]} components={components}>
        {children}
      </ReactMarkdown>
    </div>
  )
}
