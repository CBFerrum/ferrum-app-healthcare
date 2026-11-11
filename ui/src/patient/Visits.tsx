// The patient's page of the workspace's portal, at /portal/visits/: the patients known by the
// address the person signed in with (themselves, or a child), each with their visits (cancelled here
// while it may still be), the consents their coming visits need (given here), and their invoices
// (opening on ERP's "Your invoices"). Nothing clinical.

import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { CalendarDays } from "lucide-react";
import { refusal, t, isolate } from "@ferrum/client";
import { Badge, Button, ButtonLink, confirmDialog, HomeSkeleton, LoadFailed, PageHead, Queue, QueueItem, StatePanel, StatusBadge, toast } from "@ferrum/kit";
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
    <section className="stack" aria-label={patient.name}>
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
      <Queue title={t("Visits")} count={patient.visits.length} empty={t("No visits yet.")}>
        {inOrder(patient.visits, today).map((v) => (
          <QueueItem
            key={v.id}
            lead={isolate(v.time)}
            title={isolate(v.practitioner ?? "")}
            meta={isolate([v.date, v.type].filter(Boolean).join(" · "))}
            trailing={<StatusBadge value={v.status} tones={APPOINTMENT_TONES} />}
            action={
              v.cancellable ? (
                <Button
                  loading={off.isPending && off.variables === v.id}
                  onClick={async () => {
                    if (await confirmDialog({ title: t("Cancel this visit?"), message: t("Its time is given up."), confirmLabel: t("Cancel it") })) off.mutate(v.id);
                  }}
                >
                  {t("Cancel")}
                </Button>
              ) : undefined
            }
          />
        ))}
      </Queue>
      <Queue title={t("Invoices")} count={patient.invoices.length} empty={t("No invoices yet.")}>
        {patient.invoices.map((i) => (
          <QueueItem
            key={i.id}
            title={isolate(i.number ?? i.posting_date)}
            meta={isolate(i.total)}
            trailing={Number(i.outstanding) > 0 ? <Badge tone="amber">{t("{owed} still to pay", { owed: i.outstanding })}</Badge> : <Badge tone="green">{t("Paid")}</Badge>}
            action={<ButtonLink href={`/portal/invoices/${i.id}`}>{t("Open")}</ButtonLink>}
          />
        ))}
      </Queue>
    </section>
  );
}
