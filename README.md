<div align="center">

<picture>
  <source media="(prefers-color-scheme: dark)" srcset="docs/assets/banner-dark.svg">
  <img alt="Ferrum Healthcare: appointments, consent and billing for a clinic" src="docs/assets/banner-light.svg" width="100%">
</picture>

<br>

[![Version](https://img.shields.io/badge/version-0.3.2-0e7490?style=flat-square)](CHANGELOG.md)
[![Licence: AGPL-3.0](https://img.shields.io/badge/licence-AGPL--3.0-blue?style=flat-square)](LICENSE)
[![Ferrum platform](https://img.shields.io/badge/runs%20on-Ferrum-2f5bea?style=flat-square)](https://github.com/CBFerrum/Ferrum)
![Rust](https://img.shields.io/badge/Rust-server-b7410e?style=flat-square&logo=rust)
![PostgreSQL](https://img.shields.io/badge/PostgreSQL-data-336791?style=flat-square&logo=postgresql&logoColor=white)
![React](https://img.shields.io/badge/React%20%2B%20TypeScript-interface-087ea4?style=flat-square&logo=react&logoColor=white)
![Tests](https://img.shields.io/badge/tests-3%20Rust%20%2B%20UI%20suite-16a34a?style=flat-square)

### Appointments that cannot clash, consent that is versioned, and visits billed in your own books.

Ferrum Healthcare is the administrative side of a clinic: patients, practitioners and their hours, appointments booked
at the desk or online, consent forms, reminders, a patient portal, and visits billed through Ferrum ERP with
insurers' shares claimed. It is for small and medium clinics that want a front desk and billing, not a clinical record.

[💡 Why](#-why-healthcare) · [📸 Screenshots](#-screenshots) · [✨ Features](#-features) · [📊 Compare](#-how-it-compares) · [📍 Roadmap](#-roadmap) · [🚀 Install](#-install) · [🔧 Develop](#-develop)

</div>

> [!IMPORTANT]
> This is an administration app. It keeps no diagnoses, prescriptions, test orders or results. It does not implement
> HL7 or FHIR. It claims no compliance with HIPAA, GDPR's special categories or any country's health-data law: that
> needs a legal review of how a clinic runs it.

## 💡 Why Healthcare

- 🕒 **Hours and clashes are enforced.** A visit must fall within its practitioner's hours that weekday and clash with
  neither the practitioner's nor the patient's other visits.
- 📝 **Consent follows the text.** Change a consent form's text and it becomes a new version, so every patient
  consents again. A patient is checked in only with a current consent, and consent can be withdrawn.
- 🧾 **Billing in the same books.** Completing a visit raises a draft invoice in ERP; follow-ups inside a paid visit's
  cover are not billed again; an insurer's share becomes a claim you can chase and re-bill.
- 🌐 **Patients help themselves.** A public booking page offers free times; a patient portal shows visits, consents
  and invoices; a private link lets a patient see or cancel one visit.
- 🔐 **Sensitive details held back.** Dates of birth, policy numbers, allergies and visit notes sit in field groups that
  a role must be granted, and that the assistant sees only where the workspace allows.
- 🔓 **Open source and self-hosted.** AGPL-3.0, on your own PostgreSQL.
- 🌍 **Arabic and Hindi** words and names, with right-to-left layout.

## 🎯 At a glance

| | |
|---|---|
| **Version** | 0.3.2 |
| **Kind** | Its own interface at `/healthcare/` (the day by practitioner, public booking page, patients' portal pages), with every record in Ferrum's lists and forms |
| **What it adds** | 10 record types (8 documents, 1 child table, 1 settings record), 4 roles (one for the portal), 1 menu |
| **Depends on** | Ferrum platform `>=0.3.2, <0.4`; ERP `>=0.1` (installing Healthcare installs ERP) |
| **Works with** | ERP (customers, items, companies, sales invoices, credit notes) |
| **Languages** | English, Arabic, Hindi (words and record names); right-to-left layout |
| **Tests** | 3 Rust tests (one walks a week at a clinic on PostgreSQL) and a UI suite |
| **Licence** | AGPL-3.0-only |

## 📸 Screenshots

<div align="center">
<picture><source media="(prefers-color-scheme: dark)" srcset="docs/assets/screens/healthcare-appointment-list-dark.webp"><img alt="Appointments" src="docs/assets/screens/healthcare-appointment-list-light.webp" width="100%"></picture>
<p><b>📅 Appointments</b><br><sub>Every visit with its date, time, patient, practitioner and status.</sub></p>
</div>

<table>
<tr>
<td width="50%" valign="top">
<picture><source media="(prefers-color-scheme: dark)" srcset="docs/assets/screens/healthcare-patient-list-dark.webp"><img alt="Patients" src="docs/assets/screens/healthcare-patient-list-light.webp"></picture>
<p><b>🧑‍⚕️ Patients</b><br><sub>Browse, search and filter patients.</sub></p>
</td>
<td width="50%" valign="top">
<picture><source media="(prefers-color-scheme: dark)" srcset="docs/assets/screens/healthcare-practitioner-list-dark.webp"><img alt="Practitioners" src="docs/assets/screens/healthcare-practitioner-list-light.webp"></picture>
<p><b>👩‍⚕️ Practitioners</b><br><sub>Each practitioner&#x27;s department, fee and whether patients book them online.</sub></p>
</td>
</tr>
<tr>
<td width="50%" valign="top">
<picture><source media="(prefers-color-scheme: dark)" srcset="docs/assets/screens/healthcare-home-dark.webp"><img alt="The day, by practitioner" src="docs/assets/screens/healthcare-home-light.webp"></picture>
<p><b>🩺 The day, by practitioner</b><br><sub>Today&#x27;s appointments grouped by practitioner, with a button to book a new one.</sub></p>
</td>
<td width="50%" valign="top">
<picture><source media="(prefers-color-scheme: dark)" srcset="docs/assets/screens/healthcare-patient-detail-dark.webp"><img alt="One patient" src="docs/assets/screens/healthcare-patient-detail-light.webp"></picture>
<p><b>🗂️ One patient</b><br><sub>Read and edit one patient, with insurer details and related records.</sub></p>
</td>
</tr>
<tr>
<td width="50%" valign="top">
<picture><source media="(prefers-color-scheme: dark)" srcset="docs/assets/screens/healthcare-appointment-detail-dark.webp"><img alt="One appointment" src="docs/assets/screens/healthcare-appointment-detail-light.webp"></picture>
<p><b>✅ One appointment</b><br><sub>Check a patient in, complete the visit and see what was billed.</sub></p>
</td>
<td width="50%"></td>
</tr>
</table>

### 📱 On a phone

<table>
<tr>
<td align="center" valign="top">
<picture><source media="(prefers-color-scheme: dark)" srcset="docs/assets/screens/healthcare-home-phone-dark.webp"><img alt="The day on a phone" src="docs/assets/screens/healthcare-home-phone-light.webp" width="260"></picture>
<p><b>🩺 The day on a phone</b><br><sub>The day&#x27;s appointments on a narrow screen.</sub></p>
</td>
<td align="center" valign="top">
<picture><source media="(prefers-color-scheme: dark)" srcset="docs/assets/screens/healthcare-patient-list-phone-dark.webp"><img alt="Patients on a phone" src="docs/assets/screens/healthcare-patient-list-phone-light.webp" width="260"></picture>
<p><b>🧑‍⚕️ Patients on a phone</b><br><sub>Find a patient from the front desk or the corridor.</sub></p>
</td>
</tr>
</table>


## ✨ Features

### 🧑‍⚕️ Patients

- **A patient record.** Name, sex (*Female, Male, Other, Not Stated*), date of birth, mobile, email, status (*Active,
  Inactive*) and where they came from (*Clinic* or *Online*).
- **Insurance on the patient.** An insurer (an ERP customer), a policy number and the insurer's share as a percentage.
- **Allergies** are kept in a clinical field group.
- **Billed to their own customer.** The first time a patient is billed, ERP gets a customer made from them. A
  namesake's customer is never reused.
- **Inactive patients** cannot be booked.
- **Known by email.** Online bookings and the portal find a patient by their email address, and a new patient is made
  when the address is not known.
- **Personal-data requests.** The platform's access and erasure requests find patients by email, mobile or name and
  their appointments through them. Medical records are kept as the retention note says; billing and consent
  evidence likewise.

### 👩‍⚕️ Practitioners and hours

- **Weekly hours.** Each practitioner has rows of day, from-time and to-time, in the workspace's local time. A row must
  be `HH:MM` to a later `HH:MM`.
- **A fee and an item.** Each practitioner has a consultation fee and may name the item a visit is billed as. The
  settings' item applies when they do not.
- **Their sign-in.** A practitioner's email links them to their user, so only they complete their own visits.
- **Department, active flag and online booking.** Tick *Booked Online* to offer them on the public page.

### 📅 Appointments

- **Types of visit.** An appointment type has a name, minutes, whether it is booked online, and an optional consent form.
- **Booked within hours.** A visit lasts its type's minutes unless it says otherwise, must fall inside the
  practitioner's hours that weekday, and must end the day it starts.
- **No clashes.** It cannot overlap the practitioner's or the patient's other visits that are scheduled or checked in. A
  completed or cancelled visit frees its time.
- **A life of five states.** *Scheduled, Checked In, Completed, Cancelled, No Show.* Check In, Complete the Visit, Cancel
  It and Did Not Come are buttons that show only when they apply.
- **Checked in with consent.** If the type needs a consent form, the patient must have given, and not withdrawn, consent
  to its current version.
- **Marked missed only once begun.** *Did Not Come* is refused until the visit's start time has passed, on the workspace's
  clock.
- **Read-only once under way.** After check-in the patient, practitioner, type, date and time are fixed.
- **Numbered** `APT-2026-00001`; source *Clinic* or *Online*.
- **Home page.** The day, grouped by practitioner in time order, with a day picker, a button for a new appointment and
  an empty state that tells a new clinic what to add first.

### 📝 Consent

- **Consent forms with versions.** A form starts at version 1 and moves to the next whenever its text changes.
- **A patient's consent** is to one version, with who gave it (the patient or who consents for them) and when.
- **Withdraw It.** A given consent can be withdrawn, with the time kept.
- **Kept as given.** A consent stays tied to the version it was given for. A patient has consented only if they gave, and
  have not withdrawn, a consent to the form's current version.

### 🧾 Completing a visit and billing

- **Who completes.** Only the visit's practitioner, a Healthcare Manager or an administrator.
- **Billed through ERP.** The practitioner's fee and item go on a draft Sales Invoice to the patient's customer.
- **Follow-ups.** A billed visit opens a fee validity: the next visits with the same practitioner within the settings'
  number of days, up to the settings' number of visits, are covered and not billed.
- **Insurer's share.** For an insured patient, the patient is billed the rest and the insurer its share, as an
  *Insurance Claim* with its own invoice.
- **Claim life.** A claim is *Submitted*, then *Paid* when upkeep sees the insurer's invoice settled, or *Rejected by
  the Insurer* with a reason. A rejected share is billed to the patient instead; the insurer's invoice is dropped if
  still a draft, or credited if issued.
- **Settings first.** Completing a visit without a company and a consultation item in Healthcare Settings is refused,
  with a message that says where to set them.

### 🌐 Booking online

- **A public page** at `/healthcare/book/{workspace}`, open without signing in, when the clinic ticks *Booked Online*.
  It offers the practitioners and kinds of visit marked for online booking.
- **Free times.** For a chosen day within 90 days ahead, it lists the start times inside the practitioner's hours that
  clash with nothing and are after now.
- **Booking.** The patient gives a name, an email and optionally a mobile number. They get a visit under the same rules
  as at the desk.
- **The visit's own link.** Only a hash of the link is kept. It shows the visit and cancels it, until the settings'
  number of hours before the visit.
- **Nothing clinical or personal** is shown. Requests that write are limited per address and workspace, and sizes are
  capped.
- **A page that explains itself** when the clinic does not book online.

### 🔔 Reminders

- **One email reminder.** Upkeep emails each patient the settings' number of hours before a visit, once. Zero hours
  switches it off.

### 🧑‍💻 The patients' portal

- **A Patient Portal role.** A portal person sees the patients known by their sign-in address (themselves, or a child):
  their visits, the consents their coming visits need, and their issued invoices.
- **Actions.** They cancel a visit while it may still be cancelled online and give consents, which are recorded as given
  by whoever signed in.
- **Nothing clinical.** Allergies and notes are never shown there. The role is configuration an administrator can change.

### 🔐 Roles and permissions

| Who | Can |
|---|---|
| 🏥 Healthcare Manager | Settings, practitioners, appointment types, consent forms; patients including personal and clinical groups; consents; appointments; insurance claims; reads items, companies, customers and invoices |
| 🛎️ Clinic Reception | Registers patients, books and checks in, takes consent, sees insurance claims (personal group) |
| 🩺 Practitioner | Reads patients and their visits, completes their own visits |
| 🙋 Patient Portal | A patient's own record, visits and consents, reached by their sign-in address; may cancel a visit |

Field groups: *personal* (date of birth, policy number) and *clinical* (allergies, visit notes). Roles are granted the
groups they need. The assistant and any outside AI provider are given a group's fields only where the workspace's
policy allows it.

### 🌍 Interface

- **Its own pages.** The day by practitioner, the public booking page and the patient's visit pages; records use the
  shared list and form pages.
- **Languages.** English, Arabic and Hindi words, Arabic and Hindi names for every record and field, and a workspace's
  own words through Admin.
- **Right to left.** Pages mirror as they read.
- **Assistant.** It knows the states and the sensitivity of health details, and suggests "Which appointments are booked
  for tomorrow?", "Who did not come this week?" and "Which patients withdrew a consent?".
- **Accessibility basics.** A skip link, labelled controls and told-apart empty, denied, offline and failed states.

## 🧭 How it works

```mermaid
flowchart LR
  B["Booked at the desk or online"] --> S["Scheduled"]
  S --> CI["Checked in (consent checked)"]
  CI --> CO["Completed"]
  S --> NS["No show"]
  S --> CA["Cancelled"]
  CO --> INV["Draft sales invoice in ERP"]
  CO --> FV["Fee validity covers follow-ups"]
  CO --> CL["Insurance claim to the insurer"]
```

A visit is booked within hours and without clashes, checked in with consent, and completed by its practitioner.
Completing it bills the patient, opens a follow-up cover, and claims the insurer's share when there is one.

```mermaid
flowchart TD
  F["Consent form (version n)"] -->|text changes| F2["Consent form (version n+1)"]
  F --> PC["Patient consent given"]
  PC -->|withdrawn| W["Withdrawn"]
  F2 --> NEED["Patients must consent again"]
```

Consent is tied to a form's version, so an edit to the wording asks every patient again.

## 🧱 Data model

| Entity | What it is | Key fields or states |
|---|---|---|
| Patient | A person seen at the clinic | Name, sex, date of birth, mobile, email, customer, insurer, policy number, coverage %, allergies; *Active, Inactive* |
| Practitioner | Someone who sees patients | Name, email, department, fee, item, hours, booked online, active |
| Hours (`PractitionerSlot`) | A weekly time range | Day, from, to |
| Appointment Type | A kind of visit | Name, minutes, booked online, consent form |
| Consent Form | What a patient agrees to | Name, text, version |
| Patient Consent | One consent to one form version | Patient, form, version, given by, given at; *Given, Withdrawn* |
| Appointment | A visit | Patient, practitioner, type, date, time, minutes, invoice, claim, source; *Scheduled, Checked In, Completed, Cancelled, No Show* |
| Fee Validity | Cover for follow-ups | Patient, practitioner, valid till, visits left, invoice |
| Insurance Claim | An insurer's share of a visit | Appointment, patient, insurer, amount, invoice, credit note, patient invoice, reason; *Submitted, Paid, Rejected* |
| Healthcare Settings | Billing and patient options | Company, consultation item, follow-up days and visits, booked online, cancel hours, reminder hours |

**Roles:** Healthcare Manager, Clinic Reception, Practitioner, Patient Portal.

**Reports and print formats:** none yet.

**Scheduled upkeep:** claims are marked paid when their invoice is settled, and reminders are sent.

## 🔌 Integrations

| With | How | Needs |
|---|---|---|
| **Ferrum ERP** | Customers, items and companies; visits become draft Sales Invoices; a rejected insurer's invoice is dropped or credited | ERP installed with the app; a company and a consultation item in Healthcare Settings |
| **Email** | Appointment reminders go through the platform's mail queue | Outgoing mail configured on the workspace |
| **Ferrum portal** | The Patient Portal role and the visits page | Portal enabled for the workspace |
| **Personal-data requests** | Patients, appointments, consents, fee validities and claims are declared to the platform | Nothing |

Not built in: SMS or WhatsApp, payment gateways, video visits, FHIR or HL7, lab or imaging systems, insurers' electronic
claims.

## 📊 How it compares

Healthcare is compared with **Epic**, **Oracle Health (Cerner)**, **athenahealth**, **OpenEMR** and **Frappe Healthcare**
(the open-source module in the ERPNext family that this app's design started from; Bahmni is the other well-known open
hospital system). Those are full clinical systems. This app covers the front desk and the billing, so the clinical rows are
honestly red.

| Capability | Ferrum Healthcare | Epic | Oracle Health | athenahealth | OpenEMR | Frappe Healthcare |
|---|:-:|:-:|:-:|:-:|:-:|:-:|
| Patient registry with contact and insurance details | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ |
| Practitioner weekly hours and schedule | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ |
| Appointments that cannot clash | ✅ | ✅ | ✅ | ✅ | 🟡 | ✅ |
| Appointment states: check in, complete, cancel, no show | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ |
| Booking online with free times | ✅ | ✅ | ✅ | ✅ | 🟡 | 🟡 |
| Patient portal with visits, invoices and consents | ✅ | ✅ | ✅ | ✅ | ✅ | 🟡 |
| Private link to see or cancel one visit | ✅ | 🟡 | 🟡 | 🟡 | ❌ | ❌ |
| Consent forms with versions and withdrawal | ✅ | ✅ | ✅ | ✅ | 🟡 | ❌ |
| Email appointment reminders | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ |
| Reminders by SMS or WhatsApp, confirm by reply | ❌ | ✅ | ✅ | ✅ | 🟡 | 🟡 |
| Visit billed as an invoice | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ |
| General ledger, payables and stock in the same system | ✅ | ❌ | ❌ | ❌ | ❌ | ✅ |
| Insurer's share claimed as its own invoice, rejected claims re-billed | ✅ | ✅ | ✅ | ✅ | 🟡 | 🟡 |
| Electronic claims, eligibility checks and pre-authorisation | ❌ | ✅ | ✅ | ✅ | 🟡 | ❌ |
| Role-based access to sensitive fields | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ |
| Log of who read a record, with break-glass | ❌ | ✅ | ✅ | ✅ | 🟡 | 🟡 |
| Practitioner leave, breaks, several locations, rooms | ❌ | ✅ | ✅ | ✅ | 🟡 | 🟡 |
| Recurring appointments, waitlist, recall | ❌ | ✅ | ✅ | ✅ | 🟡 | ❌ |
| Intake questionnaires and signed forms | ❌ | ✅ | ✅ | ✅ | 🟡 | ❌ |
| Clinical record: encounters, vitals, diagnoses, problem list | ❌ | ✅ | ✅ | ✅ | ✅ | ✅ |
| Prescriptions and e-prescribing | ❌ | ✅ | ✅ | ✅ | ✅ | 🟡 |
| Lab and imaging orders and results | ❌ | ✅ | ✅ | ✅ | ✅ | ✅ |
| Inpatients, wards and beds | ❌ | ✅ | ✅ | ➖ | ❌ | ✅ |
| Pharmacy and medicine dispensing | ❌ | ✅ | ✅ | ➖ | 🟡 | 🟡 |
| FHIR or HL7 interfaces | ❌ | ✅ | ✅ | ✅ | ✅ | ❌ |
| Telehealth video visits | ❌ | ✅ | ✅ | ✅ | 🟡 | 🟡 |
| Operational reports: collections, no-show rate, practitioner load | ❌ | ✅ | ✅ | ✅ | ✅ | ✅ |
| Native mobile apps | ❌ | ✅ | ✅ | ✅ | 🟡 | 🟡 |
| Self-hosted on your own servers | ✅ | 🟡 | 🟡 | ❌ | ✅ | ✅ |
| Open source | ✅ | ❌ | ❌ | ❌ | ✅ | ✅ |
| No licence fee per user or per visit | ✅ | ❌ | ❌ | ❌ | ✅ | ✅ |

Notes. **Ferrum 🟡/❌:** appointment reminders are one email, once; there is no leave or room booking; insurer
claims are invoices, not electronic claims; and the platform records changes to records but not who merely opened one.
**Leaders:** 🟡 means a lighter feature, an add-on or a commercial module, or a feature that depends on configuration. Epic and
Oracle Health are large systems, mostly hosted by the vendor or a partner; athenahealth is hosted by the vendor.
Whether a clinic meets health-data law depends on how it is run, not on the software alone.

### 🏆 Where Healthcare stands out

- 🧾 **The books are the books.** A visit is an invoice in ERP, beside stock, buying and the ledger. The clinical
  systems send billing elsewhere, or are a billing system of their own.
- 📝 **Consent that follows the wording.** Versioned forms, a check at check-in, and withdrawal that is kept.
- 🕒 **Clashes are refused by the server,** for the patient as well as the practitioner, however a visit is booked.
- 🔐 **Field groups.** Personal and clinical details are separate grants, and the assistant honours them.
- 🌐 **Booking and portal as configuration.** The patient portal is a role, not a separate product.
- 🔓 **Open and owned.** AGPL, no per-seat or per-visit fee.

### 🚧 Where the leaders are still ahead

- 🩻 **Everything clinical.** Encounters, diagnoses, prescriptions, orders, results and inpatient care.
- 📡 **Standards.** FHIR, HL7, electronic claims and eligibility.
- 📲 **Reach.** SMS and WhatsApp reminders, native apps and video visits.
- 📋 **Scheduling depth.** Leave, rooms, recurring visits, waitlists and recall.
- 🔎 **Read audit.** A log of who opened a record.

> [!NOTE]
> Leader columns are from public product information as of November 2026 and may change. Product names belong to
> their owners.

## 📍 Roadmap

### ✅ Recently shipped

- [x] **0.3.2** Tuned design system: better menu icons, lighter first page, cleaner print.
- [x] **0.3.1** The redesigned interface with shaped loading and told-apart states.
- [x] **0.3.0** The Patient Portal role as configuration; online cancellation held to the clinic's window.
- [x] **0.2.0** Booking online, reminders, the patients' portal and insurers' shares claimed.
- [x] **0.1.16** Names of records and fields in Arabic and Hindi.
- [x] **0.1.15** The assistant knows the records; personal and clinical details in field groups.

### 🔜 Next up

**Front desk**
- [ ] Practitioner leave, holidays and breaks, so the hours check respects them.
- [ ] Reminders by SMS and WhatsApp, with confirm or cancel by reply.
- [ ] Recurring appointments, a waitlist and recall lists.
- [ ] Patient identity: a record number series, address, ID, emergency contact, guardian, and a duplicate check and merge.

**Money**
- [ ] One bill with consultation, procedures, medicines and tests, with a price list and tax.
- [ ] Online payment at booking and at the desk, with receipts and refunds.
- [ ] Reports: collections, no-show rate, practitioner load and receivables by insurer.

**Trust**
- [ ] A log of who opened a patient record, with break-glass access.
- [ ] Intake forms before the visit and signed consent.

### 🔭 Later

**Clinical**
- [ ] Encounter notes, vitals, diagnoses and a problem list.
- [ ] Prescriptions with printing and templates.
- [ ] Lab and imaging orders and results.

**Standards and insurance**
- [ ] FHIR and HL7 interfaces.
- [ ] Insurers' eligibility, pre-authorisation and electronic claims.
- [ ] National health-ID integration where a country has one.

**Reach**
- [ ] Telehealth video visits.
- [ ] Rooms and several locations.
- [ ] Inpatients and pharmacy stock.
- [ ] Print formats for invoices and consent forms.

## 🚀 Install

Healthcare is a native Ferrum app: the Ferrum server includes it (`runtime: native`) and a workspace turns it on from
**App Manager**. It needs platform `>=0.3.2, <0.4`. Installing it installs **ERP**, which holds customers, items and
invoices.

After installing:

1. Give people the roles **Healthcare Manager**, **Clinic Reception** and **Practitioner**. Give the **Patient Portal**
   role to patients who sign in to the portal.
2. Open **Healthcare Settings**: set the **company**, the **consultation item**, how many days and visits a paid visit covers,
   whether visits are booked online, the **cancel hours** and the **reminder hours**.
3. Add **practitioners** with a fee and weekly hours (and their email, so they can complete their visits), **appointment
   types**, and **consent forms** where needed.
4. Book a visit. To open online booking, tick *Booked Online* in the settings, on each practitioner and on each type.

## 🔧 Develop

```bash
cargo fmt -- --check
cargo clippy --all-targets -- -D warnings
FERRUM_HEALTHCARE_TEST_DATABASE_URL=postgres://ferrum:ferrum@localhost:5432/ferrum_healthcare_test cargo test
```

The database must exist, be owned by a role that is not a superuser (superusers bypass row-level security) and be reset
before the run. `tests/healthcare_it.rs` walks a week at a clinic: hours and clashes; consent given, withdrawn and asked
again for a new version; visits completed and billed, and follow-ups covered then billed again; and who may do what.
Rust dependencies come from the Ferrum platform and Ferrum ERP repositories on `main`.

The interface lives in `ui/` and is built with Ferrum-Frontend. Check the Frontend out beside the repository as
`../ferrum/web`, install its packages, then:

```bash
cd ui
npm run build       # builds into ui/dist, which the server embeds
npm run check       # interface lint
npm run typecheck
npm test
```

CI (`.github/workflows/ci.yml`) runs Rust on PostgreSQL 17 and the interface checks.

## 📂 Project layout

```text
app.yaml                  manifest: id, roles, shell entries, public and portal pages, assistant notes
packages/healthcare/      entities, roles, field groups and personal-data declarations as YAML
src/appointments.rs       hours, clashes, check-in, completing and billing, fee validity
src/consent.rs            consent forms, versions, patient consents
src/claims.rs             insurance claims, paid and rejected
src/reminders.rs          upkeep: reminders and paid claims
src/site.rs               public booking and the patient's portal endpoints
ui/                       the day, the booking page, the patient's visits; words in Arabic and Hindi
tests/healthcare_it.rs    a week at a clinic on PostgreSQL
docs/PROVENANCE.md        where the design comes from
```

## 📜 Changelog

Every release is described in [CHANGELOG.md](CHANGELOG.md).

## 🤝 Contributing

Issues and pull requests are welcome at [github.com/CBFerrum/ferrum-app-healthcare](https://github.com/CBFerrum/ferrum-app-healthcare/issues).
Keep the checks above green. The design is adapted from ERPNext 12's Healthcare module; see
[docs/PROVENANCE.md](docs/PROVENANCE.md).

## 📄 License

AGPL-3.0-only. See [LICENSE](LICENSE).

<div align="center">

<sub>Part of <a href="https://github.com/CBFerrum/Ferrum">Ferrum</a>, the open business platform · Made with 🦀 Rust and 🐘 PostgreSQL</sub>

</div>
