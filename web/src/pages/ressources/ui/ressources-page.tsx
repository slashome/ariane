import { useMemo } from 'react'
import { compareNatural, excerpt, ItemLink, useItemsWithBody } from '@/entities/item'
import { QueryField, useFilteredItems } from '@/features/filter-items'
import { displayNode } from '@/shared/lib'
import { EmptyState, ErrorState, Highlight, Icon, LoadingState, PageHeader, useHighlightTerms } from '@/shared/ui'
import { linksOf, snippet } from '../model/library'
import './ressources.css'

export function RessourcesPage() {
  const { data: items = [], isPending, error } = useItemsWithBody('ressource')
  const terms = useHighlightTerms()
  const { items: found, total } = useFilteredItems(items)
  const ressources = useMemo(() => [...found].sort((left, right) => compareNatural(left.title ?? '', right.title ?? '')), [found])

  return (
    <div className="ressources-page">
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
        <ul className="ressources-page__list">
          {ressources.map((item) => {
            const links = linksOf(item)
            const match = snippet(item.body, terms)
            return (
              <li key={item.path} className="ressource-card">
                <div className="ressource-card__header">
                  <span className="ressource-card__initial" aria-hidden>
                    {(links[0]?.host ?? item.title ?? '?').slice(0, 1)}
                  </span>
                  <div className="ressource-card__heading">
                    <h2 className="ressource-card__title">
                      <ItemLink item={item}>
                        <Highlight text={item.title ?? item.path} />
                      </ItemLink>
                    </h2>
                    <p className="ressource-card__meta">
                      <Highlight text={item.id ?? ''} /> · {displayNode(item.node)}
                    </p>
                  </div>
                </div>
                <p className="lu-line-clamp ressource-card__excerpt">
                  <Highlight text={match ?? excerpt(item.body)} />
                </p>
                {links.length > 0 && (
                  <ul className="ressource-card__links">
                    {links.slice(0, 4).map((link) => (
                      <li key={link.url}>
                        <a
                          href={link.url}
                          target="_blank"
                          rel="noreferrer"
                          title={link.url}
                          className="ressource-card__link"
                        >
                          {link.host}
                          <Icon name="external-link" className="ressource-card__link-icon" />
                        </a>
                      </li>
                    ))}
                    {links.length > 4 && <li className="ressource-card__more">+{links.length - 4}</li>}
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
