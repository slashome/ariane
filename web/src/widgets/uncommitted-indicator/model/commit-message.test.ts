import { defaultCommitMessage } from './commit-message'

describe('defaultCommitMessage', () => {
  it('counts items when only item files changed', () => {
    expect(defaultCommitMessage([{ code: 'M', path: 'tree/a/tasks/T001.md' }])).toBe(
      'content: update 1 item from ariane serve',
    )
  })

  it('counts files when something else changed too', () => {
    expect(
      defaultCommitMessage([
        { code: 'M', path: 'tree/a/tasks/T001.md' },
        { code: 'M', path: 'config/config.toml' },
      ]),
    ).toBe('content: update 2 files from ariane serve')
  })
})
