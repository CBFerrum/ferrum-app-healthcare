# Healthcare

Healthcare administration for [Ferrum](https://github.com/CBFerrum/Ferrum):
- patients, practitioners and their hours;
- appointments without clashes;
- consent, form by form and version by version;
- visits billed through ERP, with follow-ups covered.

| | |
|---|---|
| 🩺 App | `healthcare`, native, in the server |
| 🧩 Package | `healthcare` (needs ERP's selling and accounting) |
| 🖥️ Interface | its own, at `/healthcare/`: the day's appointments by practitioner, and the records |
| 🔐 Roles | Healthcare Manager, Clinic Reception, Practitioner |
| 📜 Licence | AGPL-3.0-only |

It is administration, not a clinical record: it keeps no diagnoses, prescriptions or results. No
health-data law (HIPAA, GDPR's special categories, or a country's own) is claimed to be met until a
legal review says so.

## Highlights

- **Hours and clashes.** A visit lasts its type's minutes unless it says otherwise. It falls within
  its practitioner's hours on that weekday. It clashes with neither the practitioner's nor the
  patient's other visits still to come or under way. A cancelled visit frees its time.
- **Consent.** An appointment type may need a consent form. A patient is checked in for it only if
  they gave, and have not withdrawn, a consent to the form's current version. Changing the form's
  text makes a new version, so every patient consents again. A consent is kept as it was given.
- **Completing and billing.** Only the visit's practitioner, or a Healthcare Manager, completes a
  visit. Completing bills the practitioner's fee and item through ERP to the patient's customer,
  which is made from the patient when they are first billed.
- **Follow-ups.** A billed visit covers the next visits with the same practitioner within the
  settings' days, as many as the settings say. A covered visit is not billed (ERPNext's fee
  validity).
- **Under way or over.** A visit checked in, completed, cancelled or missed keeps its patient,
  practitioner and time. It is marked missed only once it has begun, on the workspace's clock.
- **Booking online.** Where the clinic books online, its public page (/healthcare/book/{workspace})
  offers the practitioners and kinds of visit it books online, a day's free times (within the hours,
  clashing with nothing, after now), and books one with a name and email: the patient known by that
  email, or a new one. The visit's own link (only its hash kept) shows it and cancels it until the
  settings' hours before. Nothing clinical or personal is shown, and writes are limited.
- **Reminders.** Upkeep emails each patient the settings' hours before a visit, once.
- **The patients' portal.** Signed in to the workspace's portal, a person sees the patients known by
  their address (themselves, or a child): their visits, cancelled there while they may still be,
  the consents their coming visits need (each form's current version), given there, and their
  invoices. Nothing clinical.
- **Insurance.** A patient may name an insurer, a policy and the insurer's share. Completing their
  visit bills the patient the rest and the insurer its share as an Insurance Claim. Upkeep marks a
  claim paid once its invoice is settled; rejected, the share is billed to the patient instead, and
  the insurer's invoice dropped while a draft, or credited once issued.

## Develop

```bash
cargo test                       # needs FERRUM_HEALTHCARE_TEST_DATABASE_URL's database
cd ui && npm run build && npm run check
```

`tests/healthcare_it.rs` walks a week at a clinic:
- hours and clashes;
- consent given, withdrawn and asked again for a new version;
- visits completed and billed, and follow-ups covered then billed again;
- who may do what.

## Not yet

- Clinical records: encounters, diagnoses, prescriptions, lab tests and results.
- Inpatients, service units and rooms, and practitioners' leave and holidays.
- Reminders by text message, and payment online (ERP's invoices page shows what is due).
- Insurers' own portals, pre-authorisations and electronic claims (claims are invoices to the insurer).
- Any claim to meet health-data law (BLOCK-LEGAL-REVIEW).

## Changelog

| Version | Highlights |
|---|---|
| **0.3.0** | The Patient Portal role: the patients' portal as configuration; cancelling from outside held to the clinic's window. |
| **0.2.0** | Booking online, reminders, the patients' portal, insurers' shares claimed (FEO-048). |
| **0.1.16** | Its names in Arabic and Hindi. |
| **0.1.15** | The assistant knows its records; personal and health details in field groups. |
| **0.1.14** | Rebuilt with Ferrum-Frontend 5b22e7b: its own words in Arabic and Hindi. |
| **0.1.13** | Rebuilt with Ferrum-Frontend 6537d65: once-shown answers kept; English isolated right to left; its own words translatable by a workspace. |
| **0.1.12** | Rebuilt with Ferrum-Frontend d5d98f2: a workspace's own interface words; pictures in attachment fields; a duplicate value said so. |
| **0.1.11** | Rebuilt with Ferrum-Frontend 63280d3: right to left, pages mirror; the records pages' words in Arabic and Hindi. |
| **0.1.10** | A visit is marked missed only once it has begun. |
| **0.1.9** | Rebuilt with Ferrum-Frontend a64416a: a box whose default is 1 shows ticked. |
| **0.1.8** | Rebuilt with Ferrum-Frontend 922fcfb: action dialogs say every refusal and ask once. |
| **0.1.7** | The workspace's day on the home page, patients named by link title (no 500-patient limit), empty states, a visit under way read-only on its page, new appointments for today. |
| **0.1.6** | Rebuilt with Ferrum-Frontend 24c3951: times on the workspace's clock; drafts headed as drafts. |
| **0.1.5** | Rebuilt with Ferrum-Frontend bb60115: empty numbers read as the server reads them; settings open as their record. |
| **0.1.4** | Rebuilt with Ferrum-Frontend 00dea07: a workflow state's kept fields are read-only, its needed ones required. |
| **0.1.3** | Personal-data requests find patients by email, mobile or name, and their appointments through them; kept as medical records. |
| **0.1.2** | Rebuilt with Ferrum-Frontend daeadda: actions say what they did. |
| **0.1.1** | Rebuilt with Ferrum-Frontend 80ff179: labels name their fields, and linked titles show their titles. |
| **0.1.0** | Patients, practitioners' hours, appointments without clashes, consent by form version, and visits billed through ERP with follow-ups (FERRUM-INDUSTRY-005). |

## License

AGPL-3.0-only. See [LICENSE](LICENSE).
