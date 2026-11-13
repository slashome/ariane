import { SlidersHorizontal } from 'lucide-react'
import { EmptyState, PageHeader } from '@/shared/ui'

export function RoadmapSettingsPage() {
  return (
    <div className="flex flex-col gap-6 p-6">
      <PageHeader title="Roadmap settings" description="The releases of the node and the work calendar" />
      <EmptyState icon={SlidersHorizontal} title="Not editable here yet">
        Releases are declared in <code>node.toml</code>, the work calendar in <code>calendar.toml</code> (<code>ariane calendar edit</code>).
      </EmptyState>
    </div>
  )
}
