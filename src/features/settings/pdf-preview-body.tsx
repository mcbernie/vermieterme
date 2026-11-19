"use client";

import type { CSSProperties, ReactNode } from "react";
import type { PdfTemplateConfig, SectionId } from "@/types/pdf-template";

/** Renders a section of the preview; receives its id and content, returns null when hidden. */
export type SectionBox = (id: SectionId, children: ReactNode) => ReactNode;

/** Black rule of the printed table (part of the paper imitation, not of the app theme). */
const RULE = "1px solid #000";
const RULE_STRONG = "2px solid #000";

const META_ROWS = [
  ["Abrechnungszeitraum:", "01.01.2024 bis 31.12.2024"],
  ["Kalendertage gesamt:", "366"],
  ["Ihr Abrechnungszeitraum:", "01.01.2024 bis 31.12.2024"],
  ["Geschoss:", "Erdgeschoss"],
  ["Datum der Abrechnung:", "15.03.2025"],
];

const COST_ROWS = [
  ["Grundsteuer", "450,00", "laut Bescheid", "450,00"],
  ["Gebäudeversicherung", "890,50", "45 MEA", "400,73"],
  ["Müllabfuhr", "240,00", "45 MEA", "108,00"],
];

const KEY_ROWS = [
  ["MEA", "Miteigentumsanteile laut Teilungserklärung", "45 / 100"],
  ["laut Bescheid", "laut Grundsteuerbescheid", ""],
];

interface Props {
  config: PdfTemplateConfig;
  box: SectionBox;
}

/** Four-column row of the cost table (name, total, key, share). */
function CostRow({ cells, fontSize, style }: { cells: string[]; fontSize: number; style?: CSSProperties }) {
  const [name, total, key, share] = cells;
  return (
    <div style={{ display: "flex", fontSize, ...style }}>
      <span style={{ width: "35%" }}>{name}</span>
      <span style={{ width: "20%", textAlign: "right" }}>{total}</span>
      <span style={{ width: "25%", textAlign: "center" }}>{key}</span>
      <span style={{ width: "20%", textAlign: "right" }}>{share}</span>
    </div>
  );
}

/** Sample content of the billing letter, every part wrapped by `box` so it can be selected. */
export function BillingBody({ config, box }: Props) {
  const { page, sections: s } = config;
  const title = (s.title.texts.title || "Neben-/Betriebskostenabrechnung {{year}}").replace("{{year}}", "2024");
  const costSize = s.costTable.style.fontSize;

  return (
    <>
      {box(
        "header",
        <div style={{ display: "flex", justifyContent: "space-between" }}>
          <div>
            <div style={{ fontSize: s.header.style.fontSize, fontWeight: s.header.style.bold ? "bold" : "normal" }}>
              Max Mustermann
            </div>
            <div style={{ fontSize: page.fontSize }}>Musterstr. 1</div>
            <div style={{ fontSize: page.fontSize }}>12345 Musterstadt</div>
          </div>
          <div style={{ textAlign: "right", fontSize: page.fontSize }}>
            <div>0123/456789</div>
            <div>vermieter@beispiel.de</div>
          </div>
        </div>,
      )}

      {box(
        "senderLine",
        <div style={{ textDecoration: "underline" }}>Max Mustermann - Musterstr. 1 - 12345 Musterstadt</div>,
      )}

      <div style={{ display: "flex", justifyContent: "space-between" }}>
        {box(
          "recipient",
          <div style={{ width: "100%" }}>
            <div>Herrn</div>
            <div>Hans Beispiel</div>
            <div>Musterstr. 1</div>
            <div>12345 Musterstadt</div>
          </div>,
        )}
        {box(
          "meta",
          <div style={{ width: "100%" }}>
            {META_ROWS.map(([label, value]) => (
              <div key={label} style={{ display: "flex", marginBottom: 2 }}>
                <span style={{ width: "55%", fontSize: s.meta.style.fontSize }}>{label}</span>
                <span style={{ width: "45%", fontSize: s.meta.style.fontSize }}>{value}</span>
              </div>
            ))}
          </div>,
        )}
      </div>

      {box("title", <div>{title}</div>)}
      {box("objectLine", <div>Objekt: Musterstr. 1, 12345 Musterstadt</div>)}

      {box(
        "costTable",
        <div>
          <CostRow
            cells={["Kostenart", "Kosten", "Schlüssel", "Ihr Anteil"]}
            fontSize={costSize}
            style={{ borderBottom: RULE, paddingBottom: 4, marginBottom: 4, fontWeight: "bold" }}
          />
          {COST_ROWS.map(([name, total, key, share]) => (
            <CostRow
              key={name}
              cells={[name, `${total} EUR`, key, `${share} EUR`]}
              fontSize={costSize}
              style={{ padding: "2px 0" }}
            />
          ))}
          <CostRow
            cells={["gesamte Kosten:", "1.580,50 EUR", "Ihr Anteil:", "958,73 EUR"]}
            fontSize={costSize}
            style={{ borderTop: RULE, paddingTop: 4, marginTop: 4, fontWeight: "bold" }}
          />
        </div>,
      )}

      {box(
        "prepayment",
        <div style={{ display: "flex" }}>
          <span style={{ width: "80%" }}>Ihre Vorauszahlungen Neben-/Betriebskosten:</span>
          <span style={{ width: "20%", textAlign: "right" }}>1.200,00 EUR</span>
        </div>,
      )}

      {box(
        "result",
        <div style={{ display: "flex", borderTop: RULE_STRONG, borderBottom: RULE_STRONG, padding: "6px 0" }}>
          <span style={{ width: "80%" }}>Erstattung:</span>
          <span style={{ width: "20%", textAlign: "right" }}>241,27 EUR</span>
        </div>,
      )}

      {box(
        "distributionKeys",
        <div>
          <div style={{ fontWeight: "bold", marginBottom: 6, fontSize: 10 }}>Erläuterung der Verteilerschlüssel:</div>
          {KEY_ROWS.map(([key, description, value]) => (
            <div key={key} style={{ display: "flex", padding: "2px 0" }}>
              <span style={{ width: "20%" }}>{key}</span>
              <span style={{ width: "55%" }}>{description}</span>
              <span style={{ width: "25%", textAlign: "right" }}>{value}</span>
            </div>
          ))}
        </div>,
      )}

      {box(
        "bankInfo",
        <div>
          <div style={{ fontWeight: "bold", marginBottom: 4 }}>Bankverbindung und Zahlungsdetails</div>
          <div>Max Mustermann, IBAN: DE89 3704 0044 0532 0130 00, Sparkasse</div>
          <div style={{ marginTop: 4, fontSize: 9 }}>
            {s.bankInfo.texts.paymentNote ||
              "Nachzahlungen sind, sofern nicht anders vereinbart, einen Monat nach Zugang der Abrechnung fällig."}
          </div>
        </div>,
      )}

      {box(
        "closing",
        <div>
          <div style={{ marginBottom: 8 }}>
            {s.closing.texts.text ||
              "Falls Sie Fragen zu der Abrechnung haben oder Belege einsehen möchten, stehe ich Ihnen gerne zur Verfügung."}
          </div>
          <div style={{ marginBottom: 4 }}>{s.closing.texts.greeting || "Viele Grüße"}</div>
          <div>Max Mustermann</div>
        </div>,
      )}
    </>
  );
}
