import { PageHeader } from '@/shared/ui'
import { RepoPanel } from '@/widgets/uncommitted-indicator'

export function RepoPage() {
  return (
    <div className="flex flex-col gap-5 p-6">
      <PageHeader title="Repository" description="The content repository's git state. Nothing is committed unless you ask." />
      <RepoPanel />
    </div>
  )
}
