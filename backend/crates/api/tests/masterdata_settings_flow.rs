//! Cost categories, landlord (`/api/settings`) and PDF template over the real router.

// Test bodies and helpers are plain async fns run by `dual_test!`.
#![allow(clippy::unwrap_used)]

mod masterdata_support;
mod support;

use serde_json::{Value, json};
use support::audit_count;
use vermieterme_db::{Db, dual_test};

use masterdata_support::{id_of, world};

/// Cost categories: key codes, order, validation, a category with costs cannot be deleted
/// (409), foreign ids are 404.
async fn categories(db: &Db) {
    let w = world(db).await;
    let body = |name: &str, key: &str, order: i64| json!({"name": name, "distributionKey": key, "sortOrder": order});
    let water = w
        .expect(
            201,
            "POST",
            "/api/cost-categories",
            &w.a,
            body("Wasser", "mea", 2),
        )
        .await;
    w.expect(
        201,
        "POST",
        "/api/cost-categories",
        &w.a,
        body("Müll", "per_unit", 1),
    )
    .await;
    let path = format!("/api/cost-categories/{}", id_of(&water));

    let list = w.app.get("/api/cost-categories", Some(&w.a)).await;
    assert_eq!(list.body[0]["name"], "Müll");
    assert_eq!(list.body[0]["distributionKey"], "per_unit");
    assert_eq!(list.body[1]["sortOrder"], 2);

    for (bad, field) in [
        (body("Gas", "MEA", 0), "distributionKey"),
        (body("Gas", "laut Bescheid", 0), "distributionKey"),
        (body(" ", "mea", 0), "name"),
        (body("Gas", "mea", 5_000_000), "sortOrder"),
    ] {
        let resp = w
            .expect(400, "POST", "/api/cost-categories", &w.a, bad)
            .await;
        assert!(resp.body["fields"][field].is_string(), "{field}");
    }
    let resp = w
        .expect(
            200,
            "PUT",
            &path,
            &w.a,
            body("Wasser/Abwasser", "per_notice", 3),
        )
        .await;
    assert_eq!(resp.body["distributionKey"], "per_notice");

    assert_eq!(
        w.app.get("/api/cost-categories", Some(&w.b)).await.body,
        json!([])
    );
    w.expect(404, "PUT", &path, &w.b, body("X", "mea", 0)).await;
    w.expect(404, "DELETE", &path, &w.b, Value::Null).await;

    // A cost of a billing period uses the category (billing endpoints come with P5).
    let pid = w.property(&w.a).await;
    let mut conn = db.pool().acquire().await.unwrap();
    for (sql, second) in [
        (
            "INSERT INTO billing_period (id, account_id, property_id, start_date, end_date, created_at, updated_at) VALUES ('bp1', (SELECT account_id FROM property WHERE id = ?), ?, '2024-01-01', '2024-12-31', 1, 1)",
            None,
        ),
        (
            "INSERT INTO cost (id, account_id, billing_period_id, cost_category_id, created_at, updated_at) VALUES ('c1', (SELECT account_id FROM property WHERE id = ?), 'bp1', ?, 1, 1)",
            Some(id_of(&water)),
        ),
    ] {
        let first = &pid;
        let q = sqlx::query(sql).bind(first);
        match &second {
            Some(cat) => q.bind(cat),
            None => q.bind(first),
        }
        .execute(&mut *conn)
        .await
        .unwrap();
    }
    drop(conn);
    let resp = w.expect(409, "DELETE", &path, &w.a, Value::Null).await;
    assert_eq!(resp.body["code"], "conflict");
    assert_eq!(
        w.app
            .get("/api/cost-categories", Some(&w.a))
            .await
            .body
            .as_array()
            .unwrap()
            .len(),
        2
    );
    let mut conn = db.pool().acquire().await.unwrap();
    sqlx::query("DELETE FROM cost WHERE id = 'c1'")
        .execute(&mut *conn)
        .await
        .unwrap();
    drop(conn);
    w.expect(204, "DELETE", &path, &w.a, Value::Null).await;
    w.expect(404, "DELETE", &path, &w.a, Value::Null).await;
}
dual_test!(categories_rules_and_isolation, categories);

/// Landlord: `null` first, PUT creates, PUT replaces, validation, per customer.
async fn landlord(db: &Db) {
    let w = world(db).await;
    let first = w.app.get("/api/settings", Some(&w.a)).await;
    assert_eq!((first.status, first.body), (200, Value::Null));

    let body = json!({"name": " Max Vermieter ", "street": "Weg 2", "zip": "54321", "city": "Stadt",
        "phone": "", "iban": "DE89370400440532013000", "email": "Max@Example.org"});
    let saved = w.expect(200, "PUT", "/api/settings", &w.a, body).await;
    assert_eq!(saved.body["name"], "Max Vermieter");
    assert_eq!(saved.body["iban"], "DE89 3704 0044 0532 0130 00");
    assert_eq!(saved.body["email"], "max@example.org");
    assert!(saved.body["phone"].is_null());

    let again = json!({"name": "Max", "street": "Weg 3", "zip": "1", "city": "S"});
    let replaced = w.expect(200, "PUT", "/api/settings", &w.a, again).await;
    assert_eq!(id_of(&replaced), id_of(&saved), "eine Zeile je Kunde");
    assert!(replaced.body["iban"].is_null());
    assert_eq!(
        w.app.get("/api/settings", Some(&w.a)).await.body["street"],
        "Weg 3"
    );
    assert_eq!(
        w.app.get("/api/settings", Some(&w.b)).await.body,
        Value::Null
    );

    for (patch, field) in [
        (json!({"name": ""}), "name"),
        (json!({"iban": "DE00"}), "iban"),
        (json!({"email": "x"}), "email"),
        (json!({"zip": "x".repeat(17)}), "zip"),
    ] {
        let mut bad = json!({"name": "M", "street": "S", "zip": "1", "city": "C"});
        bad.as_object_mut()
            .unwrap()
            .extend(patch.as_object().unwrap().clone());
        let resp = w.expect(400, "PUT", "/api/settings", &w.a, bad).await;
        assert!(resp.body["fields"][field].is_string(), "{field}");
    }
    w.expect(400, "PUT", "/api/settings", &w.a, json!({"name": "M"}))
        .await;
    assert_eq!(
        w.app
            .send("POST", "/api/settings", Some(&w.a), None, &[])
            .await
            .status,
        405
    );
}
dual_test!(landlord_per_customer, landlord);

/// PDF template: default first, PUT validates, stored per customer, audit as admin.
async fn pdf_template(db: &Db) {
    let w = world(db).await;
    let default = w.app.get("/api/pdf-template", Some(&w.a)).await;
    assert_eq!(default.status, 200);
    assert!(default.body["id"].is_null());
    assert_eq!(default.body["name"], "Standard");
    assert_eq!(
        default.body["config"]["sections"]["closing"]["texts"]["greeting"],
        "Viele Grüße"
    );

    let mut config = default.body["config"].clone();
    config["sections"]["closing"]["texts"]["greeting"] = json!("Mit freundlichen Grüßen");
    config["extra"] = json!({"bleibt": 1});
    let saved = w
        .expect(
            200,
            "PUT",
            "/api/pdf-template",
            &w.a,
            json!({"config": config}),
        )
        .await;
    assert_eq!(saved.body["name"], "Standard");
    let renamed = w
        .expect(
            200,
            "PUT",
            "/api/pdf-template",
            &w.a,
            json!({"config": config, "name": " Mein Entwurf "}),
        )
        .await;
    assert_eq!(id_of(&renamed), id_of(&saved));
    let stored = w.app.get("/api/pdf-template", Some(&w.a)).await;
    assert_eq!(stored.body["name"], "Mein Entwurf");
    assert_eq!(stored.body["config"], config);
    let other = w.app.get("/api/pdf-template", Some(&w.b)).await;
    assert!(
        other.body["id"].is_null(),
        "Vorlage gehört nur dem Kunden A"
    );

    let mut broken = config.clone();
    broken["page"]["fontFamily"] = json!("Comic Sans");
    let mut missing = config.clone();
    missing["sections"].as_object_mut().unwrap().remove("title");
    let mut huge = config.clone();
    huge["sections"]["closing"]["texts"]["text"] = json!("x".repeat(2000));
    for n in 0..6 {
        huge["sections"]["header"]["texts"][format!("k{n}")] = json!("y".repeat(2000));
    }
    for bad in [broken, missing, huge, json!("text"), Value::Null] {
        let resp = w
            .expect(
                400,
                "PUT",
                "/api/pdf-template",
                &w.a,
                json!({"config": bad}),
            )
            .await;
        assert!(resp.body["fields"]["config"].is_string(), "{:?}", resp.body);
    }
    w.expect(400, "PUT", "/api/pdf-template", &w.a, json!({}))
        .await;

    assert_eq!(audit_count(db, "pdf_template.save").await, 0);
    w.admin_enters_a().await;
    w.expect(
        200,
        "PUT",
        "/api/pdf-template",
        &w.admin,
        json!({"config": config}),
    )
    .await;
    assert_eq!(audit_count(db, "pdf_template.save").await, 1);
}
dual_test!(pdf_template_per_customer, pdf_template);
