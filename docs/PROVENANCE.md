# Provenance

## ERPNext 12's Healthcare module

- Source: [frappe/erpnext](https://github.com/frappe/erpnext), branch `version-12`, commit d6da1a9,
  `erpnext/healthcare/doctype/`.
- Licence: GNU General Public License v3. This app is under the GNU Affero General Public License v3,
  which may be combined with GPL-3.0 code (section 13 of each licence).
- What was taken, as designs rather than code:
  - its doctypes Patient, Healthcare Practitioner (with its schedule's time slots), Appointment Type
    and Patient Appointment, adapted as Patient, Practitioner, Appointment Type and Appointment;
  - Patient Appointment's overlap check on the practitioner or the patient;
  - Fee Validity: a billed visit covering follow-ups with the same practitioner for some days and
    some visits, set in Healthcare Settings.
- Its own: consent forms with versions and patients' consents, check-in only with consent,
  completing only by the visit's practitioner, the customer made from the patient, and all the code.

The copy read is in `ERP/referance/industry/erpnext-v12` on the machine this was written on (sparse:
`erpnext/healthcare`, `erpnext/hotels` and `erpnext/restaurant`).
