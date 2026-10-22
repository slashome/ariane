// The first paragraph of prose of a body: headings, quotes, lists, tables and code are skipped.
export function excerpt(body: string | undefined, length = 220): string {
  if (!body) {
    return ''
  }
  for (const block of body.split(/\n\s*\n/)) {
    const text = block.trim()
    if (text === '' || /^(#|>|[-*+] |\d+\. |\||<!--|---|```)/.test(text)) {
      continue
    }
    const plain = text
      .replace(/\[([^\]]*)\]\([^)]*\)/g, '$1')
      .replace(/[*_`]/g, '')
      .replace(/\s+/g, ' ')
    return plain.length > length ? `${plain.slice(0, length - 1).trimEnd()}…` : plain
  }
  return ''
}
