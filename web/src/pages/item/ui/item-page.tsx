import { useParams } from 'react-router'
import { referenceFromSplat } from '@/shared/config'
import { ItemView } from '@/widgets/item-view'
import './item.css'

// The page of a task, a reflexion or a ressource, whose address is `/<section>/<node…>/<id>`.
export function ItemPage({ sectionLabel }: { sectionLabel: string }) {
  const { '*': splat } = useParams()
  return (
    <div className="item-page">
      <ItemView reference={referenceFromSplat(splat)} sectionLabel={sectionLabel} />
    </div>
  )
}
