import { PageHeader } from '@/shared/ui'
import { RepoPanel } from '@/widgets/uncommitted-indicator'
import './repo.css'

export function RepoPage() {
  return (
    <div className="repo-page">
      <PageHeader title="Repository" description="The content repository's git state. Nothing is committed unless you ask." />
      <RepoPanel />
    </div>
  )
}
