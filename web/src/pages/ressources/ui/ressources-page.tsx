import { ExternalLink } from 'lucide-react'
import { useMemo } from 'react'
import { compareNatural, excerpt, ItemLink, useItemsWithBody } from '@/entities/item'
import { QueryField, useFilteredItems } from '@/features/filter-items'
import { displayNode } from '@/shared/lib'
import { EmptyState, ErrorState, Highlight, LoadingState, PageHeader, useHighlightTerms } from '@/shared/ui'
import { linksOf, snippet } from '../model/library'

export function RessourcesPage() {
  const { data: items = [], isPending, error } = useItemsWithBody('ressource')
  const terms = useHighlightTerms()
  const { items: found, total } = useFilteredItems(items)
  const ressources = useMemo(() => [...found].sort((left, right) => compareNatural(left.title ?? '', right.title ?? '')), [found])

  return (
    <div className="flex flex-col gap-5 p-6">
      <PageHeader
        title="Ressources"
        description={isPending ? 'Loading…' : `${total} references kept for later`}
        actions={<QueryField count={ressources.length} total={total} />}
      />
      {error ? (
        <ErrorState message={`Could not load the ressources: ${error.message}`} />
      ) : isPending ? (
        <LoadingState rows={3} />
      ) : ressources.length === 0 ? (
        <EmptyState icon="library" title={total === 0 ? 'No ressource here' : 'No ressource matches'}>
          {total === 0 ? (
            <>
              A ressource is a Markdown file under <code>ressources/</code>: a site, a book, a repository to come back to.
            </>
          ) : (
            'The search covers titles, ids and the full text of each ressource.'
          )}
        </EmptyState>
      ) : (
        <ul className="grid grid-cols-[repeat(auto-fill,minmax(18rem,1fr))] gap-4">
          {ressources.map((item) => {
            const links = linksOf(item)
            const match = snippet(item.body, terms)
            return (
              <li key={item.path} className="flex flex-col gap-3 rounded-lg border border-border bg-background p-4 shadow-xs">
                <div className="flex items-start gap-3">
                  <span className="flex size-9 shrink-0 items-center justify-center rounded-md bg-muted text-sm font-semibold text-muted-foreground uppercase" aria-hidden>
                    {(links[0]?.host ?? item.title ?? '?').slice(0, 1)}
                  </span>
                  <div className="min-w-0">
                    <h2 className="leading-snug font-medium">
                      <ItemLink item={item}>
                        <Highlight text={item.title ?? item.path} />
                      </ItemLink>
                    </h2>
                    <p className="font-mono text-[10px] text-muted-foreground">
                      <Highlight text={item.id ?? ''} /> · {displayNode(item.node)}
                    </p>
                  </div>
                </div>
                <p className="line-clamp-4 text-sm leading-relaxed text-muted-foreground">
                  <Highlight text={match ?? excerpt(item.body)} />
                </p>
                {links.length > 0 && (
                  <ul className="mt-auto flex flex-wrap gap-1.5">
                    {links.slice(0, 4).map((link) => (
                      <li key={link.url}>
                        <a
                          href={link.url}
                          target="_blank"
                          rel="noreferrer"
                          title={link.url}
                          className="inline-flex items-center gap-1 rounded-full border border-border px-2 py-0.5 text-xs text-primary hover:bg-muted"
                        >
                          {link.host}
                          <ExternalLink className="size-3" aria-hidden />
                        </a>
                      </li>
                    ))}
                    {links.length > 4 && <li className="text-xs text-muted-foreground">+{links.length - 4}</li>}
                  </ul>
                )}
              </li>
            )
          })}
        </ul>
      )}
    </div>
  )
}
