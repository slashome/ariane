export interface CodeLink {
  label: string
  url: string | null
}

// `owner/repo:branch` as a link to the branch on GitHub; a repository named `local/…` has no remote.
export function branchLink(entry: string): CodeLink {
  const separator = entry.indexOf(':')
  if (separator < 0) {
    return { label: entry, url: null }
  }
  const repository = entry.slice(0, separator)
  const branch = entry.slice(separator + 1)
  const url = repository.startsWith('local/') ? null : `https://github.com/${repository}/tree/${branch.split('/').map(encodeURIComponent).join('/')}`
  return { label: entry, url }
}

// A pull request URL shown as `owner/repo#12`, the way GitHub cites it.
export function pullRequestLink(url: string): CodeLink {
  const match = /^https:\/\/[^/]+\/([^/]+\/[^/]+)\/pull\/(\d+)/.exec(url)
  return { label: match ? `${match[1]}#${match[2]}` : url, url }
}
