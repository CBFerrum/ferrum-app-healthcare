//! Healthcare's endpoints for patients. Without signing in, at `/api/site/{workspace}/healthcare/…`,
//! where the clinic books online: the practitioners and visit types booked online, a practitioner's
//! free times on a day, booking one (as a new patient, or the one already known by that email), and
//! the visit's own link (only its SHA-256 kept) to see or cancel it until the settings' hours before.
//! Signed in to the workspace's portal, at `/api/app/healthcare/portal/…`: the visits of the patients
//! known by the address the person signed in with, their invoices, and the consents their coming
//! visits need, given there; cancelling a visit as on its link. Nothing clinical (allergies, notes)
//! is ever shown, and portal people hold no roles: everything is read for them, as the system, after
//! these checks.

use crate::appointments::{WEEKDAYS, hhmm};
use crate::{minutes, obj};
use base64::Engine as _;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use chrono::{Datelike, Duration, NaiveDate, NaiveDateTime};
use ferrum_domain::util::{id, rows, s, sys};
use ferrum_kernel::{Ctx, Doc, Error, FieldError, Kernel, Result, Site, SiteRequest, SiteResponse, Tx, with_retry};
use serde_json::{Map, Value, json};
use sha2::{Digest, Sha256};
use std::time::Duration as StdDuration;
use uuid::Uuid;

const MAX_BODY: usize = 16 * 1024;
const WRITES_PER_WINDOW: u32 = 10;
const WINDOW: StdDuration = StdDuration::from_secs(600);
/// How far ahead visits are booked online.
const AHEAD_DAYS: i64 = 90;

pub struct ClinicSite;

#[async_trait::async_trait]
impl Site for ClinicSite {
    fn max_body(&self) -> usize {
        MAX_BODY
    }

    async fn handle(&self, k: &Kernel, req: SiteRequest) -> Result<SiteResponse> {
        let segs: Vec<&str> = req.path.split('/').filter(|p| !p.is_empty()).collect();
        let (status, out) = match (req.method.as_str(), segs.as_slice()) {
            ("GET", ["book", "options"]) => (200, options(k, &public(k, &req).await?).await?),
            ("GET", ["book", "slots"]) => (200, slots(k, &public(k, &req).await?, &req).await?),
            ("POST", ["book"]) => {
                throttle(k, &req).await?;
                book(k, &public(k, &req).await?, &body(&req)?).await?
            }
            ("GET", ["manage", tok]) => (200, manage(k, &public(k, &req).await?, tok).await?),
            ("POST", ["manage", tok, "cancel"]) => {
                throttle(k, &req).await?;
                (200, cancel_by_link(k, &public(k, &req).await?, tok).await?)
            }
            ("GET", ["portal", "visits"]) => (200, visits(k, signed_in(&req)?).await?),
            ("POST", ["portal", "cancel"]) => (200, cancel_mine(k, signed_in(&req)?, &body(&req)?).await?),
            ("POST", ["portal", "consent"]) => (200, consent(k, signed_in(&req)?, &body(&req)?).await?),
            _ => return Err(Error::NotFound("page".into())),
        };
        let mut r = SiteResponse::json(status, &out);
        r.headers.push(("cache-control".into(), "no-store".into()));
        Ok(r)
    }
}

/// A limit per caller address and workspace on the requests that write.
async fn throttle(k: &Kernel, req: &SiteRequest) -> Result<()> {
    k.rate_limit(&format!("healthcare|{}|{}", req.tenant, req.client_ip), WINDOW, WRITES_PER_WINDOW, "too many requests from this address; try again later")
        .await
}

async fn public(k: &Kernel, req: &SiteRequest) -> Result<Ctx> {
    let schema = match &req.user {
        Some(u) => u.schema.clone(),
        None => k.registry.schema(req.tenant).await?,
    };
    let mut ctx = Ctx::system(req.tenant, schema);
    ctx.request_id = format!("healthcare-{}", Uuid::now_v7());
    Ok(ctx)
}

fn signed_in(req: &SiteRequest) -> Result<&Ctx> {
    req.user.as_ref().ok_or_else(|| Error::Unauthorized("sign in first".into()))
}

fn body(req: &SiteRequest) -> Result<Map<String, Value>> {
    match serde_json::from_slice::<Value>(&req.body) {
        Ok(Value::Object(m)) => Ok(m),
        _ => Err(Error::invalid("the request body is a JSON object")),
    }
}

fn text(body: &Map<String, Value>, key: &str, max: usize, errors: &mut Vec<FieldError>) -> Option<String> {
    let v = body.get(key).and_then(Value::as_str).map(str::trim).unwrap_or_default();
    if v.chars().count() > max {
        errors.push(FieldError::new(key, format!("is at most {max} characters")));
    }
    (!v.is_empty()).then(|| v.to_string())
}

fn hash(token: &str) -> String {
    hex::encode(Sha256::digest(token.as_bytes()))
}

fn uuid_of(v: Option<&Value>) -> Option<Uuid> {
    v.and_then(Value::as_str).and_then(|x| x.trim().parse().ok())
}

/// The clinic's settings for patients: (books online, hours before a visit it may still be cancelled).
pub(crate) async fn settings(tx: &mut Tx, tenant: Uuid) -> Result<(bool, i64)> {
    let row: Option<(Option<bool>, Option<i64>)> = sqlx::query_as("select book_online, cancel_hours from e_healthcare_settings where tenant_id = $1 limit 1")
        .bind(tenant)
        .fetch_optional(&mut **tx)
        .await?;
    let (online, hours) = row.unwrap_or_default();
    Ok((online.unwrap_or(false), hours.unwrap_or(2)))
}

/// The page is open: the clinic books online. 404 otherwise.
async fn open(tx: &mut Tx, tenant: Uuid) -> Result<i64> {
    match settings(tx, tenant).await? {
        (true, hours) => Ok(hours),
        _ => Err(Error::NotFound("page".into())),
    }
}

/// `GET /book/options`: who and what is booked online.
async fn options(k: &Kernel, ctx: &Ctx) -> Result<Value> {
    let mut tx = k.db.tenant_tx(ctx.tenant).await?;
    open(&mut tx, ctx.tenant).await?;
    let who: Vec<(Uuid, String, Option<String>)> = sqlx::query_as(
        "select id, practitioner_name, department from e_practitioner where tenant_id = $1 and coalesce(active, true) and coalesce(book_online, false) order by practitioner_name",
    )
    .bind(ctx.tenant)
    .fetch_all(&mut *tx)
    .await?;
    let what: Vec<(Uuid, String, Option<i64>)> =
        sqlx::query_as("select id, type_name, duration from e_appointment_type where tenant_id = $1 and coalesce(book_online, false) order by type_name")
            .bind(ctx.tenant)
            .fetch_all(&mut *tx)
            .await?;
    tx.commit().await?;
    Ok(json!({
        "practitioners": who.into_iter().map(|(id, name, dept)| json!({"id": id, "name": name, "department": dept})).collect::<Vec<_>>(),
        "types": what.into_iter().map(|(id, name, mins)| json!({"id": id, "name": name, "minutes": mins})).collect::<Vec<_>>(),
    }))
}

/// A practitioner and a visit type booked online; 404 otherwise. (practitioner, minutes)
async fn bookable(tx: &mut Tx, tenant: Uuid, practitioner: Option<Uuid>, kind: Option<Uuid>) -> Result<(Uuid, i64)> {
    let ok: Option<Uuid> =
        sqlx::query_scalar("select id from e_practitioner where tenant_id = $1 and id = $2 and coalesce(active, true) and coalesce(book_online, false)")
            .bind(tenant)
            .bind(practitioner)
            .fetch_optional(&mut **tx)
            .await?;
    let who = ok.ok_or_else(|| Error::NotFound("practitioner".into()))?;
    let mins: Option<Option<i64>> =
        sqlx::query_scalar("select duration from e_appointment_type where tenant_id = $1 and id = $2 and coalesce(book_online, false)")
            .bind(tenant)
            .bind(kind)
            .fetch_optional(&mut **tx)
            .await?;
    let mins = mins.ok_or_else(|| Error::NotFound("visit type".into()))?.unwrap_or(15).max(5);
    Ok((who, mins))
}

/// The free start times of a practitioner's day for a visit of `mins`: within their hours that
/// weekday, clashing with none of their visits, and after `now` when the day is today.
async fn free_times(k: &Kernel, ctx: &Ctx, tx: &mut Tx, practitioner: Uuid, day: NaiveDate, mins: i64, now: NaiveDateTime) -> Result<Vec<String>> {
    let doc = k.get_tx(ctx, tx, "Practitioner", practitioner, false).await?;
    let weekday = WEEKDAYS[day.weekday().num_days_from_monday() as usize];
    let taken: Vec<(i64, i64)> = sqlx::query_as(
        "select starts_min, ends_min from e_appointment where tenant_id = $1 and practitioner = $2 and \"date\" = $3 and status in ('Scheduled', 'Checked In')",
    )
    .bind(ctx.tenant)
    .bind(practitioner)
    .bind(day)
    .fetch_all(&mut **tx)
    .await?;
    let mut out = Vec::new();
    for h in rows(&doc, "hours").iter().filter(|h| s(h, "day") == Some(weekday)) {
        let (Some(from), Some(to)) = (s(h, "from_time").and_then(minutes), s(h, "to_time").and_then(minutes)) else { continue };
        let mut start = from;
        while start + mins <= to {
            let begins = day.and_hms_opt(0, 0, 0).map(|m| m + Duration::minutes(start));
            let free = taken.iter().all(|(a, b)| start >= *b || start + mins <= *a);
            if free && begins.is_some_and(|b| b > now) {
                out.push(hhmm(start));
            }
            start += mins;
        }
    }
    out.sort();
    out.dedup();
    Ok(out)
}

fn day_asked(v: Option<&str>, today: NaiveDate) -> Result<NaiveDate> {
    let day = v
        .and_then(|x| NaiveDate::parse_from_str(x, "%Y-%m-%d").ok())
        .ok_or_else(|| Error::Validation(vec![FieldError::new("date", "is a date, YYYY-MM-DD")]))?;
    if day < today || (day - today).num_days() > AHEAD_DAYS {
        return Err(Error::Validation(vec![FieldError::new("date", format!("is from {today} to {AHEAD_DAYS} days ahead"))]));
    }
    Ok(day)
}

/// `GET /book/slots?practitioner=&type=&date=`
async fn slots(k: &Kernel, ctx: &Ctx, req: &SiteRequest) -> Result<Value> {
    let q = |key: &str| req.query.get(key).and_then(Value::as_str).map(str::trim).filter(|v| !v.is_empty());
    let mut tx = k.db.tenant_tx(ctx.tenant).await?;
    open(&mut tx, ctx.tenant).await?;
    let (who, mins) = bookable(&mut tx, ctx.tenant, q("practitioner").and_then(|v| v.parse().ok()), q("type").and_then(|v| v.parse().ok())).await?;
    let now = ctx.now_local().naive_local();
    let day = day_asked(q("date"), now.date())?;
    let times = free_times(k, ctx, &mut tx, who, day, mins, now).await?;
    tx.commit().await?;
    Ok(json!({"date": day, "minutes": mins, "times": times}))
}

/// What a patient sees of a visit: nothing clinical.
async fn view(k: &Kernel, ctx: &Ctx, tx: &mut Tx, visit: &Doc, cancel_hours: i64) -> Result<Value> {
    let names: Option<(Option<String>, Option<String>)> = sqlx::query_as(
        "select pr.practitioner_name, t.type_name from e_practitioner pr, e_appointment_type t where pr.tenant_id = $1 and pr.id = $2 and t.tenant_id = $1 and t.id = $3",
    )
    .bind(ctx.tenant)
    .bind(id(visit, "practitioner"))
    .bind(id(visit, "appointment_type"))
    .fetch_optional(&mut **tx)
    .await?;
    let (practitioner, kind) = names.unwrap_or_default();
    let _ = k;
    Ok(
        json!({"id": s(visit, "id"), "number": s(visit, "number"), "practitioner": practitioner, "type": kind, "date": s(visit, "date"), "time": s(visit, "time"),
              "status": s(visit, "status"), "cancellable": cancellable(ctx, visit, cancel_hours)}),
    )
}

/// Whether a patient may still cancel a visit: scheduled, and the settings' hours or more away.
pub(crate) fn cancellable(ctx: &Ctx, visit: &Doc, hours: i64) -> bool {
    let starts = s(visit, "date")
        .and_then(|d| NaiveDate::parse_from_str(d.get(..10).unwrap_or(d), "%Y-%m-%d").ok())
        .and_then(|d| d.and_hms_opt(0, 0, 0))
        .zip(visit.get("starts_min").and_then(Value::as_i64))
        .map(|(d, m)| d + Duration::minutes(m));
    s(visit, "status") == Some("Scheduled") && starts.is_some_and(|b| b - Duration::hours(hours) >= ctx.now_local().naive_local())
}

/// `POST /book {practitioner, type, date, time, name, email, mobile}`: a visit for the patient known
/// by that email, or a new one; the same rules as at the desk (hours, clashes).
async fn book(k: &Kernel, ctx: &Ctx, body: &Map<String, Value>) -> Result<(u16, Value)> {
    if body.get("website").is_some_and(|v| !v.is_null() && v.as_str() != Some("")) {
        return Ok((201, json!({"status": "received"})));
    }
    let mut errors = Vec::new();
    let name = text(body, "name", 200, &mut errors);
    let email = text(body, "email", 254, &mut errors).map(|e| e.to_lowercase());
    let mobile = text(body, "mobile", 40, &mut errors);
    let time = text(body, "time", 5, &mut errors);
    if name.is_none() {
        errors.push(FieldError::new("name", "is required"));
    }
    if !email.as_deref().is_some_and(|e| e.contains('@') && !e.contains(char::is_whitespace)) {
        errors.push(FieldError::new("email", "is an email address, where the visit is confirmed and reminded"));
    }
    if time.as_deref().is_none_or(|t| minutes(t).is_none()) {
        errors.push(FieldError::new("time", "is HH:MM"));
    }
    if !errors.is_empty() {
        return Err(Error::Validation(errors));
    }
    with_retry(|| async {
        let mut tx = k.db.tenant_tx(ctx.tenant).await?;
        let hours = open(&mut tx, ctx.tenant).await?;
        let (who, mins) = bookable(&mut tx, ctx.tenant, uuid_of(body.get("practitioner")), uuid_of(body.get("type"))).await?;
        let now = ctx.now_local().naive_local();
        let day = day_asked(body.get("date").and_then(Value::as_str), now.date())?;
        let time = time.clone().unwrap_or_default();
        if !free_times(k, ctx, &mut tx, who, day, mins, now).await?.contains(&hhmm(minutes(&time).unwrap_or(-1))) {
            return Err(Error::Validation(vec![FieldError::new("time", "is not free: choose one of the times offered")]));
        }
        let known: Option<Uuid> =
            sqlx::query_scalar("select id from e_patient where tenant_id = $1 and lower(email) = $2 and status = 'Active' order by created_at limit 1")
                .bind(ctx.tenant)
                .bind(&email)
                .fetch_optional(&mut *tx)
                .await?;
        let patient = match known {
            Some(p) => p,
            None => {
                let made = k.insert_tx(ctx, &mut tx, "Patient", obj(json!({"patient_name": name, "email": email, "mobile": mobile}))).await?;
                let p = id(&made, "id").ok_or_else(|| Error::internal("a patient without an id"))?;
                k.set_values_tx(ctx, &mut tx, "Patient", p, obj(json!({"source": "Online"}))).await?;
                p
            }
        };
        let input = json!({"patient": patient.to_string(), "practitioner": who.to_string(), "appointment_type": body.get("type"), "date": day.to_string(), "time": time});
        let visit = k.insert_tx(ctx, &mut tx, "Appointment", obj(input)).await?;
        let vid = id(&visit, "id").ok_or_else(|| Error::internal("a visit without an id"))?;
        let token = URL_SAFE_NO_PAD.encode(rand::random::<[u8; 32]>());
        k.set_values_tx(ctx, &mut tx, "Appointment", vid, obj(json!({"source": "Online", "token_hash": hash(&token)}))).await?;
        let visit = k.get_tx(ctx, &mut tx, "Appointment", vid, false).await?;
        let shown = view(k, ctx, &mut tx, &visit, hours).await?;
        tx.commit().await?;
        Ok((201, json!({"visit": shown, "token": token})))
    })
    .await
}

async fn by_token(tx: &mut Tx, tenant: Uuid, tok: &str) -> Result<Uuid> {
    let shaped = tok.len() == 43 && tok.bytes().all(|c| c.is_ascii_alphanumeric() || c == b'-' || c == b'_');
    let found: Option<Uuid> = if shaped {
        sqlx::query_scalar("select id from e_appointment where tenant_id = $1 and token_hash = $2")
            .bind(tenant)
            .bind(hash(tok))
            .fetch_optional(&mut **tx)
            .await?
    } else {
        None
    };
    found.ok_or_else(|| Error::NotFound("visit".into()))
}

/// `GET /manage/{token}`
async fn manage(k: &Kernel, ctx: &Ctx, tok: &str) -> Result<Value> {
    let mut tx = k.db.tenant_tx(ctx.tenant).await?;
    let (_, hours) = settings(&mut tx, ctx.tenant).await?;
    let vid = by_token(&mut tx, ctx.tenant, tok).await?;
    let visit = k.get_tx(ctx, &mut tx, "Appointment", vid, false).await?;
    let shown = view(k, ctx, &mut tx, &visit, hours).await?;
    tx.commit().await?;
    Ok(json!({"visit": shown}))
}

/// Cancels `vid` for its patient, while it may still be.
async fn cancel_visit(k: &Kernel, ctx: &Ctx, tx: &mut Tx, vid: Uuid) -> Result<Value> {
    let (_, hours) = settings(tx, ctx.tenant).await?;
    let visit = k.get_tx(ctx, tx, "Appointment", vid, true).await?;
    if !cancellable(ctx, &visit, hours) {
        return Err(Error::invalid(format!(
            "this visit is not cancelled online: it is {} or less than {hours} hours away; call the clinic",
            s(&visit, "status").unwrap_or("closed").to_lowercase()
        )));
    }
    k.run_action_tx(ctx, tx, "Appointment", vid, "cancel_appointment", &Value::Null).await?;
    let visit = k.get_tx(ctx, tx, "Appointment", vid, false).await?;
    view(k, ctx, tx, &visit, hours).await
}

/// `POST /manage/{token}/cancel`
async fn cancel_by_link(k: &Kernel, ctx: &Ctx, tok: &str) -> Result<Value> {
    with_retry(|| async {
        let mut tx = k.db.tenant_tx(ctx.tenant).await?;
        let vid = by_token(&mut tx, ctx.tenant, tok).await?;
        let shown = cancel_visit(k, ctx, &mut tx, vid).await?;
        tx.commit().await?;
        Ok(json!({"visit": shown}))
    })
    .await
}

/// The patients known by `email`.
async fn patients_of(tx: &mut Tx, tenant: Uuid, email: &str) -> Result<Vec<(Uuid, String, Option<Uuid>)>> {
    Ok(sqlx::query_as("select id, patient_name, customer from e_patient where tenant_id = $1 and lower(email) = $2 order by patient_name limit 20")
        .bind(tenant)
        .bind(email)
        .fetch_all(&mut **tx)
        .await?)
}

/// `GET /portal/visits`: each patient known by the caller's address, with their visits, invoices
/// and the consents their coming visits need.
async fn visits(k: &Kernel, user: &Ctx) -> Result<Value> {
    let email = user.email.trim().to_lowercase();
    if email.is_empty() {
        return Ok(json!({"patients": []}));
    }
    let sctx = sys(user);
    let mut tx = k.db.tenant_tx(user.tenant).await?;
    let (_, hours) = settings(&mut tx, user.tenant).await?;
    let mut out = Vec::new();
    for (patient, name, customer) in patients_of(&mut tx, user.tenant, &email).await? {
        let ids: Vec<Uuid> =
            sqlx::query_scalar("select id from e_appointment where tenant_id = $1 and patient = $2 order by date desc, starts_min desc limit 50")
                .bind(user.tenant)
                .bind(patient)
                .fetch_all(&mut *tx)
                .await?;
        let mut shown = Vec::new();
        for v in ids {
            let visit = k.get_tx(&sctx, &mut tx, "Appointment", v, false).await?;
            shown.push(view(k, &sctx, &mut tx, &visit, hours).await?);
        }
        let needed: Vec<(Uuid, String, Option<i64>, String)> = sqlx::query_as(
            "select distinct f.id, f.form_name, f.revision, f.text from e_appointment a join e_appointment_type t on t.tenant_id = a.tenant_id and t.id = a.appointment_type \
             join e_consent_form f on f.tenant_id = t.tenant_id and f.id = t.consent_form \
             where a.tenant_id = $1 and a.patient = $2 and a.status = 'Scheduled' \
             and not exists (select 1 from e_patient_consent c where c.tenant_id = a.tenant_id and c.patient = a.patient and c.consent_form = f.id \
                             and c.status = 'Given' and c.form_version = f.revision)",
        )
        .bind(user.tenant)
        .bind(patient)
        .fetch_all(&mut *tx)
        .await?;
        let invoices: Vec<(Uuid, Option<String>, NaiveDate, rust_decimal::Decimal, rust_decimal::Decimal)> = match customer {
            Some(c) => sqlx::query_as(
                "select id, number, posting_date, grand_total, outstanding_amount from e_sales_invoice where tenant_id = $1 and customer = $2 and docstatus = 1 \
                 and coalesce(is_return, false) = false order by posting_date desc limit 50",
            )
            .bind(user.tenant)
            .bind(c)
            .fetch_all(&mut *tx)
            .await?,
            None => vec![],
        };
        out.push(json!({
            "id": patient, "name": name, "visits": shown,
            "consents_needed": needed.into_iter().map(|(f, n, v, text)| json!({"form": f, "name": n, "version": v, "text": text})).collect::<Vec<_>>(),
            "invoices": invoices.into_iter().map(|(i, n, on, total, owed)| json!({"id": i, "number": n, "posting_date": on, "total": total.to_string(), "outstanding": owed.to_string()})).collect::<Vec<_>>(),
        }));
    }
    tx.commit().await?;
    Ok(json!({"patients": out}))
}

/// `POST /portal/cancel {visit}`: one of the caller's patients' visits, cancelled as on its link.
async fn cancel_mine(k: &Kernel, user: &Ctx, body: &Map<String, Value>) -> Result<Value> {
    let vid = uuid_of(body.get("visit")).ok_or_else(|| Error::field("visit", "is required"))?;
    let email = user.email.trim().to_lowercase();
    let sctx = sys(user);
    with_retry(|| async {
        let mut tx = k.db.tenant_tx(user.tenant).await?;
        let mine: Vec<Uuid> = patients_of(&mut tx, user.tenant, &email).await?.into_iter().map(|p| p.0).collect();
        let of: Option<Uuid> = sqlx::query_scalar("select patient from e_appointment where tenant_id = $1 and id = $2")
            .bind(user.tenant)
            .bind(vid)
            .fetch_optional(&mut *tx)
            .await?;
        if email.is_empty() || !of.is_some_and(|p| mine.contains(&p)) {
            return Err(Error::NotFound("visit".into()));
        }
        let shown = cancel_visit(k, &sctx, &mut tx, vid).await?;
        tx.commit().await?;
        Ok(json!({"visit": shown}))
    })
    .await
}

/// `POST /portal/consent {patient, form}`: the patient's consent to a form's current version, given
/// on the portal by the person signed in.
async fn consent(k: &Kernel, user: &Ctx, body: &Map<String, Value>) -> Result<Value> {
    let patient = uuid_of(body.get("patient")).ok_or_else(|| Error::field("patient", "is required"))?;
    let form = uuid_of(body.get("form")).ok_or_else(|| Error::field("form", "is required"))?;
    let email = user.email.trim().to_lowercase();
    let sctx = sys(user);
    with_retry(|| async {
        let mut tx = k.db.tenant_tx(user.tenant).await?;
        let mine = patients_of(&mut tx, user.tenant, &email).await?;
        let Some((_, name, _)) = mine.iter().find(|p| p.0 == patient).filter(|_| !email.is_empty()) else {
            return Err(Error::NotFound("patient".into()));
        };
        let input = json!({"patient": patient.to_string(), "consent_form": form.to_string(), "given_by": format!("{name}, on the portal as {email}")});
        let given = k.insert_tx(&sctx, &mut tx, "PatientConsent", obj(input)).await?;
        tx.commit().await?;
        Ok(json!({"consent": given.get("number").or(given.get("id")), "version": given.get("form_version"), "message": "Consent given"}))
    })
    .await
}
