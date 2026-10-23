// Where `current` stands in `list` and what comes before and after it; `null` when it is not there.
export function neighbours<T>(list: T[], isCurrent: (entry: T) => boolean): { position: number; previous?: T; next?: T } | null {
  const index = list.findIndex(isCurrent)
  if (index < 0) {
    return null
  }
  return { position: index + 1, previous: list[index - 1], next: list[index + 1] }
}
