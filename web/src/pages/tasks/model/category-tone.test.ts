import { categoryTone } from './category-tone'

describe('categoryTone', () => {
  it('gives each category its own colour', () => {
    expect(categoryTone('todo')).toContain('blue')
    expect(categoryTone('in_progress')).toContain('amber')
    expect(categoryTone('blocked')).toContain('red')
    expect(categoryTone('done')).toContain('green')
    expect(categoryTone('backlog')).toContain('slate')
    expect(categoryTone('dropped')).toContain('line-through')
  })

  it('marks an item without category as such', () => {
    expect(categoryTone(null)).toContain('border-dashed')
  })
})
