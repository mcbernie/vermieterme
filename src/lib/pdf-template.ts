import type {
  PdfFontFamily,
  PdfTemplateConfig,
  SectionId,
  SectionConfig,
  SectionMeta,
} from "@/types/pdf-template";

// --- Section metadata for the editor UI ---

export const SECTION_META: Record<SectionId, SectionMeta> = {
  header: {
    label: "Kopfzeile",
    description: "Name und Kontaktdaten des Vermieters",
  },
  senderLine: {
    label: "Absenderzeile",
    description: "Kompakte Adresszeile unter der Kopfzeile",
  },
  recipient: {
    label: "Empfänger",
    description: "Adresse des Mieters",
  },
  meta: {
    label: "Abrechnungsdetails",
    description: "Zeitraum, Geschoss, Datum",
  },
  title: {
    label: "Titel",
    description: "Überschrift der Abrechnung",
    textFields: [
      {
        key: "title",
        label: "Titel-Text ({{year}} wird durch das Jahr ersetzt)",
      },
    ],
  },
  objectLine: {
    label: "Objektzeile",
    description: "Adresse des Mietobjekts",
  },
  costTable: {
    label: "Kostentabelle",
    description: "Aufstellung der Betriebskosten",
  },
  prepayment: {
    label: "Vorauszahlungen",
    description: "Zeile mit den geleisteten Vorauszahlungen",
  },
  result: {
    label: "Ergebnis",
    description: "Nachzahlung oder Erstattung",
  },
  distributionKeys: {
    label: "Verteilerschlüssel",
    description: "Erläuterung der verwendeten Schlüssel",
  },
  bankInfo: {
    label: "Bankverbindung",
    description: "Zahlungsdetails und Hinweis",
    textFields: [
      { key: "paymentNote", label: "Zahlungshinweis", multiline: true },
    ],
  },
  closing: {
    label: "Schlusstext",
    description: "Abschließende Nachricht und Grußformel",
    textFields: [
      { key: "text", label: "Schlusstext", multiline: true },
      { key: "greeting", label: "Grußformel" },
    ],
  },
};

export const SECTION_ORDER: SectionId[] = [
  "header",
  "senderLine",
  "recipient",
  "meta",
  "title",
  "objectLine",
  "costTable",
  "prepayment",
  "result",
  "distributionKeys",
  "bankInfo",
  "closing",
];

export const FONT_OPTIONS: { value: PdfFontFamily; label: string }[] = [
  { value: "Helvetica", label: "Helvetica (Sans-Serif)" },
  { value: "Times-Roman", label: "Times (Serif)" },
  { value: "Courier", label: "Courier (Monospace)" },
];

// --- Default config matching current PDF ---

function section(
  overrides: {
    visible?: boolean;
    style?: Partial<SectionConfig["style"]>;
    texts?: Record<string, string>;
  } = {}
): SectionConfig {
  const baseStyle: SectionConfig["style"] = {
    fontFamily: "Helvetica",
    fontSize: 10,
    bold: false,
    italic: false,
    textAlign: "left",
    color: "#000000",
    marginTop: 0,
    marginBottom: 0,
  };
  return {
    visible: overrides.visible ?? true,
    style: { ...baseStyle, ...overrides.style },
    texts: overrides.texts ?? {},
  };
}

export function getDefaultConfig(): PdfTemplateConfig {
  return {
    page: {
      fontFamily: "Helvetica",
      fontSize: 10,
      paddingTop: 40,
      paddingBottom: 40,
      paddingLeft: 50,
      paddingRight: 50,
    },
    sections: {
      header: section({ style: { fontSize: 12, bold: true, marginBottom: 20 } }),
      senderLine: section({ style: { fontSize: 7, color: "#555555", marginBottom: 4 } }),
      recipient: section({ style: { marginBottom: 10 } }),
      meta: section({ style: { fontSize: 8, marginBottom: 20 } }),
      title: section({
        style: { fontSize: 14, bold: true, textAlign: "center", marginBottom: 12 },
        texts: { title: "Neben-/Betriebskostenabrechnung {{year}}" },
      }),
      objectLine: section({ style: { bold: true, marginBottom: 12 } }),
      costTable: section({ style: { marginBottom: 12 } }),
      prepayment: section({ style: { marginTop: 8, marginBottom: 4 } }),
      result: section({ style: { bold: true, marginTop: 4 } }),
      distributionKeys: section({ style: { fontSize: 9, marginTop: 20, marginBottom: 16 } }),
      bankInfo: section({
        style: { marginTop: 16, marginBottom: 16 },
        texts: {
          paymentNote:
            "Nachzahlungen sind, sofern nicht anders vereinbart, einen Monat nach Zugang der Abrechnung fällig.",
        },
      }),
      closing: section({
        style: { marginTop: 20 },
        texts: {
          text: "Falls Sie Fragen zu der Abrechnung haben oder Belege einsehen möchten, stehe ich Ihnen gerne zur Verfügung.",
          greeting: "Viele Grüße",
        },
      }),
    },
  };
}

// --- Font resolution for @react-pdf ---

export function resolvePdfFont(
  family: PdfFontFamily,
  bold: boolean,
  italic: boolean
): string {
  if (family === "Helvetica") {
    if (bold && italic) return "Helvetica-BoldOblique";
    if (bold) return "Helvetica-Bold";
    if (italic) return "Helvetica-Oblique";
    return "Helvetica";
  }
  if (family === "Times-Roman") {
    if (bold && italic) return "Times-BoldItalic";
    if (bold) return "Times-Bold";
    if (italic) return "Times-Italic";
    return "Times-Roman";
  }
  if (family === "Courier") {
    if (bold && italic) return "Courier-BoldOblique";
    if (bold) return "Courier-Bold";
    if (italic) return "Courier-Oblique";
    return "Courier";
  }
  return family;
}

// --- CSS font mapping for preview ---

export function fontToCss(family: PdfFontFamily): string {
  switch (family) {
    case "Helvetica":
      return "Helvetica, Arial, sans-serif";
    case "Times-Roman":
      return "'Times New Roman', Times, serif";
    case "Courier":
      return "'Courier New', Courier, monospace";
    default:
      return "sans-serif";
  }
}
