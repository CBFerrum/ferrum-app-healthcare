import { describe, expect, it } from "vitest";
import { inOrder, visitLink, type Visit } from "./patient/api";

const v = (id: string, date: string, time: string, status = "Scheduled"): Visit => ({ id, number: null, practitioner: null, type: null, date, time, status, cancellable: false });

describe("the patient's pages", () => {
  it("lists visits to come first, soonest first, then the rest, latest first", () => {
    const order = inOrder([v("a", "2026-11-01", "09:00", "Completed"), v("b", "2026-12-02", "10:00"), v("c", "2026-12-01", "09:00"), v("d", "2026-11-20", "09:00", "Cancelled")], "2026-11-10");
    expect(order.map((x) => x.id)).toEqual(["c", "b", "d", "a"]);
  });

  it("keeps a visit's link under the booking page", () => {
    expect(visitLink("lake side", "a/b")).toBe("/book/lake%20side/visit/a%2Fb");
  });
});
