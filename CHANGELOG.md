# Changelog

## 0.1.8

Rebuilt with Ferrum-Frontend 922fcfb: an action's dialog says every refusal, the errors on what the action made (another record's rows) among them, and an action with parameters and a confirmation asks once.

## 0.1.7

What walking a clinic's day through the pages found:
- The day is the workspace's, not UTC's, so an evening reception west of Greenwich, or an early one east of it, sees its own day.
- Patients are named by each appointment's own link title. The page fetched at most 500 patients, so a larger clinic saw blank names.
- An empty clinic says what to set up first, and a practitioner with nothing booked says so.
- A visit checked in, completed, cancelled or missed shows its patient, practitioner, type, date and time as read-only, as the server already holds them. The minutes past midnight it is checked by are no longer shown.
- A new appointment is for today unless another day is chosen.
- Rebuilt with Ferrum-Frontend a76295e: times on a person's own clock when they chose one on their Account page.

## 0.1.6

Rebuilt with Ferrum-Frontend 24c3951: its pages show times on the workspace's clock, naming the zone when it is not the browser's, a draft numbered on submit is headed as a draft, and a record of an app that is not installed says so.

## 0.1.5

Rebuilt with Ferrum-Frontend bb60115: a record page reads an empty number or check as the server does, so an action whose condition needs one shows, and a workspace's settings open as their one record.

## 0.1.4

Rebuilt with Ferrum-Frontend 00dea07: in a workflow state that names what may change, the other fields show read-only, and the fields the state needs show required.

## 0.1.3

Personal-data requests find a patient by email, mobile number or name, and their appointments, consents and fee validities through the patient. All of it is kept when erasure is asked, as medical, consent and billing records, and the answer says why (Ferrum's FERRUM-PRIVACY-002).

## 0.1.2

Rebuilt with Ferrum-Frontend daeadda: after an action runs, a record page shows what the action said it did rather than only that it is done.

## 0.1.1

Rebuilt with Ferrum-Frontend 80ff179: a field's label names its control for assistive technology, and a record whose title is a link shows that record's title, not its id.

## 0.1.0

Healthcare administration (FERRUM-INDUSTRY-005).

- Patients, practitioners with their hours each weekday, and appointment types.
- Appointments within the practitioner's hours, clashing with neither's other visits.
- Consent forms with versions, consents given and withdrawn, and check-in only with consent to the
  current version where the type needs it.
- Visits completed by their practitioner, billed through ERP to a customer made from the patient,
  with follow-ups covered within the settings' days.
- Its own interface at `/healthcare/`: the day's appointments by practitioner.
