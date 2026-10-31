import { makeItem, type Item } from "@/shared/api";
import { compareManualOrder, sortByManualOrder } from "./manual-order";

const ids = (items: Item[]) =>
  [...items].sort(compareManualOrder).map((item) => item.id);

describe("manual order", () => {
  it("puts ranked items first by ascending rank, negative ranks included", () => {
    expect(
      ids([
        makeItem({ id: "T1", rank: 1500 }),
        makeItem({ id: "T2", priority: "high" }),
        makeItem({ id: "T3", rank: -1000 }),
      ]),
    ).toEqual(["T3", "T1", "T2"]);
  });

  it("orders unranked items by descending priority, then ids in natural order", () => {
    expect(
      ids([
        makeItem({ id: "T10" }),
        makeItem({ id: "T9" }),
        makeItem({ id: "T2", priority: "low" }),
        makeItem({ id: "T3", priority: "high" }),
      ]),
    ).toEqual(["T3", "T2", "T9", "T10"]);
  });

  it("breaks equal ranks by id and ignores the rank of an archived item", () => {
    expect(
      ids([
        makeItem({ id: "T12", rank: 2000 }),
        makeItem({ id: "T3", rank: 2000 }),
        makeItem({ id: "T1", rank: 1, archived: true }),
      ]),
    ).toEqual(["T3", "T12", "T1"]);
  });

  it("keeps nodes and kinds in the order they first appear in, and never compares ranks across nodes", () => {
    const items = [
      makeItem({ node: "p", id: "T2", rank: 9000 }),
      makeItem({ node: "p/a", id: "T1", rank: -5 }),
      makeItem({ node: "p", id: "S1", kind: "story" }),
      makeItem({ node: "p", id: "T1", rank: 1000 }),
    ];
    expect(
      sortByManualOrder(items, (item) => item).map(
        (item) => `${item.node}#${item.id}`,
      ),
    ).toEqual(["p#T1", "p#T2", "p#S1", "p/a#T1"]);
  });
});
