// Healthcare's endpoints for patients. Without signing in (`/api/site/{workspace}/healthcare/…`):
// who and what is booked online, a day's free times, booking, and a visit's own link. Signed in to
// the portal (`/api/app/healthcare/portal/…`): the patients known by the person's address, with
// their visits, invoices and the consents their coming visits need. The server holds every rule.

import { api } from "@ferrum/client";

export interface Options {
  practitioners: { id: string; name: string; department: string | null }[];
  types: { id: string; name: string; minutes: number | null }[];
}

export interface Visit {
  id: string;
  number: string | null;
  practitioner: string | null;
  type: string | null;
  date: string;
  time: string;
  status: string;
  cancellable: boolean;
}

export interface Patient {
  id: string;
  name: string;
  visits: Visit[];
  consents_needed: { form: string; name: string; version: number | null; text: string }[];
  invoices: { id: string; number: string | null; posting_date: string; total: string; outstanding: string }[];
}

const site = (tenant: string) => `/api/site/${encodeURIComponent(tenant)}/healthcare`;
const PORTAL = "/api/app/healthcare/portal";

export const options = (tenant: string, signal?: AbortSignal) => api.get<Options>(`${site(tenant)}/book/options`, undefined, signal);

export const freeTimes = (tenant: string, q: { practitioner: string; type: string; date: string }, signal?: AbortSignal) =>
  api.get<{ date: string; minutes: number; times: string[] }>(`${site(tenant)}/book/slots`, q, signal);

export type Booked = { visit: Visit; token: string } | { status: "received" };

export const book = (tenant: string, body: unknown) => api.post<Booked>(`${site(tenant)}/book`, body);

export const visitByLink = (tenant: string, token: string, signal?: AbortSignal) => api.get<{ visit: Visit }>(`${site(tenant)}/manage/${encodeURIComponent(token)}`, undefined, signal);

export const cancelByLink = (tenant: string, token: string) => api.post<{ visit: Visit }>(`${site(tenant)}/manage/${encodeURIComponent(token)}/cancel`, {});

export const myVisits = (signal?: AbortSignal) => api.get<{ patients: Patient[] }>(`${PORTAL}/visits`, undefined, signal);

export const cancelMine = (visit: string) => api.post<{ visit: Visit }>(`${PORTAL}/cancel`, { visit });

export const giveConsent = (patient: string, form: string) => api.post<{ message: string }>(`${PORTAL}/consent`, { patient, form });

/** The page of a visit booked online, under the booking page. */
export const visitLink = (tenant: string, token: string) => `/book/${encodeURIComponent(tenant)}/visit/${encodeURIComponent(token)}`;

/** Visits still to come first, soonest first; then the rest, latest first. */
export function inOrder(visits: Visit[], today: string): Visit[] {
  const ahead = visits.filter((v) => v.status === "Scheduled" && v.date >= today).sort((a, b) => `${a.date} ${a.time}`.localeCompare(`${b.date} ${b.time}`));
  const rest = visits.filter((v) => !ahead.includes(v)).sort((a, b) => `${b.date} ${b.time}`.localeCompare(`${a.date} ${a.time}`));
  return [...ahead, ...rest];
}
