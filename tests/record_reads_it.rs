//! Who opened a patient's record, against a real PostgreSQL (`FERRUM_HEALTHCARE_TEST_DATABASE_URL`, default
//! `postgres://ferrum:ferrum@localhost:5432/ferrum_healthcare_test`; reset first): on a workspace that set
//! nothing, a staff member opening a patient, a visit, a consent or a claim leaves an `open` event on the audit trail
//! (category `access`) and a list leaves none; the workspace's own `record_reads` wins, `off` for one entity stopping
//! it for that entity only and `all` adding lists; and what the patient's own portal reads.

use ferrum_kernel::tenant::NewTenant;
use ferrum_kernel::zones::{Credential, Entry, Placed, Zones};
use ferrum_kernel::{Ctx, Db, DbConfig, Doc, HookRegistry, Kernel, KernelSettings, SiteRequest};
use ferrum_meta::Catalog;
use serde_json::{Value, json};
use std::sync::Arc;
use uuid::Uuid;

fn obj(v: Value) -> Doc {
    v.as_object().cloned().unwrap()
}

fn uid(v: &Value) -> Uuid {
    v.as_str().unwrap_or_else(|| panic!("not an id: {v}")).parse().unwrap()
}

#[test]
fn reads_of_a_patients_records_are_recorded_unless_the_workspace_says_otherwise() {
    let rt = tokio::runtime::Builder::new_multi_thread().enable_all().thread_stack_size(16 << 20).build().unwrap();
    rt.block_on(async { tokio::spawn(scenario()).await.unwrap() });
}

/// What the audit trail holds in category `access`: `(entity, action)`, oldest first.
async fn trail(k: &Kernel, tenant: Uuid) -> Vec<(String, String)> {
    let mut tx = k.db.system_tx().await.unwrap();
    let rows = sqlx::query_as("select diff->>'entity', action from audit_event where tenant_id = $1 and category = 'access' order by id")
        .bind(tenant)
        .fetch_all(&mut *tx)
        .await
        .unwrap();
    tx.commit().await.unwrap();
    rows
}

/// What one request of `user`'s, opening the record and then listing its entity, wrote to the trail.
async fn reads_recorded(k: &Kernel, tenant: Uuid, user: Uuid, entity: &str, id: Uuid) -> Vec<(String, String)> {
    let before = trail(k, tenant).await.len();
    let ctx = request(k, tenant, user).await;
    k.get(&ctx, entity, id, false).await.unwrap_or_else(|e| panic!("get {entity}: {e:?}"));
    k.list(&ctx, entity, &Default::default()).await.unwrap_or_else(|e| panic!("list {entity}: {e:?}"));
    trail(k, tenant).await.split_off(before)
}

async fn request(k: &Kernel, tenant: Uuid, user: Uuid) -> Ctx {
    let entry = Arc::new(Entry { zones: Arc::new(Zones::default()), placed: Placed::Nowhere, address: "10.1.2.3".into(), credential: Credential::Cookie });
    k.request_ctx(tenant, user, None, "it".into(), &entry).await.expect("a request")
}

async fn scenario() {
    let url = std::env::var("FERRUM_HEALTHCARE_TEST_DATABASE_URL").unwrap_or_else(|_| "postgres://ferrum:ferrum@localhost:5432/ferrum_healthcare_test".into());
    let db = Db::connect(DbConfig { url, max_connections: 8, ..Default::default() }).await.expect("connect");
    sqlx::raw_sql("drop schema if exists public cascade; create schema public;").execute(&db.pool).await.expect("reset");
    let mut packages = ferrum_domain::packages().expect("platform packages");
    packages.extend(ferrum_app_erp::packages().expect("ERP's packages"));
    packages.extend(ferrum_app_healthcare::packages().expect("app packages"));
    let catalog = Catalog::from_packages(packages).expect("catalog");
    let mut hooks = HookRegistry::default();
    ferrum_domain::register(&mut hooks);
    hooks.app("erp", &[], ferrum_app_erp::register);
    ferrum_app_healthcare::register(&mut hooks);
    let k = Kernel::boot(db, catalog, hooks, KernelSettings::default(), None).await.expect("boot");
    ferrum_domain::maintenance(&k).await.expect("partitions");
    // A workspace that sets nothing about reads.
    let tenant = k
        .create_tenant(NewTenant {
            slug: "clinic".into(),
            name: "Lakeside Clinic".into(),
            admin_email: "admin@clinic.test".into(),
            admin_password: "Correct-Horse-9".into(),
            admin_name: None,
            packages: vec!["healthcare".into()],
            settings: Some(json!({"timezone": "Asia/Kolkata"})),
        })
        .await
        .expect("tenant");
    let session = k.login("clinic", "admin@clinic.test", "Correct-Horse-9").await.expect("login");
    let admin = k.load_ctx(tenant, session.user, Some(session.token_version), "it".into()).await.expect("ctx");
    assert!(k.tenant_settings(&admin).await.unwrap()["record_reads"].is_null(), "the workspace set nothing");
    let desk = k.create_user(&admin, "desk@clinic.test", "Desk", Some("Correct-Horse-9"), &["Clinic Reception".to_string()], false).await.expect("user");
    let manager =
        k.create_user(&admin, "manager@clinic.test", "Manager", Some("Correct-Horse-9"), &["Healthcare Manager".to_string()], false).await.expect("user");

    // A patient, a visit, a consent and a claim.
    let insert = async |entity: &str, v: Value| uid(&k.insert(&admin, entity, obj(v)).await.unwrap_or_else(|e| panic!("insert {entity}: {e:?}"))["id"]);
    let patient = insert("Patient", json!({"patient_name": "Cleo Park", "email": "cleo@home.test", "allergies": "penicillin"})).await;
    let doctor = insert(
        "Practitioner",
        json!({"practitioner_name": "Dr Mira Shah", "consultation_fee": "800", "hours": [{"day": "Monday", "from_time": "09:00", "to_time": "13:00"}]}),
    )
    .await;
    let kind = insert("AppointmentType", json!({"type_name": "General", "duration": 20})).await;
    // 2026-11-09 is a Monday.
    let visit = insert(
        "Appointment",
        json!({"patient": patient.to_string(), "practitioner": doctor.to_string(), "appointment_type": kind.to_string(), "date": "2026-11-09", "time": "09:00"}),
    )
    .await;
    let form = insert("ConsentForm", json!({"form_name": "Treatment", "text": "I agree."})).await;
    let consent = insert("PatientConsent", json!({"patient": patient.to_string(), "consent_form": form.to_string(), "given_by": "Cleo Park"})).await;
    // A claim is made by completing an insured visit, which is ERP's billing and not this test's business.
    let claim = insert("InsuranceClaim", json!({})).await;

    // ───────────── staff on a workspace that set nothing: each record opened is recorded, a list is not ─────────────
    let open = |entity: &str| vec![(entity.to_string(), "open".to_string())];
    for (entity, id) in [("Patient", patient), ("Appointment", visit), ("PatientConsent", consent), ("InsuranceClaim", claim)] {
        assert_eq!(reads_recorded(&k, tenant, desk, entity, id).await, open(entity), "{entity} opened by the front desk");
    }
    // What holds nobody's record is not recorded.
    for (entity, id) in [("Practitioner", doctor), ("AppointmentType", kind), ("ConsentForm", form)] {
        assert!(reads_recorded(&k, tenant, manager, entity, id).await.is_empty(), "{entity} holds no patient's record");
    }

    // ───────────── the workspace's own setting wins ─────────────
    let set = async |v: Value| k.update_tenant_settings(&admin, &obj(json!({"record_reads": v}))).await.unwrap();
    set(json!({"Patient": "off"})).await;
    assert!(reads_recorded(&k, tenant, desk, "Patient", patient).await.is_empty(), "the workspace switched Patient off");
    assert_eq!(reads_recorded(&k, tenant, desk, "Appointment", visit).await, open("Appointment"), "and only Patient");
    set(json!({"Patient": "all"})).await;
    let all = reads_recorded(&k, tenant, desk, "Patient", patient).await;
    assert_eq!(all.iter().map(|(_, action)| action.as_str()).collect::<Vec<_>>(), ["open", "list"], "raised to all: its lists too");
    set(Value::Null).await;
    assert_eq!(reads_recorded(&k, tenant, desk, "Patient", patient).await, open("Patient"), "the declaration again");

    // ───────────── the patient's own portal ─────────────
    let portal = k.create_user(&admin, "cleo@home.test", "Cleo", None, &[], false).await.unwrap();
    k.set_user_kind(&admin, portal, "portal").await.unwrap();
    k.set_user_roles_until(&admin, portal, &["Patient Portal".to_string()], &Default::default()).await.expect("the portal role");
    // The generic portal role reads the record through the kernel like anyone: recorded.
    assert_eq!(reads_recorded(&k, tenant, portal, "Patient", patient).await, open("Patient"), "the patient's own read through the portal role");
    // The app's own page, /portal/visits, reads by its own SQL and is not recorded.
    let before = trail(&k, tenant).await.len();
    let req = SiteRequest {
        tenant,
        user: Some(request(&k, tenant, portal).await),
        method: "GET".into(),
        path: "/portal/visits".into(),
        query: Default::default(),
        headers: Default::default(),
        body: vec![],
        client_ip: "198.51.100.4".into(),
    };
    let page = k.hooks.site("healthcare").expect("healthcare's endpoints").clone().handle(&k, req).await.expect("their visits");
    assert_eq!(serde_json::from_slice::<Value>(&page.body).unwrap()["patients"][0]["name"], "Cleo Park");
    assert_eq!(trail(&k, tenant).await.len(), before, "the app's portal page reads by its own SQL: nothing recorded");
}
