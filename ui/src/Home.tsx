// The day's appointments by practitioner, in time order, each with its patient and where it stands.
// An appointment opens its record, where it is checked in and completed. The day is the
// workspace's, as the server counts it.

import { useQuery } from "@tanstack/react-query";
import { useState } from "react";
import { Link } from "react-router";
import { api, docPath, todayIso, t, isolate } from "@ferrum/client";
import { Badge, EmptyState, ErrorState, ListSkeleton, statusTone, useShellTitle } from "@ferrum/kit";
import { APPOINTMENT_TONES } from "./patient/api";

export interface Row {
  id: string;
  [k: string]: unknown;
}

const rows = (entity: string, filters: unknown[], extra: Record<string, string | number | boolean> = {}) =>
  api.get<{ rows: Row[] }>(docPath(entity), { filters: JSON.stringify(filters), limit: 500, ...extra }).then((r) => r.rows);

/** The patient an appointment is for, by name (the link's title), not by id. */
export const patientOf = (a: Row): string => String((a._titles as Record<string, string> | undefined)?.patient ?? "");

/** Appointments by practitioner id, each list in time order. */
export function byPractitioner(list: Row[]): Map<string, Row[]> {
  const out = new Map<string, Row[]>();
  for (const a of [...list].sort((x, y) => String(x.time).localeCompare(String(y.time)))) {
    const p = String(a.practitioner ?? "");
    out.set(p, [...(out.get(p) ?? []), a]);
  }
  return out;
}

export function Home() {
  useShellTitle(t("Healthcare"));
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
  if (q.isPending) return <ListSkeleton className="np-page" />;
  if (q.isError) return <ErrorState error={q.error} onRetry={() => void q.refetch()} />;
  const grouped = byPractitioner(q.data.appointments);
  return (
    <div className="np-page">
      <header className="row" style={{ justifyContent: "space-between" }}>
        <h2>{t("The day")}</h2>
        <span className="row">
          <input className="input" type="date" value={day} onChange={(e) => setDay(e.target.value)} aria-label={t("Day")} />
          <Link to="/e/Appointment/new">{t("New appointment")}</Link>
        </span>
      </header>
      {q.data.practitioners.length === 0 ? (
        <EmptyState title={t("No practitioners yet")} icon="calendar">
          <p className="muted">
            {t("Add a practitioner with their fee and weekly hours, and an appointment type, and the day's appointments show here.")} <Link to="/e/Practitioner/new">{t("Add a practitioner")}</Link>
          </p>
        </EmptyState>
      ) : (
        <div className="np-columns">
          {q.data.practitioners.map((p) => (
            <section key={p.id} className="card card-pad stack" aria-label={String(p.practitioner_name)}>
              <h3>{isolate(String(p.practitioner_name))}</h3>
              {grouped.get(p.id)?.length ? (
                <ul className="stack">
                  {(grouped.get(p.id) ?? []).map((a) => (
                    <li key={a.id}>
                      <Link to={`/e/Appointment/${a.id}`}>
                        {isolate(String(a.time))} · {isolate(patientOf(a))}
                      </Link>{isolate(" ")}
                      <Badge tone={statusTone(String(a.status), APPOINTMENT_TONES)}>{isolate(String(a.status))}</Badge>
                    </li>
                  ))}
                </ul>
              ) : (
                <p className="muted">{t("Nothing booked.")}</p>
              )}
            </section>
          ))}
        </div>
      )}
    </div>
  );
}
