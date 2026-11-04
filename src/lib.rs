//! Healthcare administration (FERRUM-INDUSTRY-005): patients, practitioners' hours, appointments
//! without clashes, consent by form version, and visits billed through ERP with follow-ups.
//!
//! * `appointments`: a visit falls within its practitioner's hours that day and clashes with
//!   neither the practitioner's nor the patient's other visits; a patient is checked in only with
//!   the consent its type needs; completing a visit bills it, unless a paid visit's follow-ups
//!   cover it.
//! * `consent`: changing a form's text makes a new version; a consent is to one version, and may be
//!   withdrawn.
//!
//! Adapted from ERPNext 12's Healthcare module (GPL-3.0; docs/PROVENANCE.md). Hooks are registered
//! per entity, so a workspace without the `healthcare` package never runs them. No health-data law
//! is claimed to be met (BLOCK-LEGAL-REVIEW).

mod appointments;
mod consent;

use ferrum_domain::reports::ReportSource;
use ferrum_kernel::{Doc, HookRegistry, Kernel, Result};
use serde_json::Value;
use std::sync::Arc;

include!(concat!(env!("OUT_DIR"), "/packages.rs"));
include!(concat!(env!("OUT_DIR"), "/ui.rs"));

/// This app's manifest (`app.yaml`).
pub const MANIFEST: &str = include_str!("../app.yaml");

/// This app's metadata packages, embedded at build time.
pub fn packages() -> std::result::Result<Vec<ferrum_meta::Package>, ferrum_meta::MetaError> {
    ferrum_meta::catalog::load_embedded(EMBEDDED)
}

/// Registers this app's document hooks.
pub fn register(hooks: &mut HookRegistry) {
    hooks
        .register("Appointment", Arc::new(appointments::AppointmentHook))
        .register("Practitioner", Arc::new(appointments::PractitionerHook))
        .register("ConsentForm", Arc::new(consent::FormHook))
        .register("PatientConsent", Arc::new(consent::ConsentHook));
}

/// This app's reports: none of its own yet; what is billed is ERP's receivables.
pub fn reports() -> Vec<Arc<dyn ReportSource>> {
    vec![]
}

/// Upkeep: nothing runs by itself.
pub async fn maintenance(_k: &Kernel) -> Result<()> {
    Ok(())
}

/// A document from JSON.
pub(crate) fn obj(v: Value) -> Doc {
    v.as_object().cloned().unwrap_or_default()
}

/// Minutes past midnight of an `HH:MM` time.
pub(crate) fn minutes(hhmm: &str) -> Option<i64> {
    let (h, m) = hhmm.trim().split_once(':')?;
    let (h, m): (i64, i64) = (h.parse().ok()?, m.get(..2).unwrap_or(m).parse().ok()?);
    ((0..24).contains(&h) && (0..60).contains(&m)).then_some(h * 60 + m)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn times_of_day() {
        assert_eq!(minutes("09:30"), Some(570));
        assert_eq!(minutes("9:05"), Some(545));
        assert_eq!(minutes("17:00:00"), Some(1020));
        assert_eq!(minutes("24:00"), None);
        assert_eq!(minutes("noon"), None);
    }

    #[test]
    fn the_interface_is_a_release_the_platform_takes() {
        let m = ferrum_apps::AppManifest::parse(MANIFEST).expect("app.yaml");
        assert_eq!(m.routes(), vec![("healthcare".to_string(), vec![])]);
        let files = UI.iter().map(|(p, b)| (p.to_string(), b.to_vec())).collect();
        let ui = ferrum_apps::ui::prepare("healthcare", files).expect("ui/dist is an interface release");
        assert!(UI.iter().any(|(p, _)| *p == "index.html") && !UI.iter().any(|(p, _)| p.starts_with('.')), "{}", ui.digest);
    }
}
