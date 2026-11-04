import { describe, expect, it } from "vitest";
import { byPractitioner } from "./Home";

describe("the day", () => {
  it("groups appointments by practitioner, in time order", () => {
    const m = byPractitioner([
      { id: "b", practitioner: "p1", time: "10:30" },
      { id: "a", practitioner: "p1", time: "09:00" },
      { id: "c", practitioner: "p2", time: "11:00" },
    ]);
    expect(m.get("p1")?.map((x) => x.id)).toEqual(["a", "b"]);
    expect(m.get("p2")?.length).toBe(1);
  });
});
