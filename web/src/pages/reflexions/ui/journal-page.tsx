import { useMemo } from 'react'
import { Link, useLocation } from 'react-router'
import { excerpt, ItemLink } from '@/entities/item'
import { QueryField } from '@/features/filter-items'
import { routes, scopeSearch } from '@/shared/config'
import { displayNode } from '@/shared/lib'
import { EmptyState, ErrorState, Highlight, LoadingState, PageHeader } from '@/shared/ui'
import { journal } from '../model/reflexions'
import { useReflexions } from './use-reflexions'
import './reflexions.css'

export function JournalPage() {
  const { reflexions, total, isPending, error, scope } = useReflexions()
  const { search } = useLocation()
  const months = useMemo(() => journal(reflexions), [reflexions])
  const trial = reflexions.filter((item) => item.status === 'trial')
  const settled = months.reduce((sum, month) => sum + month.items.length, 0)

  return (
    <div className="journal">
      <PageHeader
        title="Journal"
        description={isPending ? 'Loading…' : `${settled} settled reflexions, dated by their latest commit · ${trial.length} on trial`}
        actions={
          <>
            <QueryField count={reflexions.length} total={total} />
            <Link to={{ pathname: routes.reflexions, search: scopeSearch(search) }} className="reflexions__link">
              Back to the board
            </Link>
          </>
        }
      />
      {error ? (
        <ErrorState message={`Could not load the items: ${error.message}`} />
      ) : isPending ? (
        <LoadingState />
      ) : settled === 0 && trial.length === 0 ? (
        <EmptyState icon="notebook" title="Nothing settled yet">
          A reflexion enters the journal when its status becomes <code>settled</code>.
        </EmptyState>
      ) : (
        <div className="journal__body">
          {trial.length > 0 && (
            <section className="journal__trial">
              <h2 className="journal__trial-title">On trial — decided, to be confirmed</h2>
              <ul className="journal__trial-list">
                {trial.map((item) => (
                  <li key={item.path}>
                    <span className="journal__trial-id">{item.id}</span>
                    <ItemLink item={item}>
                      <Highlight text={item.title ?? item.path} />
                    </ItemLink>
                  </li>
                ))}
              </ul>
            </section>
          )}
          {months.map((month) => (
            <section key={month.key} aria-labelledby={`month-${month.key}`}>
              <h2 id={`month-${month.key}`} className="journal__month">
                {month.label}
              </h2>
              <ol className="journal__timeline">
                {month.items.map((item) => (
                  <li key={item.path} className="journal__entry">
                    <span className="journal__dot" aria-hidden />
                    <p className="journal__meta">
                      {item.updated ?? item.created ?? 'undated'}
                      {item.node !== scope && <span className="journal__meta-node">{displayNode(item.node)}</span>}
                      {item.archived && <span className="journal__meta-tag">archived</span>}
                    </p>
                    <h3 className="journal__entry-title">
                      <ItemLink item={item}>
                        <Highlight text={item.title ?? item.path} />
                      </ItemLink>
                    </h3>
                    {excerpt(item.body, 400) && (
                      <p className="journal__excerpt">
                        <Highlight text={excerpt(item.body, 400)} />
                      </p>
                    )}
                  </li>
                ))}
              </ol>
            </section>
          ))}
        </div>
      )}
    </div>
  )
}
