"use client";

import { useEffect, useState, useCallback } from "react";
import Link from "next/link";
import { Nav } from "@/components/nav";
import { Combobox } from "@/components/ui/combobox";
import { Loading } from "@/components/ui/loading";
import {
  SECTION_META,
  FONT_OPTIONS,
  getDefaultConfig,
  fontToCss,
} from "@/lib/pdf-template";
import type {
  PdfTemplateConfig,
  SectionId,
  SectionConfig,
  PdfFontFamily,
} from "@/types/pdf-template";

type Selection = SectionId | "page" | null;

export default function PdfTemplatePage() {
  const [config, setConfig] = useState<PdfTemplateConfig>(getDefaultConfig());
  const [selected, setSelected] = useState<Selection>(null);
  const [loading, setLoading] = useState(true);
  const [saving, setSaving] = useState(false);
  const [status, setStatus] = useState<string | null>(null);

  useEffect(() => {
    fetch("/api/pdf-template")
      .then((r) => r.json())
      .then((data) => setConfig(data.config))
      .catch(console.error)
      .finally(() => setLoading(false));
  }, []);

  const handleSave = useCallback(async () => {
    setSaving(true);
    setStatus(null);
    try {
      const res = await fetch("/api/pdf-template", {
        method: "PUT",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ config }),
      });
      if (res.ok) {
        setStatus("Gespeichert");
        setTimeout(() => setStatus(null), 2000);
      } else {
        setStatus("Fehler beim Speichern");
      }
    } catch {
      setStatus("Fehler beim Speichern");
    } finally {
      setSaving(false);
    }
  }, [config]);

  const handleReset = () => {
    setConfig(getDefaultConfig());
    setSelected(null);
  };

  const updateSection = (id: SectionId, patch: Partial<SectionConfig>) => {
    setConfig((prev) => ({
      ...prev,
      sections: {
        ...prev.sections,
        [id]: { ...prev.sections[id], ...patch },
      },
    }));
  };

  const updateSectionStyle = (
    id: SectionId,
    patch: Partial<SectionConfig["style"]>
  ) => {
    setConfig((prev) => ({
      ...prev,
      sections: {
        ...prev.sections,
        [id]: {
          ...prev.sections[id],
          style: { ...prev.sections[id].style, ...patch },
        },
      },
    }));
  };

  const updateSectionText = (id: SectionId, key: string, value: string) => {
    setConfig((prev) => ({
      ...prev,
      sections: {
        ...prev.sections,
        [id]: {
          ...prev.sections[id],
          texts: { ...prev.sections[id].texts, [key]: value },
        },
      },
    }));
  };

  if (loading) {
    return (
      <>
        <Nav />
        <main className="mx-auto max-w-7xl px-4 py-8 sm:px-6 lg:px-8">
          <Loading />
        </main>
      </>
    );
  }

  return (
    <>
      <Nav />
      <main className="mx-auto max-w-[1400px] px-4 py-8 sm:px-6 lg:px-8">
        {/* Header */}
        <div className="mb-6 flex items-center justify-between">
          <div>
            <div className="mb-1 flex items-center gap-2">
              <Link
                href="/settings"
                className="text-sm text-zinc-500 hover:text-zinc-700"
              >
                Einstellungen
              </Link>
              <span className="text-sm text-zinc-400">/</span>
              <h1 className="text-lg font-bold text-zinc-900">
                PDF-Vorlage bearbeiten
              </h1>
            </div>
            <p className="text-sm text-zinc-500">
              Klicken Sie auf einen Bereich in der Vorschau, um ihn zu
              bearbeiten.
            </p>
          </div>
          <div className="flex items-center gap-3">
            {status && (
              <span
                className={`text-sm ${status === "Gespeichert" ? "text-green-600" : "text-red-600"}`}
              >
                {status}
              </span>
            )}
            <button
              onClick={handleReset}
              className="rounded-lg border border-zinc-300 px-4 py-2 text-sm font-medium text-zinc-700 hover:bg-zinc-50"
            >
              Zurücksetzen
            </button>
            <button
              onClick={handleSave}
              disabled={saving}
              className="rounded-lg bg-red-700 px-4 py-2 text-sm font-medium text-white hover:bg-red-800 disabled:opacity-50"
            >
              {saving ? "Speichern..." : "Speichern"}
            </button>
          </div>
        </div>

        {/* Editor Layout */}
        <div className="flex gap-6">
          {/* Left: A4 Preview */}
          <div className="flex-1 overflow-auto">
            <PdfPreview
              config={config}
              selected={selected}
              onSelect={setSelected}
            />
          </div>

          {/* Right: Property Panel */}
          <div className="w-80 shrink-0">
            <div className="sticky top-4">
              {selected === null && (
                <div className="rounded-xl border border-zinc-200 bg-white p-6 shadow-sm">
                  <p className="text-sm text-zinc-500">
                    Klicken Sie auf einen Bereich in der Vorschau, um seine
                    Eigenschaften zu bearbeiten.
                  </p>
                  <p className="mt-3 text-sm text-zinc-500">
                    Klicken Sie auf den Seitenhintergrund für die
                    Seiten-Einstellungen.
                  </p>
                </div>
              )}
              {selected === "page" && (
                <PageEditor
                  config={config}
                  onChange={(page) => setConfig((prev) => ({ ...prev, page }))}
                />
              )}
              {selected && selected !== "page" && (
                <SectionEditor
                  sectionId={selected}
                  config={config.sections[selected]}
                  pageFont={config.page.fontFamily}
                  onUpdate={(patch) => updateSection(selected, patch)}
                  onUpdateStyle={(patch) => updateSectionStyle(selected, patch)}
                  onUpdateText={(key, value) =>
                    updateSectionText(selected, key, value)
                  }
                />
              )}
            </div>
          </div>
        </div>
      </main>
    </>
  );
}

// --- A4 Preview ---

const A4_WIDTH = 595;
const A4_HEIGHT = 842;
const PREVIEW_WIDTH = 520;
const SCALE = PREVIEW_WIDTH / A4_WIDTH;

function PdfPreview({
  config,
  selected,
  onSelect,
}: {
  config: PdfTemplateConfig;
  selected: Selection;
  onSelect: (s: Selection) => void;
}) {
  const { page, sections: s } = config;

  const sectionBox = (id: SectionId, children: React.ReactNode) => {
    if (!s[id].visible) return null;
    const isSelected = selected === id;
    const style = s[id].style;
    return (
      <div
        className={`relative cursor-pointer rounded transition-all ${
          isSelected
            ? "ring-2 ring-blue-400 bg-blue-50/40"
            : "hover:ring-1 hover:ring-blue-200 hover:bg-blue-50/20"
        }`}
        style={{
          fontFamily: fontToCss(style.fontFamily),
          fontSize: style.fontSize,
          fontWeight: style.bold ? "bold" : "normal",
          fontStyle: style.italic ? "italic" : "normal",
          textAlign: style.textAlign,
          color: style.color,
          marginTop: style.marginTop,
          marginBottom: style.marginBottom,
        }}
        onClick={(e) => {
          e.stopPropagation();
          onSelect(id);
        }}
      >
        {isSelected && (
          <span className="absolute -top-4 left-1 text-[9px] font-medium text-blue-500">
            {SECTION_META[id].label}
          </span>
        )}
        {children}
      </div>
    );
  };

  const titleText = (s.title.texts.title || "Neben-/Betriebskostenabrechnung {{year}}").replace(
    "{{year}}",
    "2024"
  );

  return (
    <div
      className="mx-auto overflow-hidden rounded-lg border border-zinc-300 bg-white shadow-md"
      style={{
        width: PREVIEW_WIDTH,
        height: A4_HEIGHT * SCALE,
      }}
    >
      <div
        className="origin-top-left"
        style={{
          width: A4_WIDTH,
          height: A4_HEIGHT,
          transform: `scale(${SCALE})`,
          fontFamily: fontToCss(page.fontFamily),
          fontSize: page.fontSize,
          padding: `${page.paddingTop}px ${page.paddingRight}px ${page.paddingBottom}px ${page.paddingLeft}px`,
        }}
        onClick={() => onSelect("page")}
      >
        {/* Header */}
        {sectionBox(
          "header",
          <div style={{ display: "flex", justifyContent: "space-between" }}>
            <div>
              <div
                style={{
                  fontSize: s.header.style.fontSize,
                  fontWeight: s.header.style.bold ? "bold" : "normal",
                }}
              >
                Max Mustermann
              </div>
              <div style={{ fontSize: page.fontSize }}>Musterstr. 1</div>
              <div style={{ fontSize: page.fontSize }}>
                12345 Musterstadt
              </div>
            </div>
            <div style={{ textAlign: "right", fontSize: page.fontSize }}>
              <div>0123/456789</div>
              <div>vermieter@beispiel.de</div>
            </div>
          </div>
        )}

        {/* Sender Line */}
        {sectionBox(
          "senderLine",
          <div style={{ textDecoration: "underline" }}>
            Max Mustermann - Musterstr. 1 - 12345 Musterstadt
          </div>
        )}

        {/* Recipient + Meta */}
        <div style={{ display: "flex", justifyContent: "space-between" }}>
          {sectionBox(
            "recipient",
            <div style={{ width: "100%" }}>
              <div>Herrn</div>
              <div>Hans Beispiel</div>
              <div>Musterstr. 1</div>
              <div>12345 Musterstadt</div>
            </div>
          )}
          {sectionBox(
            "meta",
            <div style={{ width: "100%" }}>
              {[
                ["Abrechnungszeitraum:", "01.01.2024 bis 31.12.2024"],
                ["Kalendertage gesamt:", "366"],
                ["Ihr Abrechnungszeitraum:", "01.01.2024 bis 31.12.2024"],
                ["Geschoss:", "Erdgeschoss"],
                ["Datum der Abrechnung:", "15.03.2025"],
              ].map(([label, value]) => (
                <div
                  key={label}
                  style={{ display: "flex", marginBottom: 2 }}
                >
                  <span style={{ width: "55%", fontSize: s.meta.style.fontSize }}>
                    {label}
                  </span>
                  <span style={{ width: "45%", fontSize: s.meta.style.fontSize }}>
                    {value}
                  </span>
                </div>
              ))}
            </div>
          )}
        </div>

        {/* Title */}
        {sectionBox("title", <div>{titleText}</div>)}

        {/* Object Line */}
        {sectionBox(
          "objectLine",
          <div>Objekt: Musterstr. 1, 12345 Musterstadt</div>
        )}

        {/* Cost Table */}
        {sectionBox(
          "costTable",
          <div>
            <div
              style={{
                display: "flex",
                borderBottom: "1px solid #000",
                paddingBottom: 4,
                marginBottom: 4,
                fontWeight: "bold",
                fontSize: s.costTable.style.fontSize,
              }}
            >
              <span style={{ width: "35%" }}>Kostenart</span>
              <span style={{ width: "20%", textAlign: "right" }}>Kosten</span>
              <span style={{ width: "25%", textAlign: "center" }}>
                Schlüssel
              </span>
              <span style={{ width: "20%", textAlign: "right" }}>
                Ihr Anteil
              </span>
            </div>
            {[
              ["Grundsteuer", "450,00", "laut Bescheid", "450,00"],
              ["Gebäudeversicherung", "890,50", "45 MEA", "400,73"],
              ["Müllabfuhr", "240,00", "45 MEA", "108,00"],
            ].map(([name, total, key, share]) => (
              <div
                key={name}
                style={{
                  display: "flex",
                  padding: "2px 0",
                  fontSize: s.costTable.style.fontSize,
                }}
              >
                <span style={{ width: "35%" }}>{name}</span>
                <span style={{ width: "20%", textAlign: "right" }}>
                  {total} EUR
                </span>
                <span style={{ width: "25%", textAlign: "center" }}>
                  {key}
                </span>
                <span style={{ width: "20%", textAlign: "right" }}>
                  {share} EUR
                </span>
              </div>
            ))}
            <div
              style={{
                display: "flex",
                borderTop: "1px solid #000",
                paddingTop: 4,
                marginTop: 4,
                fontWeight: "bold",
                fontSize: s.costTable.style.fontSize,
              }}
            >
              <span style={{ width: "35%" }}>gesamte Kosten:</span>
              <span style={{ width: "20%", textAlign: "right" }}>
                1.580,50 EUR
              </span>
              <span style={{ width: "25%", textAlign: "center" }}>
                Ihr Anteil:
              </span>
              <span style={{ width: "20%", textAlign: "right" }}>
                958,73 EUR
              </span>
            </div>
          </div>
        )}

        {/* Prepayment */}
        {sectionBox(
          "prepayment",
          <div style={{ display: "flex" }}>
            <span style={{ width: "80%" }}>
              Ihre Vorauszahlungen Neben-/Betriebskosten:
            </span>
            <span style={{ width: "20%", textAlign: "right" }}>
              1.200,00 EUR
            </span>
          </div>
        )}

        {/* Result */}
        {sectionBox(
          "result",
          <div
            style={{
              display: "flex",
              borderTop: "2px solid #000",
              borderBottom: "2px solid #000",
              padding: "6px 0",
            }}
          >
            <span style={{ width: "80%" }}>Erstattung:</span>
            <span style={{ width: "20%", textAlign: "right" }}>
              241,27 EUR
            </span>
          </div>
        )}

        {/* Distribution Keys */}
        {sectionBox(
          "distributionKeys",
          <div>
            <div style={{ fontWeight: "bold", marginBottom: 6, fontSize: 10 }}>
              Erläuterung der Verteilerschlüssel:
            </div>
            {[
              ["MEA", "Miteigentumsanteile laut Teilungserklärung", "45 / 100"],
              ["laut Bescheid", "laut Grundsteuerbescheid", ""],
            ].map(([k, desc, val]) => (
              <div key={k} style={{ display: "flex", padding: "2px 0" }}>
                <span style={{ width: "20%" }}>{k}</span>
                <span style={{ width: "55%" }}>{desc}</span>
                <span style={{ width: "25%", textAlign: "right" }}>{val}</span>
              </div>
            ))}
          </div>
        )}

        {/* Bank Info */}
        {sectionBox(
          "bankInfo",
          <div>
            <div style={{ fontWeight: "bold", marginBottom: 4 }}>
              Bankverbindung und Zahlungsdetails
            </div>
            <div>Max Mustermann, IBAN: DE89 3704 0044 0532 0130 00, Sparkasse</div>
            <div style={{ marginTop: 4, fontSize: 9 }}>
              {s.bankInfo.texts.paymentNote ||
                "Nachzahlungen sind, sofern nicht anders vereinbart, einen Monat nach Zugang der Abrechnung fällig."}
            </div>
          </div>
        )}

        {/* Closing */}
        {sectionBox(
          "closing",
          <div>
            <div style={{ marginBottom: 8 }}>
              {s.closing.texts.text ||
                "Falls Sie Fragen zu der Abrechnung haben oder Belege einsehen möchten, stehe ich Ihnen gerne zur Verfügung."}
            </div>
            <div style={{ marginBottom: 4 }}>
              {s.closing.texts.greeting || "Viele Grüße"}
            </div>
            <div>Max Mustermann</div>
          </div>
        )}
      </div>
    </div>
  );
}

// --- Page Editor ---

function PageEditor({
  config,
  onChange,
}: {
  config: PdfTemplateConfig;
  onChange: (page: PdfTemplateConfig["page"]) => void;
}) {
  const { page } = config;

  return (
    <div className="rounded-xl border border-zinc-200 bg-white p-5 shadow-sm">
      <h3 className="mb-1 text-sm font-semibold text-zinc-900">
        Seiten-Einstellungen
      </h3>
      <p className="mb-4 text-xs text-zinc-500">
        Globale Einstellungen für die gesamte Seite
      </p>

      <div className="space-y-3">
        <div>
          <label className="mb-1 block text-xs font-medium text-zinc-600">
            Standard-Schriftart
          </label>
          <Combobox
            options={FONT_OPTIONS.map((f) => ({ value: f.value, label: f.label }))}
            value={page.fontFamily}
            onChange={(val) => onChange({ ...page, fontFamily: val as PdfFontFamily })}
            placeholder="Schriftart wählen..."
          />
        </div>

        <div>
          <label className="mb-1 block text-xs font-medium text-zinc-600">
            Standard-Schriftgröße (pt)
          </label>
          <input
            type="number"
            min={6}
            max={24}
            value={page.fontSize}
            onChange={(e) =>
              onChange({ ...page, fontSize: parseInt(e.target.value) || 10 })
            }
            className="w-full rounded-lg border border-zinc-300 px-3 py-1.5 text-sm"
          />
        </div>

        <div className="border-t border-zinc-100 pt-3">
          <p className="mb-2 text-xs font-medium text-zinc-600">
            Seitenränder (pt)
          </p>
          <div className="grid grid-cols-2 gap-2">
            {(["paddingTop", "paddingBottom", "paddingLeft", "paddingRight"] as const).map(
              (key) => (
                <div key={key}>
                  <label className="mb-0.5 block text-[10px] text-zinc-400">
                    {key === "paddingTop"
                      ? "Oben"
                      : key === "paddingBottom"
                        ? "Unten"
                        : key === "paddingLeft"
                          ? "Links"
                          : "Rechts"}
                  </label>
                  <input
                    type="number"
                    min={10}
                    max={100}
                    value={page[key]}
                    onChange={(e) =>
                      onChange({
                        ...page,
                        [key]: parseInt(e.target.value) || 40,
                      })
                    }
                    className="w-full rounded border border-zinc-300 px-2 py-1 text-sm"
                  />
                </div>
              )
            )}
          </div>
        </div>
      </div>
    </div>
  );
}

// --- Section Editor ---

function SectionEditor({
  sectionId,
  config,
  pageFont,
  onUpdate,
  onUpdateStyle,
  onUpdateText,
}: {
  sectionId: SectionId;
  config: SectionConfig;
  pageFont: PdfFontFamily;
  onUpdate: (patch: Partial<SectionConfig>) => void;
  onUpdateStyle: (patch: Partial<SectionConfig["style"]>) => void;
  onUpdateText: (key: string, value: string) => void;
}) {
  const meta = SECTION_META[sectionId];
  const { style } = config;

  return (
    <div className="rounded-xl border border-zinc-200 bg-white p-5 shadow-sm">
      <h3 className="mb-0.5 text-sm font-semibold text-zinc-900">
        {meta.label}
      </h3>
      <p className="mb-4 text-xs text-zinc-500">{meta.description}</p>

      <div className="space-y-3">
        {/* Visibility */}
        <label className="flex items-center gap-2 text-sm">
          <input
            type="checkbox"
            checked={config.visible}
            onChange={(e) => onUpdate({ visible: e.target.checked })}
            className="rounded border-zinc-300"
          />
          <span className="text-zinc-700">Sichtbar</span>
        </label>

        {/* Font Family */}
        <div>
          <label className="mb-1 block text-xs font-medium text-zinc-600">
            Schriftart
          </label>
          <Combobox
            options={[
              { value: pageFont, label: `Standard (${pageFont})` },
              ...FONT_OPTIONS.filter((f) => f.value !== pageFont).map((f) => ({ value: f.value, label: f.label })),
            ]}
            value={style.fontFamily}
            onChange={(val) => onUpdateStyle({ fontFamily: val as PdfFontFamily })}
            placeholder="Schriftart wählen..."
          />
        </div>

        {/* Font Size */}
        <div>
          <label className="mb-1 block text-xs font-medium text-zinc-600">
            Schriftgröße (pt)
          </label>
          <input
            type="number"
            min={5}
            max={30}
            value={style.fontSize}
            onChange={(e) =>
              onUpdateStyle({
                fontSize: parseInt(e.target.value) || 10,
              })
            }
            className="w-full rounded-lg border border-zinc-300 px-3 py-1.5 text-sm"
          />
        </div>

        {/* Bold + Italic */}
        <div className="flex gap-2">
          <button
            type="button"
            onClick={() => onUpdateStyle({ bold: !style.bold })}
            className={`rounded-lg border px-3 py-1.5 text-sm font-bold ${
              style.bold
                ? "border-zinc-800 bg-zinc-800 text-white"
                : "border-zinc-300 text-zinc-700 hover:bg-zinc-50"
            }`}
          >
            F
          </button>
          <button
            type="button"
            onClick={() => onUpdateStyle({ italic: !style.italic })}
            className={`rounded-lg border px-3 py-1.5 text-sm italic ${
              style.italic
                ? "border-zinc-800 bg-zinc-800 text-white"
                : "border-zinc-300 text-zinc-700 hover:bg-zinc-50"
            }`}
          >
            K
          </button>
        </div>

        {/* Text Alignment */}
        <div>
          <label className="mb-1 block text-xs font-medium text-zinc-600">
            Ausrichtung
          </label>
          <div className="flex gap-1">
            {(["left", "center", "right"] as const).map((align) => (
              <button
                key={align}
                type="button"
                onClick={() => onUpdateStyle({ textAlign: align })}
                className={`flex-1 rounded-lg border px-2 py-1.5 text-xs ${
                  style.textAlign === align
                    ? "border-zinc-800 bg-zinc-800 text-white"
                    : "border-zinc-300 text-zinc-700 hover:bg-zinc-50"
                }`}
              >
                {align === "left"
                  ? "Links"
                  : align === "center"
                    ? "Mitte"
                    : "Rechts"}
              </button>
            ))}
          </div>
        </div>

        {/* Color */}
        <div>
          <label className="mb-1 block text-xs font-medium text-zinc-600">
            Farbe
          </label>
          <div className="flex items-center gap-2">
            <input
              type="color"
              value={style.color}
              onChange={(e) => onUpdateStyle({ color: e.target.value })}
              className="h-8 w-8 cursor-pointer rounded border border-zinc-300"
            />
            <input
              type="text"
              value={style.color}
              onChange={(e) => onUpdateStyle({ color: e.target.value })}
              className="flex-1 rounded-lg border border-zinc-300 px-3 py-1.5 text-sm"
            />
          </div>
        </div>

        {/* Margins */}
        <div className="border-t border-zinc-100 pt-3">
          <p className="mb-2 text-xs font-medium text-zinc-600">
            Abstände (pt)
          </p>
          <div className="grid grid-cols-2 gap-2">
            <div>
              <label className="mb-0.5 block text-[10px] text-zinc-400">
                Oben
              </label>
              <input
                type="number"
                min={0}
                max={80}
                value={style.marginTop}
                onChange={(e) =>
                  onUpdateStyle({
                    marginTop: parseInt(e.target.value) || 0,
                  })
                }
                className="w-full rounded border border-zinc-300 px-2 py-1 text-sm"
              />
            </div>
            <div>
              <label className="mb-0.5 block text-[10px] text-zinc-400">
                Unten
              </label>
              <input
                type="number"
                min={0}
                max={80}
                value={style.marginBottom}
                onChange={(e) =>
                  onUpdateStyle({
                    marginBottom: parseInt(e.target.value) || 0,
                  })
                }
                className="w-full rounded border border-zinc-300 px-2 py-1 text-sm"
              />
            </div>
          </div>
        </div>

        {/* Custom Text Fields */}
        {meta.textFields && meta.textFields.length > 0 && (
          <div className="border-t border-zinc-100 pt-3">
            <p className="mb-2 text-xs font-medium text-zinc-600">
              Text-Inhalte
            </p>
            {meta.textFields.map((field) => (
              <div key={field.key} className="mb-2">
                <label className="mb-0.5 block text-[10px] text-zinc-400">
                  {field.label}
                </label>
                {field.multiline ? (
                  <textarea
                    value={config.texts[field.key] || ""}
                    onChange={(e) => onUpdateText(field.key, e.target.value)}
                    rows={3}
                    className="w-full rounded-lg border border-zinc-300 px-3 py-1.5 text-sm"
                  />
                ) : (
                  <input
                    type="text"
                    value={config.texts[field.key] || ""}
                    onChange={(e) => onUpdateText(field.key, e.target.value)}
                    className="w-full rounded-lg border border-zinc-300 px-3 py-1.5 text-sm"
                  />
                )}
              </div>
            ))}
          </div>
        )}
      </div>
    </div>
  );
}
