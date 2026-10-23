import { render } from '@testing-library/react'
import { Highlight, HighlightTerms } from './highlight'

describe('Highlight', () => {
  it('marks the terms of the search field, case and accents ignored', () => {
    const { container } = render(
      <HighlightTerms terms={['ete']}>
        <Highlight text="Préparer l’Été" />
      </HighlightTerms>,
    )
    expect([...container.querySelectorAll('mark')].map((mark) => mark.textContent)).toEqual(['Été'])
    expect(container).toHaveTextContent('Préparer l’Été')
  })

  it('prefers the ranges it is given, and leaves a text without match untouched', () => {
    const { container } = render(<Highlight text="abc def" ranges={[[4, 7]]} />)
    expect(container.querySelector('mark')).toHaveTextContent('def')
    const plain = render(<Highlight text="abc" terms={['x']} />)
    expect(plain.container.innerHTML).toBe('abc')
  })
})
