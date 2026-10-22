import { useParams } from 'react-router'
import { referenceFromSplat } from '@/shared/config'
import { ItemView } from '@/widgets/item-view'

// The page of a task, a reflexion or a ressource, whose address is `/<section>/<node…>/<id>`.
export function ItemPage({ sectionLabel }: { sectionLabel: string }) {
  const { '*': splat } = useParams()
  return (
    <div className="p-6">
      <ItemView reference={referenceFromSplat(splat)} sectionLabel={sectionLabel} />
    </div>
  )
}
