//! Period changes of neighbouring properties at the same time over the real router
//! (#116): the API wraps every billing change in its own (audited) transaction, which
//! must run under `READ COMMITTED` like the repositories. Two creates in neighbouring
//! properties, also of two accounts, both succeed; a move and a create in one property
//! still give exactly one overlap (the check sees the latest commit).

// Test bodies and helpers are plain async fns run by `dual_test!`.
#![allow(clippy::unwrap_used)]

mod masterdata_support;
mod support;

use serde_json::{Value, json};
use vermieterme_db::{Db, dual_test};

use masterdata_support::{World, id_of, world};

/// Rounds of each race (the deadlock showed in every round before #116).
const ROUNDS: u32 = 30;

/// Collection path of the periods.
const BASE: &str = "/api/billing-periods";

/// A property with one unit for `cookie`.
async fn property_with_unit(w: &World, cookie: &str) -> String {
    let id = w.property(cookie).await;
    w.unit(cookie, &id, 0).await;
    id
}

/// Body of a period of the property.
fn period(property: &str, start: &str, end: &str) -> Value {
    json!({"propertyId": property, "startDate": start, "endDate": end})
}

/// Both properties have 2025. One creates 2030 (behind all of its periods) while the
/// other creates 2020 (in front of its 2025), then the other way round: in index order
/// one of both directions is the deadlock of #116. Every create answers 201.
async fn both_directions(w: &World, one: (&str, &str), two: (&str, &str), round: u32) {
    for (cookie, property) in [one, two] {
        let body = period(property, "2025-01-01", "2025-12-31");
        w.expect(201, "POST", BASE, cookie, body).await;
    }
    for ((late_cookie, late), (early_cookie, early)) in [(one, two), (two, one)] {
        let (a, b) = tokio::join!(
            w.app.call(
                "POST",
                BASE,
                Some(late_cookie),
                period(late, "2030-01-01", "2030-12-31")
            ),
            w.app.call(
                "POST",
                BASE,
                Some(early_cookie),
                period(early, "2020-01-01", "2020-12-31")
            ),
        );
        assert_eq!(a.status, 201, "Runde {round}: {:?}", a.body);
        assert_eq!(b.status, 201, "Runde {round}: {:?}", b.body);
    }
}

/// Neighbouring properties of one account and of two accounts.
async fn neighbours(db: &Db) {
    let w = world(db).await;
    for round in 0..ROUNDS {
        let p1 = property_with_unit(&w, &w.a).await;
        let p2 = property_with_unit(&w, &w.a).await;
        both_directions(&w, (&w.a, &p1), (&w.a, &p2), round).await;
        let q1 = property_with_unit(&w, &w.a).await;
        let q2 = property_with_unit(&w, &w.b).await;
        both_directions(&w, (&w.a, &q1), (&w.b, &q2), round).await;
    }
}
dual_test!(neighbour_properties, neighbours);

/// Moving a period into a free range while another one is created there: exactly one
/// answers 409 `overlap`. Under `REPEATABLE READ` the move would check against its
/// snapshot from before the property lock and both would pass.
async fn move_vs_create(db: &Db) {
    let w = world(db).await;
    let property = property_with_unit(&w, &w.a).await;
    for round in 0..ROUNDS {
        let year = 2040 + round;
        let first = period(
            &property,
            &format!("{year}-01-01"),
            &format!("{year}-03-31"),
        );
        let id = id_of(&w.expect(201, "POST", BASE, &w.a, first).await);
        let shift =
            json!({"startDate": format!("{year}-07-01"), "endDate": format!("{year}-12-31")});
        let fresh = period(
            &property,
            &format!("{year}-09-01"),
            &format!("{year}-10-31"),
        );
        let path = format!("{BASE}/{id}");
        let (moved, made) = tokio::join!(
            w.app.call("PUT", &path, Some(&w.a), shift),
            w.app.call("POST", BASE, Some(&w.a), fresh),
        );
        let outcome = (moved.status, made.status);
        assert!(
            outcome == (200, 409) || outcome == (409, 201),
            "Runde {round}: {moved:?} / {made:?}"
        );
        let refused = if moved.status == 409 { &moved } else { &made };
        assert_eq!(refused.body["code"], "overlap", "Runde {round}");
    }
}
dual_test!(period_move_vs_create, move_vs_create);
