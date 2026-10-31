import { useLocation, useNavigate } from 'react-router'
import type { Item, SearchHit } from '@/shared/api'
import { scopeSearch, searchParamKeys } from '@/shared/config'
import { displayNode, type Command } from '@/shared/lib'
import { useItems } from '../api/use-items'
import { useSearch } from '../api/use-search'
import { itemHref } from './item-href'

// Opens an item: its page, or the preview over the current view with `preview` (and for an item without
// an id, which has no page).
function useOpenItem() {
  const navigate = useNavigate()
  const { pathname, search } = useLocation()
  return (item: Item, preview: boolean) => {
    const href = itemHref(item)
    if (href !== null && !preview) {
      void navigate({ pathname: href, search: scopeSearch(search) })
      return
    }
    const next = new URLSearchParams(search)
    next.set(searchParamKeys.preview, item.path)
    void navigate({ pathname, search: `?${next.toString()}` })
  }
}

function itemCommand(item: Item, open: (item: Item, preview: boolean) => void, group: string): Command {
  return {
    id: `${group}:${item.path}`,
    group,
    title: item.title ?? item.path,
    hint: item.reference ?? displayNode(item.node),
    keywords: [item.id, item.reference, item.node].filter((word): word is string => word !== null),
    icon: 'file',
    searchOnly: true,
    shiftLabel: 'preview',
    run: ({ shift }) => open(item, shift),
  }
}

// Every item of the tree, archived ones with the toggle: found by id, reference or title; Enter opens its
// page, Shift+Enter its preview.
export function useItemCommands(includeArchived: boolean): Command[] {
  const { data: items = [] } = useItems()
  const open = useOpenItem()
  return items.filter((item) => includeArchived || !item.archived).map((item) => itemCommand(item, open, 'Items'))
}

// The items whose body holds the text, from `ariane search`, each with the excerpt that matched; those
// found by their header are already among the items.
export function useFullTextCommands(text: string, includeArchived: boolean): { commands: Command[]; pending: boolean } {
  const { data: hits = [], isFetching } = useSearch(text, includeArchived)
  const open = useOpenItem()
  const commands = hits
    .filter((hit: SearchHit) => hit.field === 'body')
    .map((hit) => ({
      ...itemCommand(hit, open, 'In the text'),
      matched: true,
      detail: { text: hit.excerpt, ranges: hit.highlights },
    }))
  return { commands: text.trim().length >= 2 ? commands : [], pending: isFetching }
}
