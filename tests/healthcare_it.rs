//! A week at a clinic (FERRUM-INDUSTRY-005) against a real PostgreSQL
//! (`FERRUM_HEALTHCARE_TEST_DATABASE_URL`, default `postgres://ferrum:ferrum@localhost:5432/ferrum_healthcare_test`;
//! reset first): practitioners' hours, appointments refused outside them or when either the
//! practitioner or the patient is booked then; consent to a form's current version needed to check
//! in, withdrawn, and asked again when the form changes; completing a visit only by its
//! practitioner, billing it through ERP to a customer made from the patient, and covering the
//! follow-ups within the settings' days; and who may do what.

use ferrum_kernel::privacy::Person;
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
    // A visit under way keeps its patient, practitioner and time: the form shows them read-only, and a change sent anyway is ignored.
    let kept = t.k.update(&desk, "Appointment", first, obj(json!({"time": "11:30", "date": "2030-01-01"})), None).await.expect("nothing to refuse");
    assert_eq!((kept["date"].clone(), kept["time"].clone(), kept["status"].clone()), (json!("2026-11-09"), json!("09:00"), json!("Checked In")), "{kept:?}");
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
    // Missed only once it was due: a visit still to come is refused, one gone by is marked.
    let ahead = uid(&t.k.insert(&desk, "Appointment", book(ben, mira, general, "2030-11-04", "09:20")).await.unwrap()["id"]);
    refused(t.act(&desk, "Appointment", ahead, "no_show").await, "invalid", "has not begun yet: it is at 09:20 on 2030-11-04");
    let missed = uid(&t.k.insert(&desk, "Appointment", book(ben, mira, general, "2026-11-02", "09:20")).await.unwrap()["id"]);
    t.act(&desk, "Appointment", missed, "no_show").await.unwrap();
    assert_eq!(t.get("Appointment", missed).await["status"], "No Show");
    refused(t.act(&desk, "Appointment", missed, "check_in").await, "invalid", "not available");
    assert_eq!(t.get("Appointment", ben_visit).await["status"], "Scheduled");

    // ───────────── who may do what ─────────────
    refused(t.k.insert(&desk, "Practitioner", obj(json!({"practitioner_name": "Self-made", "consultation_fee": "1"}))).await, "forbidden", "");
    refused(t.k.insert(&mira_u, "Patient", obj(json!({"patient_name": "Walk-in"}))).await, "forbidden", "");
    refused(t.k.insert(&mira_u, "Appointment", book(ben, mira, general, "2026-11-16", "09:00")).await, "forbidden", "");
    refused(t.k.insert(&desk, "ConsentForm", obj(json!({"form_name": "Mine", "text": "x"}))).await, "forbidden", "");

    // ───────────── a patient's request ─────────────
    // Asha is found by her address and her number however written, her appointments through her;
    // all of it is kept, as medical records, and the answer says why. The customer her bills went
    // to is erased as any person's is, and the bills keep the name they carry.
    let asha_asks = Person { email: "asha@home.test".into(), phone: Some("+91 9000000001".into()), name: None };
    let asked = t.k.personal_data_of(&t.admin, &asha_asks).await.unwrap();
    let records = asked["records"].as_array().unwrap();
    let of = |entity: &str| records.iter().filter(|r| r["entity"] == json!(entity)).count();
    assert_eq!(of("Patient"), 1, "{asked}");
    assert!(of("Appointment") >= 2, "her appointments, through her: {asked}");
    assert!(records.iter().filter(|r| r["entity"] != "Customer").all(|r| r["retained_because"].is_string()), "{asked}");
    let erased = t.k.erase_personal_data_of(&t.admin, &asha_asks, "Request 7").await.unwrap();
    let gone: Vec<&str> = erased["erased"].as_array().unwrap().iter().map(|r| r["entity"].as_str().unwrap()).collect();
    assert_eq!(gone, ["Customer"], "{erased}");
    assert_eq!(erased["retained"].as_array().unwrap().len(), records.len() - 1, "{erased}");
    assert_eq!(t.get("Patient", asha).await["patient_name"], "Asha Rao", "kept as it was");

    // ───────────── booking online, reminders, the patients' portal ─────────────
    let tenant_id = t.tenant;
    let call = |who: Option<Ctx>, method: &'static str, path: String, body: Value| {
        let k = k.clone();
        async move {
            let site = k.hooks.site("healthcare").expect("healthcare's endpoints").clone();
            let (path, query) = path.split_once('?').map(|(p, q)| (p.to_string(), q.to_string())).unwrap_or((path, String::new()));
            let query: serde_json::Map<String, Value> = query.split('&').filter_map(|p| p.split_once('=')).map(|(k, v)| (k.to_string(), json!(v))).collect();
            let req = ferrum_kernel::SiteRequest {
                tenant: tenant_id,
                user: who,
                method: method.into(),
                path: format!("/{path}"),
                query,
                headers: Default::default(),
                body: if body.is_null() { vec![] } else { body.to_string().into_bytes() },
                client_ip: "198.51.100.4".into(),
            };
            let r = site.handle(&k, req).await?;
            Ok::<_, Error>((r.status, serde_json::from_slice::<Value>(&r.body).unwrap()))
        }
    };
    refused(call(None, "GET", "book/options".into(), Value::Null).await, "not found", "");
    let settings = uid(&t.find_one("HealthcareSettings", json!({})).await["id"]);
    t.k.update(&manager, "HealthcareSettings", settings, obj(json!({"book_online": true, "cancel_hours": 2, "reminder_hours": 24})), None).await.unwrap();
    t.k.update(&manager, "Practitioner", mira, obj(json!({"book_online": true})), None).await.unwrap();
    t.k.update(&manager, "AppointmentType", general, obj(json!({"book_online": true})), None).await.unwrap();
    let options = call(None, "GET", "book/options".into(), Value::Null).await.unwrap().1;
    assert_eq!(
        (options["practitioners"].as_array().unwrap().iter().map(|p| p["name"].as_str().unwrap()).collect::<Vec<_>>(), options["types"][0]["name"].as_str()),
        (vec!["Dr Mira Shah"], Some("General")),
        "only who and what the clinic books online"
    );
    // The next Monday at least two days ahead, on the workspace's clock.
    use chrono::Datelike;
    let today = t.admin.now_local().date_naive();
    let mut monday = today + chrono::Duration::days(2);
    while monday.weekday() != chrono::Weekday::Mon {
        monday += chrono::Duration::days(1);
    }
    let day = monday.to_string();
    let slots = |practitioner: Uuid, kind: Uuid, on: &str| format!("book/slots?practitioner={practitioner}&type={kind}&date={on}");
    let free = call(None, "GET", slots(mira, general, &day), Value::Null).await.unwrap().1;
    let times: Vec<String> = free["times"].as_array().unwrap().iter().map(|v| v.as_str().unwrap().to_string()).collect();
    assert!(times.contains(&"09:00".to_string()) && times.contains(&"12:40".to_string()) && !times.contains(&"12:50".to_string()), "{times:?}");
    t.k.insert(&desk, "Appointment", book(ben, mira, general, &day, "09:20")).await.expect("Ben at the desk");
    let free = call(None, "GET", slots(mira, general, &day), Value::Null).await.unwrap().1;
    assert!(!free["times"].as_array().unwrap().iter().any(|v| v == "09:20"), "a booked time is not offered");
    refused(call(None, "GET", slots(omar, general, &day), Value::Null).await, "not found", "");
    refused(call(None, "GET", slots(mira, dentistry, &day), Value::Null).await, "not found", "");
    refused(call(None, "GET", slots(mira, general, &(today - chrono::Duration::days(1)).to_string()), Value::Null).await, "invalid", "is from");
    let ask = |time: &str| json!({"practitioner": mira.to_string(), "type": general.to_string(), "date": day, "time": time, "name": "Cleo Park", "email": "Cleo@Home.test"});
    refused(call(None, "POST", "book".into(), ask("09:20")).await, "invalid", "is not free");
    assert_eq!(call(None, "POST", "book".into(), json!({"website": "x"})).await.unwrap(), (201, json!({"status": "received"})));
    let (code, booked) = call(None, "POST", "book".into(), ask("09:00")).await.expect("booked online");
    assert_eq!(
        (code, booked["visit"]["status"].as_str(), booked["visit"]["practitioner"].as_str(), booked["visit"]["cancellable"].as_bool()),
        (201, Some("Scheduled"), Some("Dr Mira Shah"), Some(true))
    );
    assert!(booked["visit"].get("notes").is_none() && booked["visit"].get("patient").is_none(), "nothing clinical or personal: {booked}");
    let token = booked["token"].as_str().unwrap().to_string();
    let cleo = t.find_one("Patient", json!({"email": "cleo@home.test"})).await;
    assert_eq!(cleo["source"], "Online");
    let cleo = uid(&cleo["id"]);
    let (_, second) = call(None, "POST", "book".into(), ask("09:40")).await.expect("a second visit");
    assert_eq!(
        t.k.list(
            &a,
            "Patient",
            &ListQuery { filters: ferrum_kernel::query::parse_filters(&json!({"email": "cleo@home.test"})).unwrap(), ..Default::default() }
        )
        .await
        .unwrap()
        .rows
        .len(),
        1,
        "the patient known by the email"
    );
    let first_online = t.find_one("Appointment", json!({"patient": cleo.to_string(), "time": "09:00"})).await;
    assert!(first_online["source"] == "Online" && first_online["token_hash"].as_str().is_some_and(|h| h.len() == 64 && h != token));
    let mine = call(None, "GET", format!("manage/{token}"), Value::Null).await.unwrap().1;
    assert_eq!(mine["visit"]["time"], "09:00");
    let undone = call(None, "POST", format!("manage/{token}/cancel"), Value::Null).await.expect("cancelled by its link").1;
    assert_eq!(undone["visit"]["status"], "Cancelled");
    refused(call(None, "POST", format!("manage/{token}/cancel"), Value::Null).await, "invalid", "is not cancelled online");
    refused(call(None, "GET", format!("manage/{}", "A".repeat(43)), Value::Null).await, "not found", "");
    // A reminder the day before, once; a visit later in the week waits for its own day.
    let wednesday = (monday + chrono::Duration::days(2)).to_string();
    let later_visit = uid(&t.k.insert(&desk, "Appointment", book(cleo, mira, general, &wednesday, "14:00")).await.expect("Wednesday")["id"]);
    let eve = (monday - chrono::Duration::days(1)).and_hms_opt(10, 0, 0).unwrap();
    ferrum_app_healthcare::maintenance_at(&k, eve).await.expect("upkeep");
    ferrum_app_healthcare::maintenance_at(&k, eve).await.expect("upkeep again");
    let mails = |to: &'static str| {
        let k = k.clone();
        async move {
            let mut tx = k.db.tenant_tx(tenant_id).await.unwrap();
            let n: i64 = sqlx::query_scalar("select count(*) from outbox where tenant_id = $1 and topic = 'ferrum.mail' and payload->>'to' = $2")
                .bind(tenant_id)
                .bind(to)
                .fetch_one(&mut *tx)
                .await
                .unwrap();
            n
        }
    };
    assert_eq!(mails("cleo@home.test").await, 1, "the 09:40 visit reminded once; the cancelled one not");
    let reminded = uid(&second["visit"]["id"]);
    assert!(t.get("Appointment", reminded).await["reminded_at"].is_string());
    assert!(t.get("Appointment", later_visit).await["reminded_at"].is_null(), "not yet");
    // The patient's portal: their visits, the consent a coming visit needs, given there, and cancelling.
    let portal = |email: &'static str| {
        let k = k.clone();
        let a = a.clone();
        async move {
            let id = k.create_user(&a, email, "Patient", None, &[], false).await.unwrap();
            k.set_user_kind(&a, id, "portal").await.unwrap();
            k.load_ctx(a.tenant, id, None, "it".into()).await.unwrap()
        }
    };
    let dental = uid(&t.k.insert(&desk, "Appointment", book(cleo, omar, dentistry, &day, "14:00")).await.expect("a dental visit at the desk")["id"]);
    let cleo_p = portal("cleo@home.test").await;
    let cleo_user = cleo_p.user;
    let seen = call(Some(cleo_p.clone()), "GET", "portal/visits".into(), Value::Null).await.unwrap().1;
    let me = &seen["patients"][0];
    assert_eq!((seen["patients"].as_array().unwrap().len(), me["visits"].as_array().unwrap().len()), (1, 4), "{seen}");
    assert_eq!((me["consents_needed"][0]["name"].as_str(), me["consents_needed"][0]["version"].as_i64()), (Some("Dental treatment and X-rays"), Some(2)));
    let gave =
        call(Some(cleo_p.clone()), "POST", "portal/consent".into(), json!({"patient": cleo.to_string(), "form": form_id.to_string()})).await.expect("given").1;
    assert_eq!(gave["version"], 2);
    let seen = call(Some(cleo_p.clone()), "GET", "portal/visits".into(), Value::Null).await.unwrap().1;
    assert!(seen["patients"][0]["consents_needed"].as_array().unwrap().is_empty());
    let bens = t.find_one("Appointment", json!({"patient": ben.to_string(), "date": day, "time": "09:20"})).await;
    refused(call(Some(cleo_p.clone()), "POST", "portal/cancel".into(), json!({"visit": bens["id"]})).await, "not found", "");
    refused(
        call(Some(cleo_p.clone()), "POST", "portal/consent".into(), json!({"patient": ben.to_string(), "form": form_id.to_string()})).await,
        "not found",
        "",
    );
    // Too close to cancel online: the clinic is called instead.
    t.k.update(&manager, "HealthcareSettings", settings, obj(json!({"cancel_hours": 10000})), None).await.unwrap();
    refused(call(Some(cleo_p.clone()), "POST", "portal/cancel".into(), json!({"visit": reminded.to_string()})).await, "invalid", "is not cancelled online");
    t.k.update(&manager, "HealthcareSettings", settings, obj(json!({"cancel_hours": 2})), None).await.unwrap();
    let off = call(Some(cleo_p), "POST", "portal/cancel".into(), json!({"visit": reminded.to_string()})).await.expect("cancelled on the portal").1;
    assert_eq!(off["visit"]["status"], "Cancelled");
    let stranger = portal("nobody@else.test").await;
    assert!(call(Some(stranger), "GET", "portal/visits".into(), Value::Null).await.unwrap().1["patients"].as_array().unwrap().is_empty());
    assert!(matches!(call(None, "GET", "portal/visits".into(), Value::Null).await, Err(Error::Unauthorized(_))));

    // ───────────── the same portal as configuration (Ferrum BF-PORTAL-006) ─────────────
    // The package's Patient Portal role, given to her: no app code between her and her records.
    t.k.set_user_roles_until(&a, cleo_user, &["Patient Portal".to_string()], &Default::default()).await.expect("a portal role for a portal person");
    let cleo_g = t.k.load_ctx(t.tenant, cleo_user, None, "it".into()).await.unwrap();
    let ids = |rows: &[Value]| rows.iter().map(|r| r["id"].as_str().unwrap().to_string()).collect::<std::collections::BTreeSet<_>>();
    let doc_ids = |rows: &[Doc]| rows.iter().map(|r| r["id"].as_str().unwrap().to_string()).collect::<std::collections::BTreeSet<_>>();
    let page = call(Some(cleo_g.clone()), "GET", "portal/visits".into(), Value::Null).await.unwrap().1;
    let on_page = ids(page["patients"][0]["visits"].as_array().unwrap());
    let generic = t.k.list(&cleo_g, "Appointment", &ListQuery { limit: Some(500), ..Default::default() }).await.unwrap().rows;
    assert_eq!(doc_ids(&generic), on_page, "the configured portal reaches exactly the visits the app's page shows");
    assert!(t.k.get(&cleo_g, "Appointment", uid(&bens["id"]), false).await.is_err(), "another patient's visit");
    let mine = t.k.list(&cleo_g, "Patient", &ListQuery::default()).await.unwrap().rows;
    assert_eq!(doc_ids(&mine), [cleo.to_string()].into_iter().collect(), "her own record, no one else's");
    for hidden in ["date_of_birth", "policy_number", "allergies"] {
        assert!(mine[0].get(hidden).is_none_or(Value::is_null), "{hidden}, a personal or clinical field, reached the portal");
    }
    // Cancelling, as the app's page does: within the clinic's window, and only her own.
    t.k.update(&manager, "HealthcareSettings", settings, obj(json!({"cancel_hours": 10000})), None).await.unwrap();
    refused(t.act(&cleo_g, "Appointment", dental, "cancel_appointment").await, "invalid", "is not cancelled online");
    t.k.update(&manager, "HealthcareSettings", settings, obj(json!({"cancel_hours": 2})), None).await.unwrap();
    refused(t.act(&cleo_g, "Appointment", uid(&bens["id"]), "cancel_appointment").await, "not found", "");
    assert!(t.k.update(&cleo_g, "Appointment", dental, obj(json!({"time": "16:00"})), None).await.is_err(), "the role cancels; it does not move visits");
    t.act(&cleo_g, "Appointment", dental, "cancel_appointment").await.expect("cancelled through the portal role");
    assert_eq!(t.get("Appointment", dental).await["status"], "Cancelled");
    // Consenting: for her own record, given by whoever signed in, whatever was typed.
    let given = t
        .k
        .insert(&cleo_g, "PatientConsent", obj(json!({"patient": cleo.to_string(), "consent_form": form_id.to_string(), "given_by": "the clinic's director"})))
        .await
        .expect("consent given");
    assert_eq!(given["given_by"], "Patient, on the portal as cleo@home.test");
    let for_ben = t.k.insert(&cleo_g, "PatientConsent", obj(json!({"patient": ben.to_string(), "consent_form": form_id.to_string(), "given_by": "Ben"}))).await;
    assert!(matches!(for_ben, Err(Error::Forbidden(_))), "a consent for someone else: {for_ben:?}");
    // The clinic's people cancel whenever they must: the window is the portal's alone.
    t.k.update(&manager, "HealthcareSettings", settings, obj(json!({"cancel_hours": 10000})), None).await.unwrap();
    let late = uid(&t.k.insert(&desk, "Appointment", book(cleo, omar, dentistry, &day, "15:00")).await.expect("another")["id"]);
    t.act(&desk, "Appointment", late, "cancel_appointment").await.expect("the desk cancels within the window");
    t.k.update(&manager, "HealthcareSettings", settings, obj(json!({"cancel_hours": 2})), None).await.unwrap();

    // ───────────── an insurer's share ─────────────
    let star = uid(&t.insert(&a, "Customer", json!({"customer_name": "Star Health", "customer_type": "Company"})).await["id"]);
    let insured = |name: &'static str| {
        let (t, desk) = (&t, desk.clone());
        async move {
            uid(&t
                .insert(&desk, "Patient", json!({"patient_name": name, "insurer": star.to_string(), "policy_number": "SH-1", "coverage_percent": "80"}))
                .await["id"])
        }
    };
    // Each its own patient: a follow-up the same day would be covered, and not billed.
    let (ivy, ian, isa) = (insured("Ivy Insured").await, insured("Ian Insured").await, insured("Isa Insured").await);
    let visit_of = |patient: Uuid, time: &'static str| {
        let (t, desk, omar_u, day) = (&t, desk.clone(), omar_u.clone(), day.clone());
        async move {
            let v = uid(&t.k.insert(&desk, "Appointment", book(patient, omar, general, &day, time)).await.unwrap()["id"]);
            t.act(&desk, "Appointment", v, "check_in").await.unwrap();
            t.act(&omar_u, "Appointment", v, "complete").await.unwrap()
        }
    };
    let done = visit_of(ivy, "10:00").await;
    let mine = t.get("SalesInvoice", uid(&done["sales_invoice"]["id"])).await;
    let claim = t.get("InsuranceClaim", uid(&done["claim"]["id"])).await;
    let theirs = t.get("SalesInvoice", uid(&claim["sales_invoice"])).await;
    assert_eq!(
        (dec(&mine["grand_total"]), dec(&theirs["grand_total"]), theirs["customer"].as_str(), claim["status"].as_str(), dec(&claim["amount"])),
        (240.0, 960.0, Some(star.to_string().as_str()), Some("Submitted"), 960.0),
        "1200: 80% the insurer's, the rest Ivy's"
    );
    // Refused while its invoice is a draft: the share is Ivy's, the insurer's draft goes.
    let theirs_id = uid(&theirs["id"]);
    refused(t.k.run_action(&desk, "InsuranceClaim", uid(&claim["id"]), "reject", json!({})).await, "invalid", "");
    let rejected = t.k.run_action(&desk, "InsuranceClaim", uid(&claim["id"]), "reject", json!({"reason": "not covered for this"})).await.expect("rejected");
    assert_eq!(rejected["message"], "Rejected: the patient is billed 960.00 instead (a draft)");
    refused(t.k.get(&a, "SalesInvoice", theirs_id, false).await, "not found", "");
    let claim_now = t.get("InsuranceClaim", uid(&claim["id"])).await;
    let billed_instead = t.get("SalesInvoice", uid(&claim_now["patient_invoice"])).await;
    assert_eq!(
        (claim_now["status"].as_str(), dec(&billed_instead["grand_total"]), billed_instead["customer"].clone()),
        (Some("Rejected"), 960.0, mine["customer"].clone())
    );
    // Refused once issued: the insurer is credited it.
    let issued = visit_of(ian, "10:20").await;
    let claim2 = t.get("InsuranceClaim", uid(&issued["claim"]["id"])).await;
    t.k.submit(&a, "SalesInvoice", uid(&claim2["sales_invoice"]), None).await.expect("sent to the insurer");
    t.k.run_action(&desk, "InsuranceClaim", uid(&claim2["id"]), "reject", json!({"reason": "policy lapsed"})).await.expect("rejected");
    let note = t.get("SalesInvoice", uid(&t.get("InsuranceClaim", uid(&claim2["id"])).await["credit_note"])).await;
    assert_eq!((note["is_return"].as_bool(), note["return_against"].clone()), (Some(true), claim2["sales_invoice"].clone()));
    // Paid by the insurer: upkeep marks it paid.
    let paid = visit_of(isa, "10:40").await;
    let claim3 = uid(&paid["claim"]["id"]);
    let theirs3 = uid(&t.get("InsuranceClaim", claim3).await["sales_invoice"]);
    t.k.submit(&a, "SalesInvoice", theirs3, None).await.unwrap();
    ferrum_app_healthcare::maintenance_at(&k, eve).await.unwrap();
    assert_eq!(t.get("InsuranceClaim", claim3).await["status"], "Submitted", "sent, not paid yet");
    let pay = t.k.run_action(&a, "SalesInvoice", theirs3, "make_payment", Value::Null).await.expect("a payment");
    t.k.submit(&a, "PaymentEntry", uid(&pay["id"]), None).await.expect("paid");
    ferrum_app_healthcare::maintenance_at(&k, eve).await.unwrap();
    assert_eq!(t.get("InsuranceClaim", claim3).await["status"], "Paid");
    refused(t.k.run_action(&desk, "InsuranceClaim", claim3, "reject", json!({"reason": "late"})).await, "invalid", "not available");
}
