//! Double booking under concurrent requests, against a real PostgreSQL
//! (`FERRUM_HEALTHCARE_TEST_DATABASE_URL`, default `postgres://ferrum:ferrum@localhost:5432/ferrum_healthcare_test`;
//! reset first): fifty patients asking online for one practitioner's 10:00 in the same instant get
//! one visit between them, and one patient booked at the desk with fifty practitioners at once gets
//! one. Each booking is its own request on its own connection. The first test lets them go together,
//! and shows that a writer which does not pass the appointment hook is refused by the database; the
//! second holds them until all fifty wait (the first at its write, the others for their turn behind
//! it) and expects every other one refused at once, by name.

use ferrum_kernel::tenant::NewTenant;
use ferrum_kernel::{Ctx, Db, DbConfig, Doc, Error, HookRegistry, Kernel, KernelSettings, ListQuery, SiteRequest};
use ferrum_meta::Catalog;
use serde_json::{Value, json};
use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};
use tokio::sync::Barrier;
use uuid::Uuid;

const N: usize = 50;

/// Both tests reset the same database: one at a time.
static ONE_AT_A_TIME: Mutex<()> = Mutex::new(());

fn obj(v: Value) -> Doc {
    v.as_object().cloned().unwrap()
}

fn uid(v: &Value) -> Uuid {
    v.as_str().unwrap_or_else(|| panic!("not an id: {v}")).parse().unwrap()
}

/// How many requests ended each way: `booked`, or the refusal's code and words (numbers masked).
fn tally<T>(outcomes: &[Result<T, Error>]) -> BTreeMap<String, usize> {
    let mut seen = BTreeMap::new();
    for o in outcomes {
        let said = match o {
            Ok(_) => "booked".to_string(),
            Err(e) => format!("{}: {}", e.code(), e.to_string().replace(|c: char| c.is_ascii_digit(), "#")),
        };
        *seen.entry(said).or_insert(0) += 1;
    }
    seen
}

/// Runs `n` calls of `f` at once, each on its own task and connection; their outcomes in order.
/// Without `hold` they are let go together. With it, writes to that table are held until all `n`
/// wait on a lock (there, or on a row another of them holds), then let go.
async fn at_once<T, F, Fut>(k: &Kernel, hold: Option<&str>, n: usize, f: F) -> Vec<Result<T, Error>>
where
    T: Send + 'static,
    F: Fn(usize) -> Fut,
    Fut: std::future::Future<Output = Result<T, Error>> + Send + 'static,
{
    let held = match hold {
        Some(table) => {
            let mut tx = k.db.pool.begin().await.expect("a transaction");
            sqlx::query(sqlx::AssertSqlSafe(format!("lock table {table} in share mode"))).execute(&mut *tx).await.expect("writes held");
            Some(tx)
        }
        None => None,
    };
    let gate = Arc::new(Barrier::new(if held.is_some() { 1 } else { n }));
    let tasks: Vec<_> = (0..n)
        .map(|i| {
            let (gate, call) = (gate.clone(), f(i));
            tokio::spawn(async move {
                gate.wait().await;
                call.await
            })
        })
        .collect();
    if let Some(held) = held {
        let since = std::time::Instant::now();
        loop {
            let waiting: i64 = sqlx::query_scalar(
                "select count(*) from pg_stat_activity where datname = current_database() and backend_type = 'client backend' and wait_event_type = 'Lock'",
            )
            .fetch_one(&k.db.pool)
            .await
            .expect("waiting requests");
            if waiting >= n as i64 {
                break;
            }
            assert!(since.elapsed() < std::time::Duration::from_secs(60), "{waiting} of {n} came to wait");
            tokio::time::sleep(std::time::Duration::from_millis(20)).await;
        }
        held.commit().await.expect("writes let go");
    }
    let mut out = Vec::with_capacity(n);
    for t in tasks {
        out.push(t.await.expect("a booking's task"));
    }
    out
}

/// A clinic that books online, a visit type of twenty minutes, practitioners who see patients every
/// day from nine to five, and its desk.
struct Clinic {
    k: Arc<Kernel>,
    tenant: Uuid,
    admin: Ctx,
    desk: Ctx,
    general: Uuid,
    /// A week from the workspace's today.
    day: String,
}

impl Clinic {
    async fn open() -> Clinic {
        let url =
            std::env::var("FERRUM_HEALTHCARE_TEST_DATABASE_URL").unwrap_or_else(|_| "postgres://ferrum:ferrum@localhost:5432/ferrum_healthcare_test".into());
        // A connection for each booking, and some to spare.
        let db = Db::connect(DbConfig { url, max_connections: (N + 10) as u32, ..Default::default() }).await.expect("connect");
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
        let desk_id = k.create_user(&admin, "desk@clinic.test", "Desk", Some("Correct-Horse-9"), &["Clinic Reception".to_string()], false).await.expect("user");
        let s = k.login("clinic", "desk@clinic.test", "Correct-Horse-9").await.expect("login");
        let desk = k.load_ctx(tenant, desk_id, Some(s.token_version), "it".into()).await.expect("ctx");
        let day = (admin.now_local().date_naive() + chrono::Duration::days(7)).to_string();
        let mut c = Clinic { k, tenant, admin, desk, general: Uuid::nil(), day };
        let one = |entity: &'static str, filters: Value| {
            let (k, a) = (c.k.clone(), c.admin.clone());
            async move {
                let q = ListQuery { filters: ferrum_kernel::query::parse_filters(&filters).unwrap(), ..Default::default() };
                k.list(&a, entity, &q).await.unwrap().rows.remove(0)
            }
        };
        let (inr, services, nos) = (
            one("Currency", json!({"code": "INR"})).await,
            one("ItemGroup", json!({"group_name": "Services"})).await,
            one("Uom", json!({"uom_name": "Nos"})).await,
        );
        let company = c.insert("Company", json!({"company_name": "Lakeside Clinic", "abbr": "LC", "default_currency": inr["id"]})).await;
        let consult = c
            .insert(
                "Item",
                json!({"item_code": "CONSULT", "item_name": "CONSULT", "item_group": services["id"], "stock_uom": nos["id"], "is_stock_item": false}),
            )
            .await;
        c.insert("HealthcareSettings", json!({"company": company.to_string(), "consultation_item": consult.to_string(), "book_online": true})).await;
        c.general = c.insert("AppointmentType", json!({"type_name": "General", "duration": 20, "book_online": true})).await;
        c
    }

    async fn insert(&self, entity: &str, v: Value) -> Uuid {
        uid(&self.k.insert(&self.admin, entity, obj(v)).await.unwrap_or_else(|e| panic!("insert {entity}: {e:?}"))["id"])
    }

    async fn practitioner(&self, name: &str) -> Uuid {
        let every_day: Vec<Value> = ["Monday", "Tuesday", "Wednesday", "Thursday", "Friday", "Saturday", "Sunday"]
            .iter()
            .map(|d| json!({"day": d, "from_time": "09:00", "to_time": "17:00"}))
            .collect();
        self.insert("Practitioner", json!({"practitioner_name": name, "consultation_fee": "800", "book_online": true, "hours": every_day})).await
    }

    /// The visits kept for a practitioner or a patient at `time` that day.
    async fn live(&self, who: Uuid, time: &str) -> i64 {
        let mut tx = self.k.db.tenant_tx(self.tenant).await.unwrap();
        let n: i64 = sqlx::query_scalar(
            "select count(*) from e_appointment where tenant_id = $1 and $2 in (practitioner, patient) and \"date\" = $3::date and \"time\" = $4 \
             and status in ('Scheduled', 'Checked In')",
        )
        .bind(self.tenant)
        .bind(who)
        .bind(&self.day)
        .bind(time)
        .fetch_one(&mut *tx)
        .await
        .unwrap();
        tx.commit().await.unwrap();
        n
    }

    /// Fifty patients book `who` at `time` on the clinic's page at once, each from an address of
    /// its own (the page's limit per address is not what is tested).
    async fn online(&self, hold: Option<&str>, who: Uuid, time: &'static str) -> BTreeMap<String, usize> {
        let site = self.k.hooks.site("healthcare").expect("healthcare's endpoints").clone();
        let outcomes = at_once(&self.k, hold, N, |i| {
            let (k, site) = (self.k.clone(), site.clone());
            let ask = json!({"practitioner": who.to_string(), "type": self.general.to_string(), "date": self.day, "time": time,
                             "name": format!("Patient {i}"), "email": format!("patient{i}@home.test")});
            let req = SiteRequest {
                tenant: self.tenant,
                user: None,
                method: "POST".into(),
                path: "/book".into(),
                query: Default::default(),
                headers: Default::default(),
                body: ask.to_string().into_bytes(),
                client_ip: format!("198.51.100.{}", i + 1),
            };
            async move { site.handle(&k, req).await.map(|r| r.status) }
        })
        .await;
        tally(&outcomes)
    }

    /// A new patient booked at the desk with fifty new practitioners at `time` at once.
    async fn at_desk(&self, hold: Option<&str>, time: &'static str) -> (Uuid, BTreeMap<String, usize>) {
        let ravi = self.insert("Patient", json!({"patient_name": "Ravi Iyer"})).await;
        let mut doctors = Vec::with_capacity(N);
        for i in 0..N {
            doctors.push(self.practitioner(&format!("Dr {i}")).await);
        }
        let outcomes = at_once(&self.k, hold, N, |i| {
            let (k, desk) = (self.k.clone(), self.desk.clone());
            let visit = json!({"patient": ravi.to_string(), "practitioner": doctors[i].to_string(), "appointment_type": self.general.to_string(),
                               "date": self.day, "time": time});
            async move { k.insert(&desk, "Appointment", obj(visit)).await }
        })
        .await;
        (ravi, tally(&outcomes))
    }
}

fn run(test: impl std::future::Future<Output = ()> + Send + 'static) {
    let _one = ONE_AT_A_TIME.lock().unwrap_or_else(|e| e.into_inner());
    let rt = tokio::runtime::Builder::new_multi_thread().enable_all().thread_stack_size(16 << 20).build().unwrap();
    rt.block_on(async { tokio::spawn(test).await.unwrap() });
}

#[test]
fn fifty_bookings_of_one_slot_yield_one() {
    run(async {
        let c = Clinic::open().await;
        let mira = c.practitioner("Dr Mira Shah").await;
        let online = c.online(None, mira, "10:00").await;
        let (ravi, at_desk) = c.at_desk(None, "14:00").await;
        assert_eq!((online.get("booked"), at_desk.get("booked")), (Some(&1), Some(&1)), "one visit each: online {online:#?}, at the desk {at_desk:#?}");
        assert_eq!((c.live(mira, "10:00").await, c.live(ravi, "14:00").await), (1, 1), "one visit kept each");
        // Everyone else is told the time is taken: in the page's words, the practitioner's or the patient's.
        let taken = [
            "conflict: The practitioner is booked then",
            "validation_failed: validation failed: time: is not free: choose one of the times offered",
            "validation_failed: validation failed: time: Dr Mira Shah is booked then, under APT-####-#####",
        ];
        assert!(online.keys().all(|s| s == "booked" || taken.contains(&s.as_str())), "{online:#?}");
        let taken = ["conflict: The patient is booked then", "validation_failed: validation failed: time: The patient is booked then, under APT-####-#####"];
        assert!(at_desk.keys().all(|s| s == "booked" || taken.contains(&s.as_str())), "{at_desk:#?}");

        // A writer that does not pass the appointment hook is refused by the database itself: the
        // visit kept online, copied under a new id.
        let mut tx = c.k.db.tenant_tx(c.tenant).await.unwrap();
        let copied = sqlx::query(
            "insert into e_appointment select r.* from e_appointment a, \
             lateral jsonb_populate_record(null::e_appointment, to_jsonb(a) || jsonb_build_object('id', gen_random_uuid(), 'number', null)) r \
             where a.tenant_id = $1 and a.practitioner = $2 and a.status = 'Scheduled'",
        )
        .bind(c.tenant)
        .bind(mira)
        .execute(&mut *tx)
        .await;
        assert!(matches!(&copied, Err(sqlx::Error::Database(e)) if e.code().as_deref() == Some("23P01")), "a copy of the kept visit: {copied:?}");
    });
}

#[test]
fn fifty_bookings_held_at_the_write_yield_one_at_once() {
    run(async {
        let c = Clinic::open().await;
        let mira = c.practitioner("Dr Mira Shah").await;
        let online = c.online(Some("e_appointment"), mira, "10:00").await;
        let (ravi, at_desk) = c.at_desk(Some("e_appointment"), "14:00").await;
        // One kept each, and every other one refused by the hook, naming the visit kept: none left
        // to deadlock detection or the statement timeout, none given the database's bare refusal.
        let one_and = |refusal: &str| BTreeMap::from([("booked".to_string(), 1), (format!("validation_failed: validation failed: time: {refusal}"), N - 1)]);
        assert_eq!(online, one_and("Dr Mira Shah is booked then, under APT-####-#####"), "online");
        assert_eq!(at_desk, one_and("The patient is booked then, under APT-####-#####"), "at the desk");
        assert_eq!((c.live(mira, "10:00").await, c.live(ravi, "14:00").await), (1, 1), "one visit kept each");
    });
}
