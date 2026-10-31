import { makeItem } from "@/shared/api";
import { epicRows } from "./epic-rows";

describe("epicRows", () => {
  it("puts live epics first in their manual order, closed ones after, each with its stories", () => {
    const epics = [
      makeItem({ id: "E10", kind: "epic" }),
      makeItem({ id: "E1", kind: "epic", category: "done", status: "done" }),
      makeItem({ id: "E2", kind: "epic" }),
    ];
    const items = [
      ...epics,
      makeItem({ id: "S1", kind: "story", parent: "p#E2" }),
    ];
    const rows = epicRows(epics, items);
    expect(rows.map((row) => row.epic.id)).toEqual(["E2", "E10", "E1"]);
    expect(rows[0]?.stories.map((story) => story.id)).toEqual(["S1"]);
  });

  it("follows the rank before the id", () => {
    const epics = [
      makeItem({ id: "E1", kind: "epic" }),
      makeItem({ id: "E2", kind: "epic", rank: 1000 }),
    ];
    expect(epicRows(epics, epics).map((row) => row.epic.id)).toEqual([
      "E2",
      "E1",
    ]);
  });
});
