//! Insurance claims: an insurer's share of a visit, billed to it through ERP when the visit is
//! completed. Upkeep marks a claim paid once its invoice is submitted and settled. Rejected by the
//! insurer, the share is the patient's: their draft invoice is made for it, and the insurer's
//! invoice dropped while a draft, or credited (a draft credit note) once issued.

use crate::appointments::customer_of;
use crate::obj;
use async_trait::async_trait;
use ferrum_domain::flow::created;
use ferrum_domain::util::{d, elevated, id, money, rows, s};
use ferrum_kernel::{Ctx, Doc, DocHook, Error, HookCtx, Kernel, Result, Tx};
use serde_json::{Value, json};
use uuid::Uuid;

pub struct ClaimHook;

#[async_trait]
impl DocHook for ClaimHook {
    fn name(&self) -> &'static str {
        "healthcare.claim"
    }

    async fn action(&self, hc: &mut HookCtx<'_>, name: &str, doc: &Doc, params: &Value) -> Result<Option<Value>> {
        if name != "reject" {
            return Ok(None);
        }
        let reason = params.get("reason").and_then(Value::as_str).map(str::trim).filter(|r| !r.is_empty()).ok_or_else(|| Error::field("reason", "say why"))?;
        let claim = id(doc, "id").ok_or_else(|| Error::internal("a claim without an id"))?;
        let system = elevated(hc.ctx);
        let theirs = id(doc, "sales_invoice").ok_or_else(|| Error::internal("a claim without its invoice"))?;
        let inv = hc.kernel.get_tx(&system, hc.tx, "SalesInvoice", theirs, true).await?;
        let amount = d(doc, "amount");
        let words = format!("The insurer's share of {}, refused: {reason}", s(doc, "number").unwrap_or("a claim"));
        // The share moves to the patient.
        let patient = id(doc, "patient").ok_or_else(|| Error::internal("a claim without its patient"))?;
        let customer = customer_of(hc.kernel, &system, hc.tx, patient).await?;
        let items: Vec<Value> = rows(&inv, "items")
            .iter()
            .map(|l| json!({"item": s(l, "item"), "qty": "1", "price_list_rate": money(amount), "discount_percent": "0", "description": words}))
            .take(1)
            .collect();
        let mine = hc
            .kernel
            .insert_tx(
                &system,
                hc.tx,
                "SalesInvoice",
                obj(
                    json!({"company": s(&inv, "company"), "customer": customer.to_string(), "currency": s(&inv, "currency"), "remarks": words, "items": items}),
                ),
            )
            .await?;
        let mut values = obj(json!({"status": "Rejected", "rejected_reason": reason, "patient_invoice": mine.get("id")}));
        match inv.get("docstatus").and_then(Value::as_i64) {
            // Never sent: the insurer's draft goes, once nothing links to it.
            Some(0) => {
                hc.kernel.set_values_tx(&system, hc.tx, "InsuranceClaim", claim, obj(json!({"sales_invoice": null}))).await?;
                drop_links(hc, theirs).await?;
                hc.kernel.delete_tx(&system, hc.tx, "SalesInvoice", theirs, None).await?;
            }
            // Issued: the insurer is credited it.
            _ => {
                let item = rows(&inv, "items").first().and_then(|l| s(l, "item").map(str::to_string));
                let note = hc
                    .kernel
                    .insert_tx(
                        &system,
                        hc.tx,
                        "SalesInvoice",
                        obj(json!({"company": s(&inv, "company"), "customer": s(&inv, "customer"), "currency": s(&inv, "currency"), "is_return": true,
                                   "return_against": theirs.to_string(), "remarks": words,
                                   "items": [{"item": item, "qty": "-1", "price_list_rate": money(amount), "discount_percent": "0", "description": words}]})),
                    )
                    .await?;
                values.insert("credit_note".into(), note.get("id").cloned().unwrap_or(Value::Null));
            }
        }
        hc.kernel.set_values_tx(&system, hc.tx, "InsuranceClaim", claim, values).await?;
        Ok(Some(json!({"status": "Rejected", "patient_invoice": created("SalesInvoice", &mine),
                       "message": format!("Rejected: the patient is billed {:.2} instead (a draft)", amount)})))
    }
}

/// Other records' links to a draft invoice about to go: a fee validity's.
async fn drop_links(hc: &mut HookCtx<'_>, invoice: Uuid) -> Result<()> {
    let holders: Vec<Uuid> = sqlx::query_scalar("select id from e_fee_validity where tenant_id = $1 and sales_invoice = $2")
        .bind(hc.ctx.tenant)
        .bind(invoice)
        .fetch_all(&mut **hc.tx)
        .await?;
    for v in holders {
        hc.kernel.set_values_tx(&elevated(hc.ctx), hc.tx, "FeeValidity", v, obj(json!({"sales_invoice": null}))).await?;
    }
    Ok(())
}

/// Upkeep: claims whose insurer's invoice is submitted and settled are paid.
pub(crate) async fn settle(k: &Kernel, ctx: &Ctx, tx: &mut Tx) -> Result<usize> {
    let paid: Vec<Uuid> = sqlx::query_scalar(
        "select c.id from e_insurance_claim c join e_sales_invoice i on i.tenant_id = c.tenant_id and i.id = c.sales_invoice \
         where c.tenant_id = $1 and c.status = 'Submitted' and i.docstatus = 1 and coalesce(i.outstanding_amount, 0) <= 0",
    )
    .bind(ctx.tenant)
    .fetch_all(&mut **tx)
    .await?;
    for claim in &paid {
        k.set_values_tx(ctx, tx, "InsuranceClaim", *claim, obj(json!({"status": "Paid"}))).await?;
    }
    Ok(paid.len())
}
