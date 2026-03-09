export type PdfFontFamily = "Helvetica" | "Times-Roman" | "Courier";

export type SectionId =
  | "header"
  | "senderLine"
  | "recipient"
  | "meta"
  | "title"
  | "objectLine"
  | "costTable"
  | "prepayment"
  | "result"
  | "distributionKeys"
  | "bankInfo"
  | "closing";

export interface SectionStyle {
  fontFamily: PdfFontFamily;
  fontSize: number;
  bold: boolean;
  italic: boolean;
  textAlign: "left" | "center" | "right";
  color: string;
  marginTop: number;
  marginBottom: number;
}

export interface SectionConfig {
  visible: boolean;
  style: SectionStyle;
  texts: Record<string, string>;
}

export interface PageConfig {
  fontFamily: PdfFontFamily;
  fontSize: number;
  paddingTop: number;
  paddingBottom: number;
  paddingLeft: number;
  paddingRight: number;
}

export interface PdfTemplateConfig {
  page: PageConfig;
  sections: Record<SectionId, SectionConfig>;
}

export interface SectionMeta {
  label: string;
  description: string;
  textFields?: Array<{ key: string; label: string; multiline?: boolean }>;
}
