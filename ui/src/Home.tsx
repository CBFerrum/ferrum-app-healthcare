// The day's appointments by practitioner, in time order, each with its patient and where it stands.
// An appointment opens its record, where it is checked in and completed. The day is the
// workspace's, as the server counts it.

import { useQuery } from "@tanstack/react-query";
import { useState } from "react";
import { api, docPath, formatValue, todayIso, t, isolate } from "@ferrum/client";
import { ButtonLink, HomeSkeleton, LoadFailed, PageHead, Queue, QueueGrid, QueueItem, StatePanel, StatusBadge } from "@ferrum/kit";
import { APPOINTMENT_TONES } from "./patient/api";

export interface Row {
  id: string;
  [k: string]: unknown;
}

const rows = (entity: string, filters: unknown[], extra: Record<string, string | number | boolean> = {}) =>
  api.get<{ rows: Row[] }>(docPath(entity), { filters: JSON.stringify(filters), limit: 500, ...extra }).then((r) => r.rows);

/** The patient an appointment is for, by name (the link's title), not by id. */
export const patientOf = (a: Row): string => String((a._titles as Record<string, string> | undefined)?.patient ?? "");

/** The appointment type's name (the record's title), for the row's quiet line. */
export const typeOf = (a: Row): string => String((a._titles as Record<string, string> | undefined)?.appointment_type ?? "");

/** Appointments by practitioner id, each list in time order. */
export function byPractitioner(list: Row[]): Map<string, Row[]> {
  const out = new Map<string, Row[]>();
  for (const a of [...list].sort((x, y) => String(x.time).localeCompare(String(y.time)))) {
    const p = String(a.practitioner ?? "");
    out.set(p, [...(out.get(p) ?? []), a]);
  }
  return out;
}

/** A day as the person reads it ("11 Nov 2026"). */
const dayText = (v: string) => formatValue({ type: "date" }, v);

export function Home() {
  const [day, setDay] = useState(todayIso);
  const q = useQuery({
    queryKey: ["healthcare", "day", day],
    queryFn: async () => {
      const [practitioners, appointments] = await Promise.all([
        rows("Practitioner", [["active", "eq", true]], { sort: "practitioner_name asc" }),
        rows("Appointment", [["date", "eq", day]], { titles: true }),
      ]);
      return { practitioners, appointments };
    },
    refetchInterval: 30_000,
  });
  const actions = (
    <>
      <label className="hc-day-pick">
        <span className="muted">{t("Day")}</span>
        <input className="input" type="date" value={day} onChange={(e) => e.target.value && setDay(e.target.value)} />
      </label>
      <ButtonLink to="/e/Appointment/new" variant="primary" icon="plus">
        {t("New appointment")}
      </ButtonLink>
    </>
  );
  if (q.isPending)
    return (
      <>
        <PageHead title={t("The day")} actions={actions} />
        <HomeSkeleton queues={2} />
      </>
    );
  if (q.isError)
    return (
      <>
        <PageHead title={t("The day")} actions={actions} />
        <LoadFailed error={q.error} onRetry={() => void q.refetch()} />
      </>
    );
  const grouped = byPractitioner(q.data.appointments);
  return (
    <>
      <PageHead title={t("The day")} description={t("{day} · {n} booked", { day: dayText(day), n: q.data.appointments.length })} actions={actions} />
      {q.data.practitioners.length === 0 ? (
        <StatePanel
          kind="empty"
          title={t("No practitioners yet")}
          action={
            <ButtonLink to="/e/Practitioner/new" variant="primary" icon="plus">
              {t("Add a practitioner")}
            </ButtonLink>
          }
        >
          {t("Add a practitioner with their fee and weekly hours, and an appointment type, and the day's appointments show here.")}
        </StatePanel>
      ) : (
        <QueueGrid>
          {q.data.practitioners.map((p) => {
            const list = grouped.get(p.id) ?? [];
            return (
              <Queue key={p.id} title={isolate(String(p.practitioner_name))} count={list.length} empty={t("Nothing booked.")}>
                {list.map((a) => (
                  <QueueItem key={a.id} to={`/e/Appointment/${a.id}`} lead={isolate(String(a.time))} title={isolate(patientOf(a))} meta={isolate(typeOf(a))} trailing={<StatusBadge value={a.status} tones={APPOINTMENT_TONES} />} />
                ))}
              </Queue>
            );
          })}
        </QueueGrid>
      )}
    </>
  );
}
