import {
  categoryOrder,
  childrenOf,
  groupBy,
  isClosed,
  progressOf,
  sortByManualOrder,
  type Group,
  type Progress,
} from "@/entities/item";
import type { Item } from "@/shared/api";

export type StoryGrouping =
  "milestone" | "release" | "epic" | "node" | "status";

export const groupings: { value: StoryGrouping; label: string }[] = [
  { value: "milestone", label: "Release › milestone" },
  { value: "release", label: "Release" },
  { value: "epic", label: "Epic" },
  { value: "node", label: "Node" },
  { value: "status", label: "Status" },
];

export interface StoryRow {
  story: Item;
  tasks: Item[];
  progress: Progress;
}

export interface StoryGroup extends Group<StoryRow> {
  progress: Progress;
}

function keyOf(grouping: StoryGrouping, story: Item): string | null {
  switch (grouping) {
    case "milestone":
      return story.release === null && story.milestone === null
        ? null
        : [story.release ?? "—", story.milestone]
            .filter((part) => part !== null)
            .join(" › ");
    case "release":
      return story.release;
    case "epic":
      return story.epic;
    case "node":
      return story.node === "" ? "(home)" : story.node;
    case "status":
      return story.category;
  }
}

const noneLabels: Record<StoryGrouping, string> = {
  milestone: "No release or milestone",
  release: "No release",
  epic: "No epic",
  node: "(home)",
  status: "No status",
};

// Each story with its tasks — the items whose `parent` is the story — and their progress, grouped.
// Within a group, stories follow the manual order of their node, tasks too: live stories first, closed ones after.
export function groupStories(
  stories: Item[],
  tasks: Item[],
  grouping: StoryGrouping,
  label = (key: string) => key,
): StoryGroup[] {
  const ordered = sortByManualOrder(
    stories.map((story) => {
      const children = sortByManualOrder(
        childrenOf(tasks, story.reference),
        (task) => task,
      );
      return { story, tasks: children, progress: progressOf(children) };
    }),
    (row) => row.story,
  );
  const rows = [
    ...ordered.filter((row) => !isClosed(row.story.category)),
    ...ordered.filter((row) => isClosed(row.story.category)),
  ];
  const groups = groupBy(
    rows,
    (row) => keyOf(grouping, row.story),
    noneLabels[grouping],
    label,
  );
  if (grouping === "status") {
    groups.sort(
      (left, right) =>
        categoryOrder.indexOf(left.key as never) -
        categoryOrder.indexOf(right.key as never),
    );
  }
  return groups.map((group) => ({
    ...group,
    progress: progressOf(group.items.flatMap((row) => row.tasks)),
  }));
}
