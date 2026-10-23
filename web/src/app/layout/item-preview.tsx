import { useItems } from '@/entities/item'
import { searchParamKeys } from '@/shared/config'
import { useSearchParam } from '@/shared/lib'
import { ItemDetails } from '@/widgets/item-details'

// The item previewed over any view, `?preview=<path>`, as the command palette opens it with Shift+Enter.
export function ItemPreview() {
  const { data: items = [] } = useItems()
  const [path, setPath] = useSearchParam(searchParamKeys.preview)
  const item = path === null ? null : (items.find((entry) => entry.path === path) ?? null)
  return <ItemDetails item={item} onClose={() => setPath(null)} />
}
