import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { setTimeZone } from "@ferrum/client";
import { inOrder, tomorrow, visitLink, type Visit } from "./patient/api";

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

describe("the clinic's day", () => {
  // 11:00 UTC on 5 Nov is already 6 Nov in Kiritimati (UTC+14), and still 5 Nov in any browser west of UTC+13.
  beforeEach(() => {
    vi.useFakeTimers();
    vi.setSystemTime(new Date("2026-11-05T11:00:00Z"));
    setTimeZone("Pacific/Kiritimati");
  });
  afterEach(() => {
    setTimeZone(undefined);
    vi.useRealTimers();
  });

  it("counts a visit as to come from the workspace's today, not the browser's", () => {
    expect(inOrder([v("a", "2026-11-05", "09:00"), v("b", "2026-11-06", "09:00")]).map((x) => x.id)).toEqual(["b", "a"]);
  });

  it("starts the booking page on the workspace's tomorrow", () => {
    expect(tomorrow()).toBe("2026-11-07");
  });
});
