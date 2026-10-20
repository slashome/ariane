import type { Category } from '@/shared/api'

const tones: Record<Category, string> = {
  backlog: 'bg-slate-500/15 text-slate-700 dark:text-slate-300',
  todo: 'bg-blue-500/15 text-blue-700 dark:text-blue-300',
  in_progress: 'bg-amber-500/15 text-amber-700 dark:text-amber-300',
  blocked: 'bg-red-500/15 text-red-700 dark:text-red-300',
  done: 'bg-green-500/15 text-green-700 dark:text-green-300',
  dropped: 'bg-muted text-muted-foreground line-through',
}

export function categoryTone(category: Category | null): string {
  return category ? tones[category] : 'border-dashed border-border text-muted-foreground'
}
