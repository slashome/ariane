// A handle suggested from a name: its first word, lower case, accents and other signs dropped
// (`Keyvan Coen` → `keyvan`, `Élodie` → `elodie`); `null` when nothing is left.
export function suggestHandle(name: string): string | null {
  const first = name.trim().split(/\s+/)[0] ?? ''
  const plain = first
    .normalize('NFD')
    .replace(/[̀-ͯ]/g, '')
    .toLowerCase()
    .replace(/[^a-z0-9-]/g, '')
    .replace(/^[^a-z]+/, '')
  return plain === '' ? null : plain
}

export function isHandle(handle: string): boolean {
  return /^[a-z][a-z0-9-]*$/.test(handle)
}

// A persona id reads `<group>:<role>`, each part a handle (`cmc:dev`, `user:rh`).
export function isPersonaId(id: string): boolean {
  return /^[a-z][a-z0-9-]*:[a-z][a-z0-9-]*$/.test(id)
}
