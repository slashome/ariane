import { ExternalLink, Library, Search } from 'lucide-react'
import { useMemo } from 'react'
import { compareNatural, excerpt, ItemLink, useItemsWithBody } from '@/entities/item'
import { emptyFilters, filterItems, useItemFilters } from '@/features/filter-items'
import { displayNode } from '@/shared/lib'
import { EmptyState, ErrorState, Input, LoadingState, PageHeader } from '@/shared/ui'
import { linksOf, snippet } from '../model/library'

export function RessourcesPage() {
  const { data: items = [], isPending, error } = useItemsWithBody('ressource')
  const { filters, setQuery } = useItemFilters()
  const ressources = useMemo(
    () =>
      filterItems(items, { ...emptyFilters, node: filters.node, includeArchived: filters.includeArchived, query: filters.query }).sort(
        (left, right) => compareNatural(left.title ?? '', right.title ?? ''),
      ),
    [items, filters],
  )
  const total = filterItems(items, { ...emptyFilters, node: filters.node, includeArchived: filters.includeArchived }).length

  return (
    <div className="flex flex-col gap-5 p-6">
      <PageHeader
        title="Ressources"
        description={isPending ? 'Loading…' : filters.query ? `${ressources.length} of ${total} match “${filters.query}”` : `${total} references kept for later`}
        actions={
          <div className="relative w-80">
            <Search className="pointer-events-none absolute top-1/2 left-2.5 size-4 -translate-y-1/2 text-muted-foreground" aria-hidden />
            <Input
              type="search"
              aria-label="Search the ressources, bodies included"
              placeholder="Search titles and full text…"
              className="pl-8"
              value={filters.query}
              onChange={(event) => setQuery(event.target.value)}
            />
          </div>
        }
      />
      {error ? (
        <ErrorState message={`Could not load the ressources: ${error.message}`} />
      ) : isPending ? (
        <LoadingState rows={3} />
      ) : ressources.length === 0 ? (
        <EmptyState icon={Library} title={total === 0 ? 'No ressource here' : 'No ressource matches'}>
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
            const match = snippet(item.body, filters.query)
            return (
              <li key={item.path} className="flex flex-col gap-3 rounded-lg border border-border bg-background p-4 shadow-xs">
                <div className="flex items-start gap-3">
                  <span className="flex size-9 shrink-0 items-center justify-center rounded-md bg-muted text-sm font-semibold text-muted-foreground uppercase" aria-hidden>
                    {(links[0]?.host ?? item.title ?? '?').slice(0, 1)}
                  </span>
                  <div className="min-w-0">
                    <h2 className="leading-snug font-medium">
                      <ItemLink item={item}>{item.title ?? item.path}</ItemLink>
                    </h2>
                    <p className="font-mono text-[10px] text-muted-foreground">
                      {item.id} · {displayNode(item.node)}
                    </p>
                  </div>
                </div>
                <p className="line-clamp-4 text-sm leading-relaxed text-muted-foreground">{match ?? excerpt(item.body)}</p>
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
