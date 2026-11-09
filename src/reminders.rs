//! Upkeep: claims whose insurer's invoice is settled are paid, and patients are reminded by email
//! of their visits starting within the settings' hours, once each.

use crate::obj;
use chrono::{Duration, NaiveDate, NaiveDateTime, Utc};
use ferrum_kernel::{Ctx, Kernel, Result};
use ferrum_meta::effective::Schema;
use serde_json::json;
use std::sync::Arc;
use uuid::Uuid;

fn escape(v: &str) -> String {
    v.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;").replace('"', "&quot;")
}

/// One workspace's upkeep, its clock reading `now` (local time), or its own.
pub(crate) async fn upkeep(k: &Kernel, tenant: Uuid, schema: Arc<Schema>, now: Option<NaiveDateTime>) -> Result<()> {
    let ctx = Ctx::system(tenant, schema);
    let now = now.unwrap_or_else(|| ctx.now_local().naive_local());
    let mut tx = k.db.tenant_tx(tenant).await?;
    crate::claims::settle(k, &ctx, &mut tx).await?;
    let hours: Option<i64> = sqlx::query_scalar("select reminder_hours from e_healthcare_settings where tenant_id = $1 limit 1")
        .bind(tenant)
        .fetch_optional(&mut *tx)
        .await?
        .flatten();
    if let Some(hours) = hours.filter(|h| *h > 0) {
        let until = now + Duration::hours(hours);
        let due: Vec<(Uuid, NaiveDate, String, String, Option<String>, Option<String>)> = sqlx::query_as(
            "select a.id, a.date, a.time, p.email, p.patient_name, pr.practitioner_name from e_appointment a \
             join e_patient p on p.tenant_id = a.tenant_id and p.id = a.patient \
             left join e_practitioner pr on pr.tenant_id = a.tenant_id and pr.id = a.practitioner \
             where a.tenant_id = $1 and a.status = 'Scheduled' and a.reminded_at is null and coalesce(p.email, '') <> '' \
             and a.date + make_interval(mins => a.starts_min::int) > $2 and a.date + make_interval(mins => a.starts_min::int) <= $3 order by a.date, a.starts_min",
        )
        .bind(tenant)
        .bind(now)
        .bind(until)
        .fetch_all(&mut *tx)
        .await?;
        for (visit, day, time, email, patient, practitioner) in due {
            let subject = format!("Your visit on {day} at {time}");
            let html = format!(
                "<p>Hello {},</p><p>A reminder of your visit with {} on {day} at {time}.</p>",
                escape(patient.as_deref().unwrap_or("there")),
                escape(practitioner.as_deref().unwrap_or("your practitioner"))
            );
            k.queue_mail(&mut tx, tenant, &email, &subject, &html).await?;
            k.set_values_tx(&ctx, &mut tx, "Appointment", visit, obj(json!({"reminded_at": Utc::now().to_rfc3339()}))).await?;
        }
    }
    tx.commit().await?;
    Ok(())
}
