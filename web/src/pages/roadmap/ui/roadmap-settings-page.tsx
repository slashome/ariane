import { Lock, Users } from 'lucide-react'
import type { ReactNode } from 'react'
import { useItemFilters } from '@/features/filter-items'
import { displayNode } from '@/shared/lib'
import { PageHeader } from '@/shared/ui'
import { useCalendar } from '../api/use-calendar'
import { nodeTomlPath } from '../model/estimates'
import { CalendarSection } from './calendar-section'
import { EstimatesSection } from './estimates-section'

function SettingsBlock({
  id,
  icon,
  title,
  file,
  summary,
  children,
}: {
  id: string
  icon: ReactNode
  title: string
  file: string
  summary: string
  children: ReactNode
}) {
  return (
    <section aria-labelledby={id} className="space-y-4 rounded-lg border border-border p-4">
      <header className="space-y-1">
        <h2 id={id} className="flex flex-wrap items-center gap-2 text-base font-semibold">
          {icon}
          {title}
          <code className="rounded bg-muted px-1.5 py-0.5 font-mono text-xs font-normal" data-testid={`${id}-file`}>
            {file}
          </code>
        </h2>
        <p className="text-xs text-muted-foreground">{summary}</p>
      </header>
      {children}
    </section>
  )
}

export function RoadmapSettingsPage() {
  const { filters } = useItemFilters()
  const node = filters.node
  const { data: overview } = useCalendar()
  return (
    <div className="flex flex-col gap-6 p-6">
      <PageHeader
        title="Roadmap settings"
        description={node === null ? 'Your work calendar, and the settings of the node chosen in the header' : `Settings of ${displayNode(node)}`}
      />
      <SettingsBlock
        id="personal"
        icon={<Lock className="size-4" aria-hidden />}
        title="Personal"
        file={overview?.file ?? 'calendar.toml'}
        summary="Your work calendar, in your personal repository: it turns hours into dates and is never shared with the team."
      >
        <CalendarSection node={node} />
      </SettingsBlock>
      <SettingsBlock
        id="shared"
        icon={<Users className="size-4" aria-hidden />}
        title="Shared with the team"
        file={node === null ? 'node.toml' : nodeTomlPath(node)}
        summary="The node's own node.toml: everyone who reads the node sees these values, and its sub-nodes inherit them key by key."
      >
        <EstimatesSection node={node} />
      </SettingsBlock>
    </div>
  )
}
