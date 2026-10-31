import type { Item } from "@/shared/api";
import { compareNatural } from "./groups";

const levels = ["low", "medium", "high"];

function weight(priority: string | null): number {
  return priority === null ? -1 : levels.indexOf(priority);
}

function rankOf(item: Item): number | null {
  return item.archived ? null : item.rank;
}

export function compareManualOrder(left: Item, right: Item): number {
  const leftRank = rankOf(left);
  const rightRank = rankOf(right);
  const byRank =
    leftRank !== null && rightRank !== null
      ? leftRank - rightRank
      : leftRank !== null
        ? -1
        : rightRank !== null
          ? 1
          : weight(right.priority) - weight(left.priority);
  return byRank !== 0
    ? byRank
    : compareNatural(left.id ?? "\uffff", right.id ?? "\uffff");
}

export function sortByManualOrder<T>(
  entries: T[],
  itemOf: (entry: T) => Item,
): T[] {
  const nodes = new Map<string, number>();
  const kinds = new Map<string, number>();
  for (const entry of entries) {
    const item = itemOf(entry);
    if (!nodes.has(item.node)) {
      nodes.set(item.node, nodes.size);
    }
    const kind = `${item.node}#${item.kind}`;
    if (!kinds.has(kind)) {
      kinds.set(kind, kinds.size);
    }
  }
  const group = (item: Item) => [
    nodes.get(item.node) ?? 0,
    kinds.get(`${item.node}#${item.kind}`) ?? 0,
  ];
  return [...entries].sort((left, right) => {
    const [leftNode = 0, leftKind = 0] = group(itemOf(left));
    const [rightNode = 0, rightKind = 0] = group(itemOf(right));
    return (
      leftNode - rightNode ||
      leftKind - rightKind ||
      compareManualOrder(itemOf(left), itemOf(right))
    );
  });
}
