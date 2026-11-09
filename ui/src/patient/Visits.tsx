// The patient's page of the workspace's portal, at /portal/visits/: the patients known by the
// address the person signed in with (themselves, or a child), each with their visits (cancelled here
// while it may still be), the consents their coming visits need (given here), and their invoices
// (opening on ERP's "Your invoices"). Nothing clinical.

import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { refusal, t, isolate } from "@ferrum/client";
import { Badge, Button, confirmDialog, EmptyState, ErrorState, PageLoading, statusTone, toast, useShellTitle } from "@ferrum/kit";
import { cancelMine, giveConsent, inOrder, myVisits, type Patient } from "./api";

export function VisitsPage() {
  useShellTitle(t("Your visits"));
  const q = useQuery({ queryKey: ["hc-mine"], queryFn: ({ signal }) => myVisits(signal) });
  if (q.error) return <ErrorState error={q.error} onRetry={() => void q.refetch()} />;
  if (!q.data) return <PageLoading />;
  return (
    <div className="stack hc-portal">
      <div className="page-head">
        <div>
          <h1 className="page-title">{t("Your visits")}</h1>
          <p className="page-sub">{t("Your visits, the consents they need, and your invoices.")}</p>
        </div>
      </div>
      {q.data.patients.length === 0 ? (
        <EmptyState title={t("No visits here yet")} icon="calendar">
          <p className="muted">{t("Once the clinic knows you by this address, your visits are here.")}</p>
        </EmptyState>
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
            <Button size="sm" variant="primary" loading={agree.isPending} onClick={() => agree.mutate(c.form)}>
              {t("I consent")}
            </Button>
          </div>
        </div>
      ))}
      <h3>{t("Visits")}</h3>
      {patient.visits.length === 0 ? (
        <p className="muted">{t("No visits yet.")}</p>
      ) : (
        <ul className="stack">
          {inOrder(patient.visits, today).map((v) => (
            <li key={v.id} className="row" style={{ justifyContent: "space-between" }}>
              <span>
                {isolate(`${v.date} ${v.time}`)} · {isolate(v.practitioner ?? "")} · {isolate(v.type ?? "")} <Badge tone={statusTone(v.status)}>{isolate(v.status)}</Badge>
              </span>
              {v.cancellable && (
                <Button
                  size="sm"
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
        <ul className="stack">
          {patient.invoices.map((i) => (
            <li key={i.id}>
              <a href={`/portal/invoices/${i.id}`}>{isolate(i.number ?? i.posting_date)}</a> · {isolate(i.total)} ·{isolate(" ")}
              {Number(i.outstanding) > 0 ? t("{owed} still to pay", { owed: i.outstanding }) : t("Paid")}
            </li>
          ))}
        </ul>
      )}
    </section>
  );
}
