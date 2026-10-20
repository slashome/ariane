import type { UncommittedFile } from '@/shared/api'

export function defaultCommitMessage(files: UncommittedFile[]): string {
  const items = files.filter((file) => file.path.startsWith('tree/') && file.path.endsWith('.md')).length
  if (items > 0 && items === files.length) {
    return `content: update ${items} ${items === 1 ? 'item' : 'items'} from ariane serve`
  }
  return `content: update ${files.length} ${files.length === 1 ? 'file' : 'files'} from ariane serve`
}
