import { categoryFill, categoryTone } from './category-tone'

describe('categoryTone', () => {
  it('gives each category its own modifier', () => {
    expect(categoryTone('todo')).toBe('category-tone category-tone--todo')
    expect(categoryTone('dropped')).toBe('category-tone category-tone--dropped')
  })

  it('marks an item without category as such', () => {
    expect(categoryTone(null)).toBe('category-tone category-tone--none')
  })

  it('fills a bar segment by category', () => {
    expect(categoryFill('blocked')).toBe('category-fill category-fill--blocked')
  })
})
