// The patient's page of the workspace's portal, at /portal/visits/: the patients known by the
// address the person signed in with (themselves, or a child), each with their visits (cancelled here
// while it may still be), the consents their coming visits need (given here), and their invoices
// (opening on ERP's "Your invoices"). Nothing clinical.

import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { CalendarDays } from "lucide-react";
import { refusal, t, isolate } from "@ferrum/client";
import { Badge, Button, confirmDialog, HomeSkeleton, LoadFailed, PageHead, StatePanel, StatusBadge, toast } from "@ferrum/kit";
import { APPOINTMENT_TONES, cancelMine, giveConsent, inOrder, myVisits, type Patient } from "./api";

export function VisitsPage() {
  const q = useQuery({ queryKey: ["hc-mine"], queryFn: ({ signal }) => myVisits(signal) });
  const head = <PageHead title={t("Your visits")} description={t("Your visits, the consents they need, and your invoices.")} />;
  if (q.error)
    return (
      <div className="hc-portal">
        {head}
        <LoadFailed error={q.error} onRetry={() => void q.refetch()} />
      </div>
    );
  if (!q.data)
    return (
      <div className="hc-portal">
        <PageHead title={t("Your visits")} />
        <HomeSkeleton queues={1} />
      </div>
    );
  return (
    <div className="stack hc-portal">
      {head}
      {q.data.patients.length === 0 ? (
        <StatePanel kind="empty" title={t("No visits here yet")} icon={CalendarDays}>
          {t("Once the clinic knows you by this address, your visits are here.")}
        </StatePanel>
      ) : (
        q.data.patients.map((p) => <PatientCard key={p.id} patient={p} />)
      )}
    </div>
  );
}

function PatientCard({ patient }: { patient: Patient }) {
  const qc = useQueryClient();
  const done = (message: string) => {
    toast(message);
    void qc.invalidateQueries({ queryKey: ["hc-mine"] });
  };
  const off = useMutation({ mutationFn: (visit: string) => cancelMine(visit), onSuccess: () => done(t("The visit is cancelled")), onError: (e) => toast(refusal(e), "error") });
  const agree = useMutation({ mutationFn: (form: string) => giveConsent(patient.id, form), onSuccess: () => done(t("Consent given")), onError: (e) => toast(refusal(e), "error") });
  const today = new Date().toISOString().slice(0, 10);
  return (
    <section className="card card-pad stack" aria-label={patient.name}>
      <h2>{isolate(patient.name)}</h2>
      {patient.consents_needed.map((c) => (
        <div key={c.form} className="card card-pad stack">
          <strong>{t("{name}, version {version}", { name: c.name, version: c.version ?? 1 })}</strong>
          <p className="hc-consent">{isolate(c.text)}</p>
          <div>
            <Button variant="primary" loading={agree.isPending} onClick={() => agree.mutate(c.form)}>
              {t("I consent")}
            </Button>
          </div>
        </div>
      ))}
      <h3>{t("Visits")}</h3>
      {patient.visits.length === 0 ? (
        <p className="muted">{t("No visits yet.")}</p>
      ) : (
        <ul className="hc-list">
          {inOrder(patient.visits, today).map((v) => (
            <li key={v.id} className="hc-visit">
              <span className="hc-when">{isolate(`${v.date} ${v.time}`)}</span>
              <span className="hc-what">{isolate([v.practitioner, v.type].filter(Boolean).join(" · "))}</span>
              <span className="hc-state">
                <StatusBadge value={v.status} tones={APPOINTMENT_TONES} />
              </span>
              {v.cancellable && (
                <Button
                  className="hc-act"
                  loading={off.isPending && off.variables === v.id}
                  onClick={async () => {
                    if (await confirmDialog({ title: t("Cancel this visit?"), message: t("Its time is given up."), confirmLabel: t("Cancel it") })) off.mutate(v.id);
                  }}
                >
                  {t("Cancel")}
                </Button>
              )}
            </li>
          ))}
        </ul>
      )}
      <h3>{t("Invoices")}</h3>
      {patient.invoices.length === 0 ? (
        <p className="muted">{t("No invoices yet.")}</p>
      ) : (
        <ul className="hc-list">
          {patient.invoices.map((i) => (
            <li key={i.id} className="hc-visit">
              <a className="hc-when hc-link" href={`/portal/invoices/${i.id}`}>
                {isolate(i.number ?? i.posting_date)}
              </a>
              <span className="hc-what num-inline">{isolate(i.total)}</span>
              <span className="hc-state">{Number(i.outstanding) > 0 ? <Badge tone="amber">{t("{owed} still to pay", { owed: i.outstanding })}</Badge> : <Badge tone="green">{t("Paid")}</Badge>}</span>
            </li>
          ))}
        </ul>
      )}
    </section>
  );
}
