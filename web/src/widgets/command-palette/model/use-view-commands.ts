import { LayoutGrid } from 'lucide-react'
import { useLocation, useNavigate } from 'react-router'
import { scopeSearch, views } from '@/shared/config'
import type { Command } from '@/shared/lib'

// The views of `shared/config`, opened with the node scope, the archive toggle and the search field kept.
export function useViewCommands(): Command[] {
  const navigate = useNavigate()
  const { pathname, search } = useLocation()
  return views.map((view) => ({
    id: `view:${view.path}`,
    group: 'Views',
    title: view.title,
    icon: LayoutGrid,
    keywords: view.keywords,
    hint: view.path === pathname ? 'current' : undefined,
    run: () => void navigate({ pathname: view.path, search: scopeSearch(search) }),
  }))
}
