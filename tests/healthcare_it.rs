//! A week at a clinic (FERRUM-INDUSTRY-005) against a real PostgreSQL
//! (`FERRUM_HEALTHCARE_TEST_DATABASE_URL`, default `postgres://ferrum:ferrum@localhost:5432/ferrum_healthcare_test`;
//! reset first): practitioners' hours, appointments refused outside them or when either the
//! practitioner or the patient is booked then; consent to a form's current version needed to check
//! in, withdrawn, and asked again when the form changes; completing a visit only by its
//! practitioner, billing it through ERP to a customer made from the patient, and covering the
//! follow-ups within the settings' days; and who may do what.

use ferrum_kernel::secret::SecretKey;
use ferrum_kernel::tenant::NewTenant;
use ferrum_kernel::{Ctx, Db, DbConfig, Doc, Error, HookRegistry, Kernel, KernelSettings, ListQuery};
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

fn dec(v: &Value) -> f64 {
    v.as_str().map(|s| s.parse().unwrap()).or_else(|| v.as_f64()).unwrap_or_else(|| panic!("not a number: {v}"))
}

struct T {
    k: Arc<Kernel>,
    tenant: Uuid,
    admin: Ctx,
}

impl T {
    async fn insert(&self, ctx: &Ctx, entity: &str, v: Value) -> Doc {
        self.k.insert(ctx, entity, obj(v)).await.unwrap_or_else(|e| panic!("insert {entity}: {e:?}"))
    }
    async fn get(&self, entity: &str, id: Uuid) -> Doc {
        self.k.get(&self.admin, entity, id, false).await.unwrap_or_else(|e| panic!("get {entity}: {e:?}"))
    }
    async fn find_one(&self, entity: &str, filters: Value) -> Doc {
        let q = ListQuery { filters: ferrum_kernel::query::parse_filters(&filters).unwrap(), limit: Some(50), ..Default::default() };
        let rows = self.k.list(&self.admin, entity, &q).await.unwrap().rows;
        assert_eq!(rows.len(), 1, "{entity} {filters}");
        rows[0].clone()
    }
    async fn user(&self, email: &str, roles: &[&str]) -> Ctx {
        let roles: Vec<String> = roles.iter().map(|r| r.to_string()).collect();
        let id = self.k.create_user(&self.admin, email, email, Some("Correct-Horse-9"), &roles, false).await.expect("user");
        let s = self.k.login("clinic", email, "Correct-Horse-9").await.expect("login");
        self.k.load_ctx(self.tenant, id, Some(s.token_version), "it".into()).await.expect("ctx")
    }
    async fn act(&self, ctx: &Ctx, entity: &str, id: Uuid, action: &str) -> Result<Value, Error> {
        self.k.run_action(ctx, entity, id, action, Value::Null).await
    }
}

fn refused<V: std::fmt::Debug>(r: Result<V, Error>, kind: &str, says: &str) {
    let msg = format!("{r:?}");
    let ok = matches!(
        (&r, kind),
        (Err(Error::Invalid(_) | Error::Validation(_)), "invalid") | (Err(Error::Forbidden(_)), "forbidden") | (Err(Error::NotFound(_)), "not found")
    );
    assert!(ok && msg.contains(says), "expected {kind} saying {says:?}, got {msg}");
}

#[test]
fn a_week_at_a_clinic() {
    let rt = tokio::runtime::Builder::new_multi_thread().enable_all().thread_stack_size(16 << 20).build().unwrap();
    rt.block_on(async { tokio::spawn(scenario()).await.unwrap() });
}

async fn scenario() {
    let url = std::env::var("FERRUM_HEALTHCARE_TEST_DATABASE_URL").unwrap_or_else(|_| "postgres://ferrum:ferrum@localhost:5432/ferrum_healthcare_test".into());
    let db = Db::connect(DbConfig { url, max_connections: 16, ..Default::default() }).await.expect("connect");
    sqlx::raw_sql("drop schema if exists public cascade; create schema public;").execute(&db.pool).await.expect("reset");
    let mut packages = ferrum_domain::packages().expect("platform packages");
    packages.extend(ferrum_app_erp::packages().expect("ERP's packages"));
    packages.extend(ferrum_app_healthcare::packages().expect("app packages"));
    let catalog = Catalog::from_packages(packages).expect("catalog");
    let mut hooks = HookRegistry::default();
    ferrum_domain::register(&mut hooks);
    ferrum_app_erp::register(&mut hooks);
    ferrum_app_healthcare::register(&mut hooks);
    let settings = KernelSettings { secret_key: Some(SecretKey::new([7u8; 32])), ..Default::default() };
    let k = Kernel::boot(db, catalog, hooks, settings, None).await.expect("boot");
    ferrum_domain::maintenance(&k).await.expect("partitions");
    ferrum_app_erp::maintenance(&k).await.expect("partitions");
    let tenant = k
        .create_tenant(NewTenant {
            slug: "clinic".into(),
            name: "Lakeside Clinic".into(),
            admin_email: "admin@clinic.test".into(),
            admin_password: "Correct-Horse-9".into(),
            admin_name: None,
            packages: vec!["healthcare".into()],
            settings: Some(json!({"fiscal_year_start_month": 4, "timezone": "Asia/Kolkata"})),
        })
        .await
        .expect("tenant");
    let session = k.login("clinic", "admin@clinic.test", "Correct-Horse-9").await.expect("login");
    let admin = k.load_ctx(tenant, session.user, Some(session.token_version), "it".into()).await.expect("ctx");
    let t = T { k: k.clone(), tenant, admin };
    let a = t.admin.clone();

    // ───────────── the books, the practitioners and the forms ─────────────
    let inr = t.find_one("Currency", json!({"code": "INR"})).await;
    let company = uid(&t.insert(&a, "Company", json!({"company_name": "Lakeside Clinic", "abbr": "LC", "default_currency": inr["id"]})).await["id"]);
    let services = t.find_one("ItemGroup", json!({"group_name": "Services"})).await;
    let nos = t.find_one("Uom", json!({"uom_name": "Nos"})).await;
    let item = |code: &str| json!({"item_code": code, "item_name": code, "item_group": services["id"], "stock_uom": nos["id"], "is_stock_item": false});
    let consult = uid(&t.insert(&a, "Item", item("CONSULT")).await["id"]);
    let dental = uid(&t.insert(&a, "Item", item("DENTAL")).await["id"]);
    let manager = t.user("manager@clinic.test", &["Healthcare Manager"]).await;
    let desk = t.user("desk@clinic.test", &["Clinic Reception"]).await;
    let mira_u = t.user("mira@clinic.test", &["Practitioner"]).await;
    let omar_u = t.user("omar@clinic.test", &["Practitioner"]).await;
    let no_settings = t.insert(&manager, "Practitioner", json!({"practitioner_name": "Nobody", "consultation_fee": "1"})).await;
    let _ = no_settings;
    t.insert(
        &manager,
        "HealthcareSettings",
        json!({"company": company.to_string(), "consultation_item": consult.to_string(), "follow_up_days": 5, "follow_up_visits": 2}),
    )
    .await;
    let hours = |day: &str, from: &str, to: &str| json!({"day": day, "from_time": from, "to_time": to});
    refused(
        t.k.insert(
            &manager,
            "Practitioner",
            obj(json!({"practitioner_name": "Backwards", "consultation_fee": "500", "hours": [hours("Monday", "17:00", "09:00")]})),
        )
        .await,
        "invalid",
        "to a later HH:MM",
    );
    let mira = uid(&t
        .insert(
            &manager,
            "Practitioner",
            json!({"practitioner_name": "Dr Mira Shah", "email": "mira@clinic.test", "department": "General", "consultation_fee": "800",
                   "hours": [hours("Monday", "09:00", "13:00"), hours("Wednesday", "14:00", "18:00")]}),
        )
        .await["id"]);
    let omar = uid(&t
        .insert(
            &manager,
            "Practitioner",
            json!({"practitioner_name": "Dr Omar Ali", "email": "omar@clinic.test", "department": "Dental", "consultation_item": dental.to_string(), "consultation_fee": "1200",
                   "hours": [hours("Monday", "09:00", "17:00")]}),
        )
        .await["id"]);
    let general = uid(&t.insert(&manager, "AppointmentType", json!({"type_name": "General", "duration": 20})).await["id"]);
    let form = t.insert(&manager, "ConsentForm", json!({"form_name": "Dental treatment", "text": "I agree to the treatment explained to me."})).await;
    let form_id = uid(&form["id"]);
    assert_eq!(form["revision"], 1);
    let dentistry =
        uid(&t.insert(&manager, "AppointmentType", json!({"type_name": "Dentistry", "duration": 30, "consent_form": form_id.to_string()})).await["id"]);

    // ───────────── patients and appointments ─────────────
    let asha = uid(&t
        .insert(&desk, "Patient", json!({"patient_name": "Asha Rao", "sex": "Female", "mobile": "+91 90000 00001", "email": "asha@home.test"}))
        .await["id"]);
    let ben = uid(&t.insert(&desk, "Patient", json!({"patient_name": "Ben Ito", "sex": "Male"})).await["id"]);
    let gone = uid(&t.insert(&desk, "Patient", json!({"patient_name": "Old Patient", "status": "Inactive"})).await["id"]);
    // 2026-11-09 is a Monday, 2026-11-10 a Tuesday.
    let book = |patient: Uuid, practitioner: Uuid, kind: Uuid, day: &str, time: &str| {
        obj(json!({"patient": patient.to_string(), "practitioner": practitioner.to_string(), "appointment_type": kind.to_string(), "date": day, "time": time}))
    };
    let first = t.k.insert(&desk, "Appointment", book(asha, mira, general, "2026-11-09", "09:00")).await.expect("booked");
    assert_eq!(
        (first["duration"].as_i64(), first["starts_min"].as_i64(), first["ends_min"].as_i64()),
        (Some(20), Some(540), Some(560)),
        "the type's twenty minutes"
    );
    let first = uid(&first["id"]);
    refused(t.k.insert(&desk, "Appointment", book(ben, mira, general, "2026-11-10", "10:00")).await, "invalid", "does not see patients on Tuesdays");
    refused(t.k.insert(&desk, "Appointment", book(ben, mira, general, "2026-11-09", "12:50")).await, "invalid", "from 12:50 to 13:10");
    refused(t.k.insert(&desk, "Appointment", book(ben, mira, general, "2026-11-09", "08:50")).await, "invalid", "from 08:50 to 09:10");
    let away = uid(&t
        .insert(
            &manager,
            "Practitioner",
            json!({"practitioner_name": "Dr Away", "consultation_fee": "500", "active": false, "hours": [hours("Monday", "09:00", "17:00")]}),
        )
        .await["id"]);
    refused(t.k.insert(&desk, "Appointment", book(ben, away, general, "2026-11-09", "14:00")).await, "invalid", "Dr Away is not seeing patients");
    refused(t.k.insert(&desk, "Appointment", book(ben, mira, general, "2026-11-09", "09:10")).await, "invalid", "Dr Mira Shah is booked then");
    refused(t.k.insert(&desk, "Appointment", book(asha, omar, general, "2026-11-09", "09:15")).await, "invalid", "The patient is booked then");
    refused(t.k.insert(&desk, "Appointment", book(gone, mira, general, "2026-11-09", "11:00")).await, "invalid", "inactive");
    refused(t.k.insert(&desk, "Appointment", book(ben, mira, general, "2026-11-09", "9am")).await, "invalid", "HH:MM");
    let back_to_back = t.k.insert(&desk, "Appointment", book(ben, mira, general, "2026-11-09", "09:20")).await.expect("back to back");
    let ben_visit = uid(&back_to_back["id"]);
    // A cancelled visit frees its time.
    let spare = uid(&t.k.insert(&desk, "Appointment", book(ben, mira, general, "2026-11-09", "10:00")).await.unwrap()["id"]);
    t.act(&desk, "Appointment", spare, "cancel_appointment").await.unwrap();
    t.k.insert(&desk, "Appointment", book(asha, mira, general, "2026-11-09", "10:00")).await.expect("the cancelled time is free");

    // ───────────── consent ─────────────
    let tooth = uid(&t.k.insert(&desk, "Appointment", book(ben, omar, dentistry, "2026-11-09", "11:00")).await.unwrap()["id"]);
    refused(t.act(&desk, "Appointment", tooth, "check_in").await, "invalid", "has not consented to Dental treatment, version 1");
    let consent = t.insert(&desk, "PatientConsent", json!({"patient": ben.to_string(), "consent_form": form_id.to_string(), "given_by": "Ben Ito"})).await;
    assert_eq!((consent["form_version"].as_i64(), consent["status"].as_str()), (Some(1), Some("Given")));
    assert!(consent["given_at"].is_string());
    refused(t.k.update(&desk, "PatientConsent", uid(&consent["id"]), obj(json!({"given_by": "Someone"})), None).await, "invalid", "kept as it was given");
    t.act(&desk, "Appointment", tooth, "check_in").await.expect("with consent");
    // Withdrawn, the consent no longer counts; a new version of the form asks again.
    let later = uid(&t.k.insert(&desk, "Appointment", book(ben, omar, dentistry, "2026-11-09", "15:00")).await.unwrap()["id"]);
    t.act(&desk, "PatientConsent", uid(&consent["id"]), "withdraw").await.unwrap();
    refused(t.act(&desk, "Appointment", later, "check_in").await, "invalid", "has not consented");
    t.insert(&desk, "PatientConsent", json!({"patient": ben.to_string(), "consent_form": form_id.to_string(), "given_by": "Ben Ito"})).await;
    t.k.update(&manager, "ConsentForm", form_id, obj(json!({"text": "I agree to the treatment and to X-rays explained to me."})), None).await.unwrap();
    assert_eq!(t.get("ConsentForm", form_id).await["revision"], 2, "new text, a new version");
    t.k.update(&manager, "ConsentForm", form_id, obj(json!({"form_name": "Dental treatment and X-rays"})), None).await.unwrap();
    assert_eq!(t.get("ConsentForm", form_id).await["revision"], 2, "the same text, the same version");
    refused(t.act(&desk, "Appointment", later, "check_in").await, "invalid", "version 2");
    t.insert(&desk, "PatientConsent", json!({"patient": ben.to_string(), "consent_form": form_id.to_string(), "given_by": "Ben Ito"})).await;
    t.act(&desk, "Appointment", later, "check_in").await.expect("consented to version 2");

    // ───────────── completing and billing ─────────────
    t.act(&desk, "Appointment", first, "check_in").await.unwrap();
    refused(t.k.update(&desk, "Appointment", first, obj(json!({"time": "11:30"})), None).await, "invalid", "keeps its patient, practitioner and time");
    refused(t.act(&desk, "Appointment", first, "complete").await, "forbidden", "");
    refused(t.act(&omar_u, "Appointment", first, "complete").await, "forbidden", "the visit's practitioner");
    let done = t.act(&mira_u, "Appointment", first, "complete").await.expect("completed by Dr Shah");
    let inv = t.get("SalesInvoice", uid(&done["sales_invoice"]["id"])).await;
    assert_eq!(dec(&inv["grand_total"]), 800.0);
    assert_eq!(inv["items"][0]["item"], consult.to_string());
    let patient = t.get("Patient", asha).await;
    assert_eq!(inv["customer"], patient["customer"], "billed to the customer made from the patient");
    assert_eq!(t.get("Customer", uid(&patient["customer"])).await["customer_name"], "Asha Rao");
    let validity = t.find_one("FeeValidity", json!({"patient": asha.to_string(), "practitioner": mira.to_string()})).await;
    assert_eq!((validity["valid_till"].as_str(), validity["visits_left"].as_i64()), (Some("2026-11-14"), Some(2)), "the settings' five days");
    // Follow-ups within the week are covered, two of them; the third is billed.
    let mut follow = vec![];
    for (day, time) in [("2026-11-11", "14:00"), ("2026-11-11", "15:00"), ("2026-11-11", "16:00")] {
        let id = uid(&t.k.insert(&desk, "Appointment", book(asha, mira, general, day, time)).await.unwrap()["id"]);
        t.act(&desk, "Appointment", id, "check_in").await.unwrap();
        follow.push(t.act(&mira_u, "Appointment", id, "complete").await.unwrap());
    }
    assert!(follow[0]["covered_by"].is_string() && follow[1]["covered_by"].is_string(), "{follow:?}");
    assert!(follow[2]["sales_invoice"].is_object(), "the cover used up, the visit is billed: {}", follow[2]);
    assert_eq!(t.get("FeeValidity", uid(&validity["id"])).await["visits_left"], 0);
    // Another practitioner's visit is billed at their fee and item; the dental cover is Omar's.
    let dent = t.act(&omar_u, "Appointment", tooth, "complete").await.unwrap();
    let dinv = t.get("SalesInvoice", uid(&dent["sales_invoice"]["id"])).await;
    assert_eq!((dec(&dinv["grand_total"]), dinv["items"][0]["item"].as_str()), (1200.0, Some(dental.to_string().as_str())));
    let ben_customer = t.get("Patient", ben).await["customer"].clone();
    assert!(ben_customer.is_string(), "made at the first bill");
    let again = t.act(&omar_u, "Appointment", later, "complete").await.unwrap();
    assert!(again["covered_by"].is_string(), "a follow-up the same day");
    // The dental cover is Omar's: Ben's visit to Dr Shah within it is billed.
    let other_doctor = uid(&t.k.insert(&desk, "Appointment", book(ben, mira, general, "2026-11-11", "17:00")).await.unwrap()["id"]);
    t.act(&desk, "Appointment", other_doctor, "check_in").await.unwrap();
    assert!(t.act(&mira_u, "Appointment", other_doctor, "complete").await.unwrap()["sales_invoice"].is_object(), "another practitioner, another bill");
    // Outside the cover's days, a visit is billed again.
    let next_month = uid(&t.k.insert(&desk, "Appointment", book(ben, omar, general, "2026-12-07", "09:00")).await.unwrap()["id"]);
    t.act(&desk, "Appointment", next_month, "check_in").await.unwrap();
    assert!(t.act(&omar_u, "Appointment", next_month, "complete").await.unwrap()["sales_invoice"].is_object());
    assert_eq!(t.get("Patient", ben).await["customer"], ben_customer, "one customer a patient, however often billed");
    t.act(&desk, "Appointment", ben_visit, "no_show").await.unwrap();
    assert_eq!(t.get("Appointment", ben_visit).await["status"], "No Show");
    refused(t.act(&desk, "Appointment", ben_visit, "check_in").await, "invalid", "not available");

    // ───────────── who may do what ─────────────
    refused(t.k.insert(&desk, "Practitioner", obj(json!({"practitioner_name": "Self-made", "consultation_fee": "1"}))).await, "forbidden", "");
    refused(t.k.insert(&mira_u, "Patient", obj(json!({"patient_name": "Walk-in"}))).await, "forbidden", "");
    refused(t.k.insert(&mira_u, "Appointment", book(ben, mira, general, "2026-11-16", "09:00")).await, "forbidden", "");
    refused(t.k.insert(&desk, "ConsentForm", obj(json!({"form_name": "Mine", "text": "x"}))).await, "forbidden", "");
}
