# Changelog

## 0.3.2

Rebuilt with Ferrum-Frontend 45dad7f, the design system as tuned after the landed build was reviewed. The left menu's icons are chosen so that none repeats on more than three rows and each fits its row; their drawings load beside the first page instead of inside it, so the page starts lighter; a row that scrolls sideways fades at the edge where there is more, in every browser and with reduced motion; a printed record leaves out empty fields and keeps its totals with the table; menus and popovers sit inside the page's landmarks for assistive technology; a selection bar folds what does not fit into More on a narrow screen. Nothing else changes.

## 0.3.1

The interface follows the upgraded design system, rebuilt with Ferrum-Frontend 0a47157. Text uses the kit's type roles (nothing under 12px) and controls its height tokens; loading shows skeletons shaped like what is coming; empty, no matches, denied, offline and failed are told apart; status colours come from stored state ids. The interface check now runs the UI lint against an empty baseline. The home is rebuilt on the kit's work-home pattern: one page title, the main action as a button, queues with their counts, status at its natural width; its other own pages follow the same anatomy.

## 0.3.0

The patients' portal as configuration too (Ferrum's builder-first gate BF1, docs/builder/portal.md).

- **Patient Portal**, a portal role the package ships: a patient's own record, their visits and consents, reached through the address they sign in with (`via: email`, `via: patient.email`), on the portal's generic Your records page. It cancels a visit (it names `cancel_appointment`) without editing one, and gives consents. Nothing in the personal or clinical groups. Administrators give it to portal people and may change it like any role.
- **Cancelling from outside the clinic** is held to the settings' Cancel Online Until window, however it is asked (a portal role, as the visit's link and the visits page always were); the clinic's people still cancel at any time.
- **A consent given on the portal** is recorded as given by whoever signed in there, whatever was typed.
- Needs Ferrum with portal roles (BF1).

## 0.2.0

The administrative profile completed, with its safeguards (FEO-048).

- **Booking online** (settings' Booked Online; practitioners and visit types marked Booked Online): a public page (/healthcare/book/{workspace}, `shell.public`) offering a day's free times within the practitioner's hours, clashing with nothing, after now; a visit booked with a name and email, for the patient known by that email or a new one (source Online). The visit's own link (only its SHA-256 kept) shows and cancels it until the settings' Cancel Online Until hours before. Nothing clinical or personal is shown; writes are limited per address.
- **Reminders**: upkeep queues an email to the patient the settings' Remind hours before a visit, once (Reminded At).
- **The patients' portal** (`shell.portal` visits): the patients known by the person's address, their visits (cancelled there within the same window), the consents their coming visits need (the form's current version, given there and recorded as given on the portal), and their issued invoices. Nothing clinical.
- **Insurance**: a patient's Insurer, Policy Number and Coverage %. Completing a visit bills the patient the rest and the insurer its share as an Insurance Claim (Submitted). Upkeep marks it Paid once its invoice is settled; Rejected by the Insurer (with a reason) bills the share to the patient instead and drops the insurer's draft invoice, or credits it once issued.
- Names and words in Arabic and Hindi for the new fields and pages.

## 0.1.16

Rebuilt with Ferrum-Frontend c59c9a2: the names of its models, fields and manifest in Arabic and Hindi (109 each in ui/src/locales/names, which the platform serves to every page of the workspace, so a record's labels, the launcher and the bar read in the person's language). Translated by the model, to be reviewed by native speakers.

## 0.1.15

The assistant knows what its records mean in its pages (contributes.shell.assistant): their states, what counts and where figures come from, with questions it suggests asking there. Personal and health details are in field groups: Patient.date_of_birth (personal), Patient.allergies (clinical), Appointment.notes (clinical). Every role that read or wrote them is granted the group, so people see what they saw, and an AI provider gets them only where the workspace's egress policy allows the group.

## 0.1.14

Rebuilt with Ferrum-Frontend 5b22e7b: its own words carry built-in Arabic and Hindi (9 words each, ui/src/locales, translated by the model and to be reviewed by native speakers), fetched for the page's language only. The labels it keeps in objects, its dialogs, its page titles and the words it keeps in variables go through t() as well, and the names and sizes it shows as they are keep their order right to left. Its pages are drawn once their words are in, and its entry carries only the parts of the kit it uses.

## 0.1.13

Rebuilt with Ferrum-Frontend 6537d65: an action's link, code or token shown once is kept in a dialog to copy, and on a page that reads right to left, English left untranslated keeps its own order. Its own words go through the kit's t() (9 places), so a workspace's own words for its language reach them, and Admin, Interface words lists them under it (dist/words.json).

## 0.1.12

Rebuilt with Ferrum-Frontend d5d98f2: a workspace's own words for its interface (Admin, Interface words) reach its pages as they load; an attachment field that holds a picture shows it; and a value another record already has is said so, where the form said the record had been changed by someone else.

## 0.1.11

Rebuilt with Ferrum-Frontend 63280d3: right to left, its pages mirror as they read (icons that point along the line, margins and alignment by the line's start and end, arrow keys along it), and the records pages' words are in Arabic and Hindi.

## 0.1.10

Found walking it in a browser on 7 November: Did Not Come was taken at 03:35 for an 11:00 visit.
A visit is marked missed only once it has begun, on the workspace's clock; before, it is refused
and says when the visit is.

## 0.1.9

Rebuilt with Ferrum-Frontend a64416a: a new record's box whose default is 1 shows ticked, as it is saved.

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
