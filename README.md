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
  practitioner and time.

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
- Reminders to patients, online booking and a patients' portal.
- Insurance claims and payers.
- Any claim to meet health-data law (BLOCK-LEGAL-REVIEW).

## Changelog

| Version | Highlights |
|---|---|
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
