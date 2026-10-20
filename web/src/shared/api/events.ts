export function subscribeToChanges(onChange: () => void): () => void {
  if (typeof EventSource === 'undefined') {
    return () => undefined
  }
  const source = new EventSource('/api/events')
  source.addEventListener('changed', onChange)
  return () => {
    source.removeEventListener('changed', onChange)
    source.close()
  }
}
