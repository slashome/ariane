import { NotebookPen } from 'lucide-react'
import { useMemo } from 'react'
import { Link, useLocation } from 'react-router'
import { excerpt, ItemLink } from '@/entities/item'
import { QueryField } from '@/features/filter-items'
import { routes, scopeSearch } from '@/shared/config'
import { displayNode } from '@/shared/lib'
import { EmptyState, ErrorState, Highlight, LoadingState, PageHeader } from '@/shared/ui'
import { journal } from '../model/reflexions'
import { useReflexions } from './use-reflexions'

export function JournalPage() {
  const { reflexions, total, isPending, error, scope } = useReflexions()
  const { search } = useLocation()
  const months = useMemo(() => journal(reflexions), [reflexions])
  const trial = reflexions.filter((item) => item.status === 'trial')
  const settled = months.reduce((sum, month) => sum + month.items.length, 0)

  return (
    <div className="flex flex-col gap-6 p-6">
      <PageHeader
        title="Journal"
        description={isPending ? 'Loading…' : `${settled} settled reflexions, dated by their latest commit · ${trial.length} on trial`}
        actions={
          <>
            <QueryField count={reflexions.length} total={total} />
            <Link to={{ pathname: routes.reflexions, search: scopeSearch(search) }} className="text-sm text-primary hover:underline">
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
        <EmptyState icon={NotebookPen} title="Nothing settled yet">
          A reflexion enters the journal when its status becomes <code>settled</code>.
        </EmptyState>
      ) : (
        <div className="max-w-3xl space-y-8">
          {trial.length > 0 && (
            <section className="rounded-lg border border-amber-500/40 bg-amber-500/5 p-4">
              <h2 className="mb-2 text-sm font-semibold">On trial — decided, to be confirmed</h2>
              <ul className="space-y-1 text-sm">
                {trial.map((item) => (
                  <li key={item.path}>
                    <span className="mr-2 font-mono text-xs text-muted-foreground">{item.id}</span>
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
              <h2 id={`month-${month.key}`} className="sticky top-0 z-10 bg-background py-1 text-xs font-semibold tracking-wide text-muted-foreground uppercase">
                {month.label}
              </h2>
              <ol className="mt-2 space-y-5 border-l border-border pl-5">
                {month.items.map((item) => (
                  <li key={item.path} className="relative">
                    <span className="absolute top-1.5 -left-[1.6rem] size-2.5 rounded-full border-2 border-background bg-green-500" aria-hidden />
                    <p className="text-xs text-muted-foreground">
                      {item.updated ?? item.created ?? 'undated'}
                      {item.node !== scope && <span className="ml-2 font-mono">{displayNode(item.node)}</span>}
                      {item.archived && <span className="ml-2">archived</span>}
                    </p>
                    <h3 className="font-medium">
                      <ItemLink item={item}>
                        <Highlight text={item.title ?? item.path} />
                      </ItemLink>
                    </h3>
                    {excerpt(item.body, 400) && (
                      <p className="mt-1 text-sm leading-relaxed text-muted-foreground">
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
