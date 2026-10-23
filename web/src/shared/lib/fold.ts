// Letters that do not decompose into a base letter and an accent, spelled out as `ariane search` does.
const spelled: Record<string, string> = { æ: 'ae', œ: 'oe', ß: 'ss', ø: 'o', đ: 'd', ħ: 'h', ı: 'i', ł: 'l', ŧ: 't' }

export interface Folded {
  // The text lowercased, accents stripped.
  text: string
  // For each UTF-16 unit of `text`, the code point of the original it comes from.
  origin: number[]
}

// Lowercases a text and strips its accents, so `Été` and `ete` compare equal; `origin` points every
// folded unit back to its code point in the original, to highlight the original where the folded matched.
export function fold(text: string): Folded {
  let folded = ''
  const origin: number[] = []
  Array.from(text).forEach((character, index) => {
    for (const lower of character.toLowerCase()) {
      const plain = spelled[lower] ?? lower.normalize('NFD').replace(/\p{M}/gu, '')
      folded += plain
      for (let unit = 0; unit < plain.length; unit += 1) {
        origin.push(index)
      }
    }
  })
  return { text: folded, origin }
}

// The folded form alone, for comparisons.
export function foldText(text: string): string {
  return fold(text).text
}
