export function displayNode(path: string): string {
  return path === '' ? '(home)' : path
}

export function isWithinNode(path: string, scope: string): boolean {
  return scope === '' || path === scope || path.startsWith(`${scope}/`)
}
