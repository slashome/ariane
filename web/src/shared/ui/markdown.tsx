import type { ReactNode } from 'react'
import ReactMarkdown, { type Components } from 'react-markdown'
import remarkGfm from 'remark-gfm'
import { cn } from '@/shared/lib'

const plugins = [remarkGfm]

// Renders an item body. `renderLink` decides what a link becomes: a route to another item, an external
// link, or plain text; without it, links are left to the browser.
export function Markdown({
  source,
  renderLink,
  className,
}: {
  source: string
  renderLink?: (href: string, children: ReactNode) => ReactNode
  className?: string
}) {
  const components: Components = renderLink
    ? { a: ({ href, children }) => <>{renderLink(href ?? '', children)}</> }
    : {}
  return (
    <div className={cn('markdown', className)}>
      <ReactMarkdown remarkPlugins={plugins} components={components}>
        {source}
      </ReactMarkdown>
    </div>
  )
}
