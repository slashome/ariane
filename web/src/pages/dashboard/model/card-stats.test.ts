import type { Item, NodeSummary } from "@/shared/api";
import { cardStats } from "./card-stats";

function item(overrides: Partial<Item>): Item {
  return {
    node: "acme/web",
    id: "T001",
    title: "Ship",
    kind: "task",
    archived: false,
    status: "todo",
    category: "todo",
    size: null,
    impact: null,
    priority: null,
    rank: null,
    created: null,
    reference: null,
    type: null,
    personas: [],
    release: null,
    milestone: null,
    parent: null,
    epic: null,
    depends_on: [],
    external: [],
    branches: [],
    pull_requests: [],
    waiting_for: [],
    waiting_on: null,
    waiting_since: null,
    version: 1,
    path: "tree/acme/web/tasks/T001.md",
    ...overrides,
  };
}

const items = [
  item({ id: "T001", size: "XS", impact: "high", priority: "low" }),
  item({ id: "T002", size: "S", impact: "high", priority: "high", path: "b" }),
  item({ id: "T003", size: "M", impact: "high", path: "c" }),
  item({
    id: "T004",
    size: "XS",
    impact: "high",
    category: "done",
    status: "done",
    path: "d",
  }),
  item({ id: "T005", category: "blocked", status: "blocked", path: "e" }),
  item({ id: "T006", category: "todo", archived: true, path: "f" }),
  item({
    node: "acme/web-legacy",
    id: "T007",
    category: "blocked",
    status: "blocked",
    path: "g",
  }),
  item({
    node: "acme/web/api",
    id: "R001",
    kind: "reflexion",
    category: null,
    status: "open",
    path: "h",
  }),
];

const nodes: NodeSummary[] = [
  { path: "acme/web", items: 6, workflow: [], last_change: "2030-01-05" },
  { path: "acme/web/api", items: 1, workflow: [], last_change: "2030-02-01" },
  {
    path: "acme/web-legacy",
    items: 1,
    workflow: [],
    last_change: "2031-01-01",
  },
];

describe("cardStats", () => {
  const stats = cardStats(items, nodes, "acme/web");

  it("counts live items of the node and its descendants by category", () => {
    expect(stats.counts).toEqual({
      backlog: 0,
      todo: 3,
      in_progress: 0,
      waiting: 0,
      blocked: 1,
      done: 1,
      dropped: 0,
    });
    expect(stats.total).toBe(5);
  });

  it("lists open quick wins, highest priority first", () => {
    expect(stats.quickWins.map((entry) => entry.id)).toEqual(["T002", "T001"]);
  });

  it("lists quick wins in their manual order before their priority", () => {
    const ranked = items.map((entry) =>
      entry.id === "T001" ? { ...entry, rank: 1000 } : entry,
    );
    expect(
      cardStats(ranked, nodes, "acme/web").quickWins.map((entry) => entry.id),
    ).toEqual(["T001", "T002"]);
  });

  it("lists blockers and takes the latest change among descendants only", () => {
    expect(stats.blockers.map((entry) => entry.id)).toEqual(["T005"]);
    expect(stats.lastChange).toBe("2030-02-01");
  });
});
