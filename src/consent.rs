//! Consent. A form is at version 1 when made, and its next version whenever its text changes; a
//! consent is to the version current when it was given, and may be withdrawn. A patient has
//! consented to a form when they gave, and have not withdrawn, a consent to its current version.

use crate::obj;
use async_trait::async_trait;
use chrono::Utc;
use ferrum_domain::util::{elevated, id, s};
use ferrum_kernel::{Doc, DocHook, Error, HookCtx, Result, Tx};
use serde_json::{Value, json};
use uuid::Uuid;

pub struct FormHook;

#[async_trait]
impl DocHook for FormHook {
    fn name(&self) -> &'static str {
        "healthcare.consent_form"
    }

    fn fills(&self) -> &'static [&'static str] {
        &["revision"]
    }

    async fn before_validate(&self, hc: &mut HookCtx<'_>, doc: &mut Doc) -> Result<()> {
        let stored: Option<(Option<i64>, Option<String>)> = match id(doc, "id") {
            Some(form) => {
                sqlx::query_as("select revision, text from e_consent_form where tenant_id = $1 and id = $2")
                    .bind(hc.ctx.tenant)
                    .bind(form)
                    .fetch_optional(&mut **hc.tx)
                    .await?
            }
            None => None,
        };
        let version = match stored {
            None => 1,
            Some((v, text)) if text.as_deref() != s(doc, "text") => v.unwrap_or(0) + 1,
            Some((v, _)) => v.unwrap_or(1),
        };
        doc.insert("revision".into(), json!(version));
        Ok(())
    }
}

/// Whether `patient` has consented to `form` at its current version.
pub(crate) async fn consented(tx: &mut Tx, tenant: Uuid, patient: Uuid, form: Uuid) -> Result<bool> {
    Ok(sqlx::query_scalar(
        "select exists(select 1 from e_patient_consent c join e_consent_form f on f.tenant_id = c.tenant_id and f.id = c.consent_form \
         where c.tenant_id = $1 and c.patient = $2 and c.consent_form = $3 and c.status = 'Given' and c.form_version = f.revision)",
    )
    .bind(tenant)
    .bind(patient)
    .bind(form)
    .fetch_one(&mut **tx)
    .await?)
}

pub struct ConsentHook;

#[async_trait]
impl DocHook for ConsentHook {
    fn name(&self) -> &'static str {
        "healthcare.consent"
    }

    fn fills(&self) -> &'static [&'static str] {
        &["form_version", "given_at", "given_by"]
    }

    async fn before_validate(&self, hc: &mut HookCtx<'_>, doc: &mut Doc) -> Result<()> {
        if doc.get("form_version").is_some_and(|v| !v.is_null()) {
            return Ok(());
        }
        // Given on the portal, it is given by whoever signed in there, whatever they typed.
        if hc.ctx.outside {
            doc.insert("given_by".into(), json!(format!("{}, on the portal as {}", hc.ctx.full_name, hc.ctx.email)));
        }
        let form = id(doc, "consent_form").ok_or_else(|| Error::field("consent_form", "is required"))?;
        let version: Option<i64> = sqlx::query_scalar("select revision from e_consent_form where tenant_id = $1 and id = $2")
            .bind(hc.ctx.tenant)
            .bind(form)
            .fetch_optional(&mut **hc.tx)
            .await?
            .flatten();
        doc.insert("form_version".into(), json!(version.unwrap_or(1)));
        doc.insert("given_at".into(), json!(Utc::now().to_rfc3339()));
        Ok(())
    }

    async fn validate_change(&self, hc: &mut HookCtx<'_>, old: Option<&Doc>, doc: &Doc) -> Result<()> {
        if hc.ctx.is_system {
            return Ok(());
        }
        if let Some(o) = old
            && ["patient", "consent_form", "given_by"].iter().any(|k| s(o, k) != s(doc, k))
        {
            return Err(Error::invalid("a consent is kept as it was given: withdraw it and take another"));
        }
        Ok(())
    }

    async fn action(&self, hc: &mut HookCtx<'_>, name: &str, doc: &Doc, _params: &Value) -> Result<Option<Value>> {
        if name != "withdraw" {
            return Ok(None);
        }
        let consent = id(doc, "id").ok_or_else(|| Error::internal("a consent without an id"))?;
        hc.kernel
            .set_values_tx(&elevated(hc.ctx), hc.tx, "PatientConsent", consent, obj(json!({"status": "Withdrawn", "withdrawn_at": Utc::now().to_rfc3339()})))
            .await?;
        Ok(Some(json!({"status": "Withdrawn"})))
    }
}
