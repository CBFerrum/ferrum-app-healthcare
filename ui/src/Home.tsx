// The day's appointments by practitioner, in time order, each with its patient and where it stands.
// An appointment opens its record, where it is checked in and completed. The day is the
// workspace's, as the server counts it.

import { useQuery } from "@tanstack/react-query";
import { useState } from "react";
import { Link } from "react-router";
import { api, docPath, todayIso } from "@ferrum/client";
import { Badge, EmptyState, ErrorState, PageLoading, statusTone, useShellTitle } from "@ferrum/kit";

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
  useShellTitle("Healthcare");
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
  if (q.isPending) return <PageLoading />;
  if (q.isError) return <ErrorState error={q.error} onRetry={() => void q.refetch()} />;
  const grouped = byPractitioner(q.data.appointments);
  return (
    <div className="np-page">
      <header className="row" style={{ justifyContent: "space-between" }}>
        <h2>The day</h2>
        <span className="row">
          <input className="input" type="date" value={day} onChange={(e) => setDay(e.target.value)} aria-label="Day" />
          <Link to="/e/Appointment/new">New appointment</Link>
        </span>
      </header>
      {q.data.practitioners.length === 0 ? (
        <EmptyState title="No practitioners yet" icon="calendar">
          <p className="muted">
            Add a practitioner with their fee and weekly hours, and an appointment type, and the day's appointments show here. <Link to="/e/Practitioner/new">Add a practitioner</Link>
          </p>
        </EmptyState>
      ) : (
        <div className="np-columns">
          {q.data.practitioners.map((p) => (
            <section key={p.id} className="card card-pad stack" aria-label={String(p.practitioner_name)}>
              <h3>{String(p.practitioner_name)}</h3>
              {grouped.get(p.id)?.length ? (
                <ul className="stack">
                  {(grouped.get(p.id) ?? []).map((a) => (
                    <li key={a.id}>
                      <Link to={`/e/Appointment/${a.id}`}>
                        {String(a.time)} · {patientOf(a)}
                      </Link>{" "}
                      <Badge tone={statusTone(String(a.status))}>{String(a.status)}</Badge>
                    </li>
                  ))}
                </ul>
              ) : (
                <p className="muted">Nothing booked.</p>
              )}
            </section>
          ))}
        </div>
      )}
    </div>
  );
}
