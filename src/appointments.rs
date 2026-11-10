//! Appointments. A visit lasts its type's minutes unless it says otherwise, falls within its
//! practitioner's hours on that weekday, and clashes with neither the practitioner's nor the
//! patient's other visits that are still to come or under way. A patient is checked in only with
//! the consent the visit's type needs. Completing a visit bills the practitioner's fee through ERP
//! to the patient's customer, made from the patient when first billed, and opens a fee validity:
//! the next visits with that practitioner within the settings' days are follow-ups, covered and not
//! billed, as many as the settings say (ERPNext 12's Fee Validity).

use crate::consent::consented;
use crate::{minutes, obj};
use async_trait::async_trait;
use chrono::{Datelike, Duration, NaiveDate, Timelike};
use ferrum_domain::flow::created;
use ferrum_domain::util::{d, elevated, id, money, round, rows, s, sys};
use ferrum_kernel::{Ctx, Doc, DocHook, Error, HookCtx, Kernel, Result, Tx};
use rust_decimal::Decimal;
use serde_json::{Value, json};
use uuid::Uuid;

pub struct PractitionerHook;

#[async_trait]
impl DocHook for PractitionerHook {
    fn name(&self) -> &'static str {
        "healthcare.practitioner"
    }

    async fn validate(&self, _hc: &mut HookCtx<'_>, doc: &Doc) -> Result<()> {
        for h in rows(doc, "hours") {
            let (from, to) = (s(&h, "from_time").and_then(minutes), s(&h, "to_time").and_then(minutes));
            match (from, to) {
                (Some(f), Some(t)) if f < t => {}
                _ => return Err(Error::field("hours", format!("{}'s hours are HH:MM to a later HH:MM", s(&h, "day").unwrap_or("a day")))),
            }
        }
        if d(doc, "consultation_fee") < Decimal::ZERO {
            return Err(Error::field("consultation_fee", "is zero or more"));
        }
        Ok(())
    }
}

pub struct AppointmentHook;

pub(crate) const WEEKDAYS: [&str; 7] = ["Monday", "Tuesday", "Wednesday", "Thursday", "Friday", "Saturday", "Sunday"];

/// The visit's day, and its start and end in minutes past midnight.
fn when(doc: &Doc) -> Result<(NaiveDate, i64, i64)> {
    let day =
        s(doc, "date").and_then(|v| NaiveDate::parse_from_str(v.get(..10).unwrap_or(v), "%Y-%m-%d").ok()).ok_or_else(|| Error::field("date", "is required"))?;
    let start = s(doc, "time").and_then(minutes).ok_or_else(|| Error::field("time", "is HH:MM"))?;
    let length = doc.get("duration").and_then(Value::as_i64).unwrap_or(0);
    Ok((day, start, start + length))
}

/// HH:MM of minutes past midnight.
pub(crate) fn hhmm(m: i64) -> String {
    format!("{:02}:{:02}", m / 60, m % 60)
}

#[async_trait]
impl DocHook for AppointmentHook {
    fn name(&self) -> &'static str {
        "healthcare.appointment"
    }

    fn fills(&self) -> &'static [&'static str] {
        &["duration", "starts_min", "ends_min"]
    }

    async fn before_validate(&self, hc: &mut HookCtx<'_>, doc: &mut Doc) -> Result<()> {
        if doc.get("duration").and_then(Value::as_i64).is_none_or(|m| m <= 0) {
            let typed: Option<i64> = sqlx::query_scalar("select duration from e_appointment_type where tenant_id = $1 and id = $2")
                .bind(hc.ctx.tenant)
                .bind(id(doc, "appointment_type"))
                .fetch_optional(&mut **hc.tx)
                .await?
                .flatten();
            doc.insert("duration".into(), json!(typed.unwrap_or(15)));
        }
        if let Ok((_, start, end)) = when(doc) {
            doc.insert("starts_min".into(), json!(start));
            doc.insert("ends_min".into(), json!(end));
        }
        Ok(())
    }

    async fn validate(&self, hc: &mut HookCtx<'_>, doc: &Doc) -> Result<()> {
        let status = s(doc, "status").unwrap_or("Scheduled");
        if !matches!(status, "Scheduled" | "Checked In") {
            return Ok(());
        }
        let tenant = hc.ctx.tenant;
        let (day, start, end) = when(doc)?;
        if end > 24 * 60 {
            return Err(Error::field("duration", "a visit ends the day it starts"));
        }
        let system = sys(hc.ctx);
        let practitioner = hc
            .kernel
            .get_tx(&system, hc.tx, "Practitioner", id(doc, "practitioner").ok_or_else(|| Error::field("practitioner", "is required"))?, false)
            .await?;
        let name = s(&practitioner, "practitioner_name").unwrap_or("The practitioner").to_string();
        if practitioner.get("active").and_then(Value::as_bool) == Some(false) {
            return Err(Error::field("practitioner", format!("{name} is not seeing patients")));
        }
        let weekday = WEEKDAYS[day.weekday().num_days_from_monday() as usize];
        let within = rows(&practitioner, "hours").iter().any(|h| {
            s(h, "day") == Some(weekday)
                && s(h, "from_time").and_then(minutes).is_some_and(|f| f <= start)
                && s(h, "to_time").and_then(minutes).is_some_and(|t| end <= t)
        });
        if !within {
            return Err(Error::field("time", format!("{name} does not see patients on {weekday}s from {} to {}", hhmm(start), hhmm(end))));
        }
        let patient: Option<String> = sqlx::query_scalar("select status from e_patient where tenant_id = $1 and id = $2")
            .bind(tenant)
            .bind(id(doc, "patient"))
            .fetch_optional(&mut **hc.tx)
            .await?
            .flatten();
        if patient.as_deref() == Some("Inactive") {
            return Err(Error::field("patient", "is inactive"));
        }
        let clash: Option<(Option<String>, bool)> = sqlx::query_as(
            "select number, practitioner = $3 from e_appointment where tenant_id = $1 and \"date\" = $2 and id <> $5 \
             and status in ('Scheduled', 'Checked In') and (practitioner = $3 or patient = $4) and starts_min < $7 and ends_min > $6 limit 1",
        )
        .bind(tenant)
        .bind(day)
        .bind(id(doc, "practitioner"))
        .bind(id(doc, "patient"))
        .bind(id(doc, "id").unwrap_or_else(Uuid::nil))
        .bind(start)
        .bind(end)
        .fetch_optional(&mut **hc.tx)
        .await?;
        if let Some((other, same_practitioner)) = clash {
            let who = if same_practitioner { name } else { "The patient".to_string() };
            return Err(Error::field("time", format!("{who} is booked then, under {}", other.unwrap_or_default())));
        }
        Ok(())
    }

    async fn action(&self, hc: &mut HookCtx<'_>, name: &str, doc: &Doc, _params: &Value) -> Result<Option<Value>> {
        let appointment = id(doc, "id").ok_or_else(|| Error::internal("an appointment without an id"))?;
        let system = elevated(hc.ctx);
        let to = match name {
            "check_in" => {
                let form: Option<Uuid> = sqlx::query_scalar("select consent_form from e_appointment_type where tenant_id = $1 and id = $2")
                    .bind(hc.ctx.tenant)
                    .bind(id(doc, "appointment_type"))
                    .fetch_optional(&mut **hc.tx)
                    .await?
                    .flatten();
                if let (Some(form), Some(patient)) = (form, id(doc, "patient"))
                    && !consented(hc.tx, hc.ctx.tenant, patient, form).await?
                {
                    let (title, version): (String, Option<i64>) =
                        sqlx::query_as("select form_name, revision from e_consent_form where tenant_id = $1 and id = $2")
                            .bind(hc.ctx.tenant)
                            .bind(form)
                            .fetch_one(&mut **hc.tx)
                            .await?;
                    return Err(Error::invalid(format!("the patient has not consented to {title}, version {}", version.unwrap_or(1))));
                }
                "Checked In"
            }
            "complete" => {
                if !sees(hc, doc).await? {
                    return Err(Error::Forbidden("the visit's practitioner completes it".into()));
                }
                let out = bill(hc.kernel, &system, hc.tx, doc).await?;
                hc.kernel.set_values_tx(&system, hc.tx, "Appointment", appointment, obj(json!({"status": "Completed"}))).await?;
                return Ok(Some(out));
            }
            "cancel_appointment" => {
                // Someone outside the clinic (a patient on the portal, through a portal role naming
                // this action) cancels only while the visit may still be cancelled online, as on its
                // link; the clinic's people cancel at any time.
                if hc.ctx.outside {
                    let (_, hours) = crate::site::settings(hc.tx, hc.ctx.tenant).await?;
                    if !crate::site::cancellable(hc.ctx, doc, hours) {
                        return Err(Error::invalid(format!(
                            "this visit is not cancelled online: it is less than {hours} hours away, or not scheduled; call the clinic"
                        )));
                    }
                }
                "Cancelled"
            }
            "no_show" => {
                // Missed only once it was due: not before it begins, on the workspace's clock.
                let (day, start, _) = when(doc)?;
                let now = hc.ctx.now_local();
                if (day, start) > (now.date_naive(), i64::from(now.hour() * 60 + now.minute())) {
                    return Err(Error::invalid(format!("the visit has not begun yet: it is at {} on {day}", hhmm(start))));
                }
                "No Show"
            }
            _ => return Ok(None),
        };
        hc.kernel.set_values_tx(&system, hc.tx, "Appointment", appointment, obj(json!({"status": to}))).await?;
        Ok(Some(json!({"status": to})))
    }
}

/// Whether the caller may complete the visit: its practitioner (by the address they signed in
/// with), a Healthcare Manager, or an administrator.
async fn sees(hc: &mut HookCtx<'_>, doc: &Doc) -> Result<bool> {
    if hc.ctx.is_admin || hc.ctx.roles.iter().any(|r| r == "Healthcare Manager") {
        return Ok(true);
    }
    let email: Option<String> = sqlx::query_scalar("select email from e_practitioner where tenant_id = $1 and id = $2")
        .bind(hc.ctx.tenant)
        .bind(id(doc, "practitioner"))
        .fetch_optional(&mut **hc.tx)
        .await?
        .flatten();
    Ok(!hc.ctx.email.is_empty() && email.is_some_and(|e| e.eq_ignore_ascii_case(&hc.ctx.email)))
}

/// Bills a completed visit, or counts it against a paid visit's follow-ups.
async fn bill(k: &Kernel, ctx: &Ctx, tx: &mut Tx, doc: &Doc) -> Result<Value> {
    let tenant = ctx.tenant;
    let appointment = id(doc, "id").ok_or_else(|| Error::internal("an appointment without an id"))?;
    let (day, _, _) = when(doc)?;
    let (patient, practitioner) = (
        id(doc, "patient").ok_or_else(|| Error::field("patient", "is required"))?,
        id(doc, "practitioner").ok_or_else(|| Error::field("practitioner", "is required"))?,
    );
    // A follow-up within a paid visit's days, while it has visits left, is covered.
    let cover: Option<(Uuid, i64)> = sqlx::query_as(
        "select id, visits_left from e_fee_validity where tenant_id = $1 and patient = $2 and practitioner = $3 and valid_till >= $4 and visits_left > 0 \
         order by valid_till desc limit 1 for update",
    )
    .bind(tenant)
    .bind(patient)
    .bind(practitioner)
    .bind(day)
    .fetch_optional(&mut **tx)
    .await?;
    if let Some((validity, left)) = cover {
        k.set_values_tx(ctx, tx, "FeeValidity", validity, obj(json!({"visits_left": left - 1}))).await?;
        k.set_values_tx(ctx, tx, "Appointment", appointment, obj(json!({"covered_by": validity.to_string()}))).await?;
        return Ok(json!({"status": "Completed", "covered_by": validity.to_string()}));
    }
    let settings: Option<(Option<Uuid>, Option<Uuid>, Option<i64>, Option<i64>)> =
        sqlx::query_as("select company, consultation_item, follow_up_days, follow_up_visits from e_healthcare_settings where tenant_id = $1 limit 1")
            .bind(tenant)
            .fetch_optional(&mut **tx)
            .await?;
    let Some((Some(company), Some(default_item), days, visits)) = settings else {
        return Err(Error::invalid("set the company and the consultation item in Healthcare Settings first"));
    };
    let doctor = k.get_tx(ctx, tx, "Practitioner", practitioner, false).await?;
    let item = id(&doctor, "consultation_item").unwrap_or(default_item);
    let fee = d(&doctor, "consultation_fee");
    let customer = customer_of(k, ctx, tx, patient).await?;
    let currency: Option<Uuid> = sqlx::query_scalar("select default_currency from e_company where tenant_id = $1 and id = $2")
        .bind(tenant)
        .bind(company)
        .fetch_optional(&mut **tx)
        .await?
        .flatten();
    let what = format!("Consultation with {}, {day}", s(&doctor, "practitioner_name").unwrap_or_default());
    // An insured patient's insurer pays its share of the fee, claimed from it; the patient the rest.
    let insured = k.get_tx(ctx, tx, "Patient", patient, false).await?;
    let (insurer, cover) = (id(&insured, "insurer"), d(&insured, "coverage_percent").min(Decimal::ONE_HUNDRED));
    let theirs = match insurer {
        Some(_) if cover > Decimal::ZERO => round(fee * cover / Decimal::ONE_HUNDRED, 2),
        _ => Decimal::ZERO,
    };
    let line = |amount: Decimal, words: &str| json!({"item": item.to_string(), "qty": "1", "price_list_rate": money(amount), "discount_percent": "0", "description": words});
    let mut out = json!({"status": "Completed"});
    let mut billed = Value::Null;
    if fee - theirs > Decimal::ZERO || theirs.is_zero() {
        let inv = k
            .insert_tx(
                ctx,
                tx,
                "SalesInvoice",
                obj(json!({"company": company.to_string(), "customer": customer.to_string(), "currency": currency.map(|c| c.to_string()), "remarks": what,
                           "items": [line(fee - theirs, &what)]})),
            )
            .await?;
        billed = inv.get("id").cloned().unwrap_or(Value::Null);
        out["sales_invoice"] = created("SalesInvoice", &inv);
    }
    let mut values = obj(json!({"sales_invoice": billed}));
    if let (Some(insurer), true) = (insurer, theirs > Decimal::ZERO) {
        let words = format!("{what}: the insurer's {}% share, policy {}", cover.normalize(), s(&insured, "policy_number").unwrap_or("-"));
        let theirs_inv = k
            .insert_tx(
                ctx,
                tx,
                "SalesInvoice",
                obj(json!({"company": company.to_string(), "customer": insurer.to_string(), "currency": currency.map(|c| c.to_string()), "remarks": words,
                           "items": [line(theirs, &words)]})),
            )
            .await?;
        let claim = k
            .insert_tx(
                ctx,
                tx,
                "InsuranceClaim",
                obj(json!({"appointment": appointment.to_string(), "patient": patient.to_string(), "insurer": insurer.to_string(),
                           "policy_number": s(&insured, "policy_number"), "amount": money(theirs), "sales_invoice": theirs_inv.get("id"), "status": "Submitted"})),
            )
            .await?;
        values.insert("claim".into(), claim.get("id").cloned().unwrap_or(Value::Null));
        if billed.is_null() {
            billed = theirs_inv.get("id").cloned().unwrap_or(Value::Null);
        }
        out["claim"] = created("InsuranceClaim", &claim);
    }
    let till = day + Duration::days(days.unwrap_or(7).max(0));
    k.insert_tx(
        ctx,
        tx,
        "FeeValidity",
        obj(json!({"patient": patient.to_string(), "practitioner": practitioner.to_string(), "valid_till": till.to_string(), "visits_left": visits.unwrap_or(1).max(0),
                   "sales_invoice": billed})),
    )
    .await?;
    k.set_values_tx(ctx, tx, "Appointment", appointment, values).await?;
    Ok(out)
}

/// The customer a patient is billed to: theirs, or one made from them now.
pub(crate) async fn customer_of(k: &Kernel, ctx: &Ctx, tx: &mut Tx, patient: Uuid) -> Result<Uuid> {
    let p = k.get_tx(ctx, tx, "Patient", patient, true).await?;
    if let Some(c) = id(&p, "customer") {
        return Ok(c);
    }
    // Their own, never a namesake's.
    let name = s(&p, "patient_name").unwrap_or("Patient");
    let made = k.insert_tx(ctx, tx, "Customer", obj(json!({"customer_name": name, "customer_type": "Individual", "email": s(&p, "email")}))).await?;
    let c = id(&made, "id").ok_or_else(|| Error::internal("a customer without an id"))?;
    k.set_values_tx(ctx, tx, "Patient", patient, obj(json!({"customer": c.to_string()}))).await?;
    Ok(c)
}
