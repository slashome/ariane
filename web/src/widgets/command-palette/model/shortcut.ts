// Whether a platform string names an Apple system, where the palette opens with ⌘K rather than Ctrl+K.
export function isApple(platform: string): boolean {
  return /mac|iphone|ipad|ipod/i.test(platform)
}

// Whether a key press opens the palette: ⌘K on an Apple system, Ctrl+K elsewhere, nothing else held.
export function isPaletteShortcut(
  event: Pick<KeyboardEvent, 'key' | 'metaKey' | 'ctrlKey' | 'altKey' | 'shiftKey'>,
  apple: boolean,
): boolean {
  const command = apple ? event.metaKey && !event.ctrlKey : event.ctrlKey && !event.metaKey
  return command && !event.altKey && !event.shiftKey && event.key.toLowerCase() === 'k'
}

// The platform as the browser tells it.
export function currentPlatform(): string {
  const agent = navigator as Navigator & { userAgentData?: { platform?: string } }
  return agent.userAgentData?.platform ?? navigator.platform
}
