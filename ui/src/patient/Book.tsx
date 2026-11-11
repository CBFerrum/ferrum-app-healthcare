// The clinic's booking page, /healthcare/book/{workspace}: a patient picks a practitioner, a kind of
// visit and a day, sees the free times, and books one with their name and email. They get the visit's
// own link to see or cancel it. And that link's page.

import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { useState } from "react";
import { Link, useParams } from "react-router";
import { ApiError, bootstrap, refusal, t, isolate } from "@ferrum/client";
import { Alert, Badge, Button, confirmDialog, EmptyState, ErrorState, Field, FormSkeleton } from "@ferrum/kit";
import { book, type Booked, cancelByLink, freeTimes, options, visitByLink, visitLink } from "./api";

function Page({ children }: { children: React.ReactNode }) {
  return (
    <div className="hc-page">
      <a className="skip-link" href="#main">
        {t("Skip to content")}
      </a>
      <header className="hc-top">
        <span className="hc-brand">{isolate(bootstrap()?.workspace.name ?? t("Book a visit"))}</span>
      </header>
      <main id="main" className="hc-main" tabIndex={-1}>
        {children}
      </main>
    </div>
  );
}

const closed = (error: unknown) => error instanceof ApiError && error.status === 404;

function tomorrow(): string {
  const d = new Date();
  d.setDate(d.getDate() + 1);
  return `${d.getFullYear()}-${String(d.getMonth() + 1).padStart(2, "0")}-${String(d.getDate()).padStart(2, "0")}`;
}

export function BookPage() {
  const { tenant = "" } = useParams();
  const q = useQuery({ queryKey: ["hc-book", tenant], queryFn: ({ signal }) => options(tenant, signal) });
  const [pick, setPick] = useState({ practitioner: "", type: "", date: tomorrow() });
  const [time, setTime] = useState("");
  const [who, setWho] = useState({ name: "", email: "", mobile: "", website: "" });
  const [done, setDone] = useState<Booked | null>(null);
  const practitioner = pick.practitioner || q.data?.practitioners[0]?.id || "";
  const type = pick.type || q.data?.types[0]?.id || "";
  const times = useQuery({
    queryKey: ["hc-times", tenant, practitioner, type, pick.date],
    queryFn: ({ signal }) => freeTimes(tenant, { practitioner, type, date: pick.date }, signal),
    enabled: !!practitioner && !!type && !!pick.date,
  });
  const go = useMutation({ mutationFn: () => book(tenant, { practitioner, type, date: pick.date, time, ...who }), onSuccess: setDone });
  if (q.isPending)
    return (
      <Page>
        <FormSkeleton />
      </Page>
    );
  if (q.isError)
    return (
      <Page>
        {closed(q.error) ? (
          <EmptyState title={t("Visits are not booked online here")} icon="calendar">
            <p className="muted">{t("Please call the clinic to book.")}</p>
          </EmptyState>
        ) : (
          <ErrorState error={q.error} onRetry={() => void q.refetch()} />
        )}
      </Page>
    );
  if (done)
    return (
      <Page>
        {"visit" in done ? (
          <section className="card card-pad stack" aria-label={t("Booked")}>
            <h1>{t("Booked: {date} at {time}", { date: done.visit.date, time: done.visit.time })}</h1>
            <p>{t("With {practitioner}, for {type}.", { practitioner: done.visit.practitioner ?? "", type: done.visit.type ?? "" })}</p>
            <p className="muted">{t("Keep your visit's link: it is how you see or cancel it.")}</p>
            <Link to={visitLink(tenant, done.token)}>{t("Your visit")}</Link>
          </section>
        ) : (
          <Alert tone="success">{t("Thank you: your request is in.")}</Alert>
        )}
      </Page>
    );
  return (
    <Page>
      <div className="stack">
        <h1>{t("Book a visit")}</h1>
        <section className="card card-pad stack" aria-label={t("When")}>
          <Field label={t("Practitioner")}>
            <select className="select" value={practitioner} onChange={(e) => setPick({ ...pick, practitioner: e.target.value })}>
              {q.data.practitioners.map((p) => (
                <option key={p.id} value={p.id}>
                  {p.department ? `${p.name} (${p.department})` : p.name}
                </option>
              ))}
            </select>
          </Field>
          <Field label={t("Visit")}>
            <select className="select" value={type} onChange={(e) => setPick({ ...pick, type: e.target.value })}>
              {q.data.types.map((v) => (
                <option key={v.id} value={v.id}>
                  {v.name}
                </option>
              ))}
            </select>
          </Field>
          <Field label={t("Day")}>
            <input className="input" type="date" value={pick.date} onChange={(e) => setPick({ ...pick, date: e.target.value })} />
          </Field>
          {times.isError && <Alert tone="error">{refusal(times.error)}</Alert>}
          {times.data && (
            <div role="group" aria-label={t("Free times")} className="hc-times">
              {times.data.times.length === 0 ? (
                <p className="muted">{t("No free time that day.")}</p>
              ) : (
                times.data.times.map((x) => (
                  <Button key={x} size="sm" variant={x === time ? "primary" : "default"} aria-pressed={x === time} onClick={() => setTime(x)}>
                    {x}
                  </Button>
                ))
              )}
            </div>
          )}
        </section>
        {time && (
          <form
            className="card card-pad stack"
            aria-label={t("Your details")}
            onSubmit={(e) => {
              e.preventDefault();
              go.mutate();
            }}
          >
            <Field label={t("Name")} required>
              <input className="input" autoComplete="name" required maxLength={200} value={who.name} onChange={(e) => setWho({ ...who, name: e.target.value })} />
            </Field>
            <Field label={t("Email")} required hint={t("Where the visit is confirmed and reminded")}>
              <input className="input" type="email" autoComplete="email" required maxLength={254} value={who.email} onChange={(e) => setWho({ ...who, email: e.target.value })} />
            </Field>
            <Field label={t("Mobile")}>
              <input className="input" type="tel" autoComplete="tel" maxLength={40} value={who.mobile} onChange={(e) => setWho({ ...who, mobile: e.target.value })} />
            </Field>
            <input className="hc-trap" tabIndex={-1} autoComplete="off" aria-hidden="true" name="website" value={who.website} onChange={(e) => setWho({ ...who, website: e.target.value })} />
            {go.isError && <Alert tone="error">{refusal(go.error)}</Alert>}
            <div>
              <Button type="submit" variant="primary" loading={go.isPending}>
                {t("Book {time}", { time })}
              </Button>
            </div>
          </form>
        )}
      </div>
    </Page>
  );
}

export function VisitPage() {
  const { tenant = "", token = "" } = useParams();
  const qc = useQueryClient();
  const q = useQuery({ queryKey: ["hc-visit", tenant, token], queryFn: ({ signal }) => visitByLink(tenant, token, signal) });
  const off = useMutation({ mutationFn: () => cancelByLink(tenant, token), onSuccess: (r) => qc.setQueryData(["hc-visit", tenant, token], r) });
  if (q.isPending)
    return (
      <Page>
        <FormSkeleton />
      </Page>
    );
  if (q.isError)
    return (
      <Page>
        {closed(q.error) ? (
          <EmptyState title={t("This link does not work")} icon="calendar">
            <p className="muted">{t("It opens only the visit it was given for.")}</p>
          </EmptyState>
        ) : (
          <ErrorState error={q.error} onRetry={() => void q.refetch()} />
        )}
      </Page>
    );
  const v = q.data.visit;
  return (
    <Page>
      <section className="card card-pad stack" aria-label={t("Your visit")}>
        <div className="row" style={{ justifyContent: "space-between" }}>
          <h1>{t("{date} at {time}", { date: v.date, time: v.time })}</h1>
          <Badge tone={v.status === "Scheduled" ? "green" : "neutral"}>{isolate(v.status)}</Badge>
        </div>
        <p>{t("With {practitioner}, for {type}.", { practitioner: v.practitioner ?? "", type: v.type ?? "" })}</p>
        {off.isError && <Alert tone="error">{refusal(off.error)}</Alert>}
        {v.cancellable && (
          <div>
            <Button
              variant="danger"
              loading={off.isPending}
              onClick={async () => {
                if (await confirmDialog({ title: t("Cancel this visit?"), message: t("Its time is given up."), confirmLabel: t("Cancel it") })) off.mutate();
              }}
            >
              {t("Cancel the visit")}
            </Button>
          </div>
        )}
      </section>
    </Page>
  );
}
