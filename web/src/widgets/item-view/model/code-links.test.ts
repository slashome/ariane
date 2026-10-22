import { branchLink, pullRequestLink } from './code-links'

describe('code links', () => {
  it('link a branch to GitHub unless its repository is local', () => {
    expect(branchLink('slashome/ariane:feat/web-x')).toEqual({
      label: 'slashome/ariane:feat/web-x',
      url: 'https://github.com/slashome/ariane/tree/feat/web-x',
    })
    expect(branchLink('local/scratch:main').url).toBeNull()
  })

  it('cite a pull request as owner/repo#number', () => {
    expect(pullRequestLink('https://github.com/slashome/ariane/pull/19').label).toBe('slashome/ariane#19')
    expect(pullRequestLink('https://example.com/x').label).toBe('https://example.com/x')
  })
})
