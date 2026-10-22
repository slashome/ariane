import '@testing-library/jest-dom/vitest'

// jsdom lacks the pointer-capture and scrolling methods Radix's select calls when it opens.
if (typeof Element !== 'undefined') {
  Element.prototype.hasPointerCapture ??= () => false
  Element.prototype.releasePointerCapture ??= () => undefined
  Element.prototype.scrollIntoView ??= () => undefined
}
