//! `GET /api/vpi/suggestions`: rent adjustments proposed for active index leases. The
//! arithmetic is `vermieterme_domain::vpi::index_adjustment`; this module only loads the
//! data and shapes the answer (contract section 9).

use axum::extract::State;
use serde::Serialize;
use time::OffsetDateTime;
use vermieterme_db::repo::masterdata::suggestions::{IndexTenant, index_tenants};
use vermieterme_db::repo::masterdata::vpi::{self, VpiRecord};
use vermieterme_domain::vpi::{VpiEntry, VpiValue, index_adjustment, latest_for_base_year};
use vermieterme_domain::{CalendarDate, Cents};

use crate::auth::AccountCtx;
use crate::error::ApiResult;
use crate::routes::AppState;

/// One proposal.
#[derive(Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SuggestionJson {
    tenant_id: String,
    unit_id: String,
    tenant_name: String,
    unit_name: String,
    property_address: String,
    lease_type: &'static str,
    current_rent_cents: i64,
    reference_vpi_tenths: i64,
    current_vpi_tenths: i64,
    percentage_change_basis_points: i64,
    suggested_rent_cents: i64,
    months_since_last_adjustment: i64,
    min_months: i64,
    eligible: bool,
    base_year: i64,
}

/// Proposals for the index tenants of the account.
pub async fn list(
    State(state): State<AppState>,
    ctx: AccountCtx,
) -> ApiResult<axum::Json<Vec<SuggestionJson>>> {
    let mut conn = state.conn().await?;
    let tenants = index_tenants(&mut conn, &ctx.scope).await?;
    let values = vpi::list(&mut conn).await?;
    let today = CalendarDate::from_date(OffsetDateTime::now_utc().date()).ok();
    let items = today
        .map(|today| build(&tenants, &values, today))
        .unwrap_or_default();
    Ok(axum::Json(items))
}

/// Builds the proposals: tenants without a rent, without a stored index value of their
/// base year or with unreadable dates get none (as before).
pub fn build(
    tenants: &[IndexTenant],
    values: &[VpiRecord],
    today: CalendarDate,
) -> Vec<SuggestionJson> {
    let entries: Vec<VpiEntry> = values.iter().filter_map(entry_of).collect();
    tenants
        .iter()
        .filter_map(|t| suggestion(t, &entries, today))
        .collect()
}

/// The domain form of a stored value; rows with impossible numbers are skipped.
fn entry_of(record: &VpiRecord) -> Option<VpiEntry> {
    Some(VpiEntry {
        year: i32::try_from(record.entry.year).ok()?,
        month: u8::try_from(record.entry.month).ok()?,
        value: VpiValue::from_tenths(record.entry.value_tenths),
        base_year: i32::try_from(record.entry.base_year).ok()?,
    })
}

/// The proposal for one tenant.
fn suggestion(
    t: &IndexTenant,
    entries: &[VpiEntry],
    today: CalendarDate,
) -> Option<SuggestionJson> {
    let current = latest_for_base_year(entries, i32::try_from(t.base_year).ok()?)?;
    let rent = Cents::new(t.current_rent_cents?);
    let reference_date =
        CalendarDate::parse(t.reference_date.as_deref().unwrap_or(&t.move_in_date)).ok()?;
    let reference = VpiValue::from_tenths(t.reference_tenths);
    let adjustment = index_adjustment(
        rent,
        reference,
        current.value,
        reference_date,
        today,
        t.min_months,
    )?;
    Some(SuggestionJson {
        tenant_id: t.tenant_id.clone(),
        unit_id: t.unit_id.clone(),
        tenant_name: format!("{} {}", t.first_name, t.last_name),
        unit_name: t.unit_name.clone(),
        property_address: format!("{}, {}", t.street, t.city),
        lease_type: "index",
        current_rent_cents: rent.value(),
        reference_vpi_tenths: reference.tenths(),
        current_vpi_tenths: current.value.tenths(),
        percentage_change_basis_points: adjustment.percentage_change_basis_points,
        suggested_rent_cents: adjustment.suggested_rent.value(),
        months_since_last_adjustment: adjustment.months_since_last_adjustment,
        min_months: t.min_months,
        eligible: adjustment.eligible,
        base_year: t.base_year,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use vermieterme_db::repo::masterdata::vpi::VpiRow;

    /// A tenant with rent 650.00 EUR, reference 100.0 from 2024-01-01.
    fn tenant() -> IndexTenant {
        IndexTenant {
            tenant_id: "t".into(),
            unit_id: "u".into(),
            first_name: "Erika".into(),
            last_name: "Muster".into(),
            unit_name: "Wohnung 1".into(),
            street: "Musterweg 1".into(),
            city: "Beispielstadt".into(),
            base_year: 2020,
            reference_tenths: 1000,
            reference_date: Some("2024-01-01".into()),
            move_in_date: "2020-01-01".into(),
            min_months: 12,
            current_rent_cents: Some(65_000),
        }
    }

    /// A stored value.
    fn value(year: i64, month: i64, tenths: i64, base_year: i64) -> VpiRecord {
        VpiRecord {
            id: format!("{year}-{month}"),
            entry: VpiRow {
                year,
                month,
                value_tenths: tenths,
                base_year,
            },
        }
    }

    #[test]
    fn proposes_with_the_latest_value_of_the_base_year() {
        let values = [
            value(2025, 1, 1030, 2020),
            value(2025, 6, 1050, 2020),
            value(2026, 1, 2000, 2015),
        ];
        let today = CalendarDate::from_ymd(2025, 7, 15).unwrap();
        let items = build(&[tenant()], &values, today);
        assert_eq!(items.len(), 1);
        let s = &items[0];
        assert_eq!(s.current_vpi_tenths, 1050);
        assert_eq!(s.percentage_change_basis_points, 500);
        assert_eq!(s.suggested_rent_cents, 68_250);
        assert_eq!(s.months_since_last_adjustment, 18);
        assert!(s.eligible);
        assert_eq!(s.property_address, "Musterweg 1, Beispielstadt");
    }

    #[test]
    fn skips_tenants_without_rent_or_index_value() {
        let today = CalendarDate::from_ymd(2025, 7, 15).unwrap();
        let mut no_rent = tenant();
        no_rent.current_rent_cents = None;
        let mut zero_rent = tenant();
        zero_rent.current_rent_cents = Some(0);
        let values = [value(2025, 6, 1050, 2020)];
        assert!(build(&[no_rent, zero_rent], &values, today).is_empty());
        assert!(build(&[tenant()], &[value(2025, 6, 1050, 2015)], today).is_empty());
    }

    #[test]
    fn too_early_adjustments_are_not_eligible() {
        let today = CalendarDate::from_ymd(2024, 6, 1).unwrap();
        let items = build(&[tenant()], &[value(2024, 5, 1010, 2020)], today);
        assert!(!items[0].eligible);
    }
}
