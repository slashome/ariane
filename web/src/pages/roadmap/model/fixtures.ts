import type { EpicPlan, Moment, ReleasePlan, Roadmap, StoryPlan, WorkPlan } from '@/shared/api'

export const now = '2026-11-13T12:00:00Z'

export function at(hours: number): Moment {
  return { hours, at: null }
}

const summary = { hours: 0, test_hours: 0, span: null, closed: null, done: 0, open: 0 }

export function makeWork(reference: string, overrides: Partial<WorkPlan> = {}): WorkPlan {
  return {
    reference,
    node: 'p',
    kind: 'task',
    title: `Work ${reference}`,
    stage: 'open',
    hours: 12,
    test_hours: 3,
    tester: 'owner',
    basis: 'size',
    target: null,
    sequence: 0,
    start: null,
    end: null,
    delivered: null,
    segments: [],
    validation: null,
    closed: null,
    ...overrides,
  }
}

export function planned(reference: string, from: number, work: number, test: number, overrides: Partial<WorkPlan> = {}): WorkPlan {
  return makeWork(reference, {
    hours: work,
    test_hours: test,
    start: at(from),
    end: at(from + work + test),
    delivered: at(from + work + test),
    segments: [
      { phase: 'work', start: at(from), end: at(from + work) },
      { phase: 'test', start: at(from + work), end: at(from + work + test) },
    ],
    ...overrides,
  })
}

export function makeStory(reference: string | null, work: WorkPlan[], overrides: Partial<StoryPlan> = {}): StoryPlan {
  return { ...summary, reference, title: reference && `Story ${reference}`, target: null, work, ...overrides }
}

export function makeEpic(reference: string | null, stories: StoryPlan[], overrides: Partial<EpicPlan> = {}): EpicPlan {
  return { ...summary, reference, title: reference && `Epic ${reference}`, target: null, stories, ...overrides }
}

export function makeRelease(name: string, epics: EpicPlan[], overrides: Partial<ReleasePlan> = {}): ReleasePlan {
  return { ...summary, name, declared_by: 'p', target: null, epics, ...overrides }
}

export function makeRoadmap(releases: ReleasePlan[], overrides: Partial<Roadmap> = {}): Roadmap {
  return {
    node: 'p',
    carrier: null,
    timezone: null,
    now,
    dated: false,
    hours: 0,
    test_hours: 0,
    releases,
    unscheduled: [],
    warnings: [],
    ...overrides,
  }
}

export function sampleRoadmap(): Roadmap {
  return makeRoadmap(
    [
      makeRelease('0.1', [
        makeEpic(
          'p#E1',
          [
            makeStory('p#S1', [
              makeWork('p#T1', { stage: 'done', closed: { date: '2026-11-10', approximate: false } }),
              planned('p#T2', 0, 12, 3),
            ]),
            makeStory('p#S2', [planned('p#S2', 15, 12, 3, { kind: 'story', basis: 'provisional' })]),
          ],
          { hours: 24, test_hours: 6, done: 1, open: 2 },
        ),
        makeEpic('p#E2', [makeStory(null, [planned('p#E2', 30, 12, 3, { kind: 'epic', basis: 'default' })])]),
        makeEpic(null, [makeStory(null, [planned('p#T3', 45, 4, 1)])]),
      ]),
      makeRelease('0.2', [
        makeEpic('p#E1', [makeStory('p#S3', [planned('p#T4', 60, 8, 2)])], { hours: 8, test_hours: 2, open: 1 }),
      ]),
    ],
    { warnings: [{ kind: 'missed_target', count: 1, items: [{ reference: 'p#T2', detail: 'target 2026-11-12, forecast 2026-11-14' }] }] },
  )
}
