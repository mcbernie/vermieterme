import { prisma } from "@/lib/prisma";
import { auth } from "@/lib/auth";
import React from "react";
import {
  Document,
  Page,
  Text,
  View,
  StyleSheet,
  renderToBuffer,
} from "@react-pdf/renderer";
import { getDefaultConfig, resolvePdfFont } from "@/lib/pdf-template";
import type { PdfTemplateConfig } from "@/types/pdf-template";

// --- Helpers ---

function formatCurrency(value: number): string {
  return value
    .toFixed(2)
    .replace(".", ",")
    .replace(/\B(?=(\d{3})+(?!\d))/g, ".");
}

function formatDate(date: Date): string {
  const d = new Date(date);
  const day = String(d.getDate()).padStart(2, "0");
  const month = String(d.getMonth() + 1).padStart(2, "0");
  const year = d.getFullYear();
  return `${day}.${month}.${year}`;
}

function daysBetween(start: Date, end: Date): number {
  const msPerDay = 1000 * 60 * 60 * 24;
  return Math.round((end.getTime() - start.getTime()) / msPerDay) + 1;
}

// --- Dynamic Styles from Template Config ---

function createStyles(cfg: PdfTemplateConfig) {
  const { page, sections: s } = cfg;
  const pageFont = resolvePdfFont(page.fontFamily, false, false);

  return StyleSheet.create({
    page: {
      fontFamily: pageFont,
      fontSize: page.fontSize,
      paddingTop: page.paddingTop,
      paddingBottom: page.paddingBottom,
      paddingLeft: page.paddingLeft,
      paddingRight: page.paddingRight,
    },
    header: {
      flexDirection: "row",
      justifyContent: "space-between",
      marginBottom: s.header.style.marginBottom,
    },
    headerLeft: {},
    headerRight: {
      textAlign: "right",
    },
    headerName: {
      fontSize: s.header.style.fontSize,
      fontFamily: resolvePdfFont(
        s.header.style.fontFamily,
        s.header.style.bold,
        s.header.style.italic
      ),
      color: s.header.style.color,
      marginBottom: 2,
    },
    smallText: {
      fontSize: 8,
    },
    senderLine: {
      fontSize: s.senderLine.style.fontSize,
      textDecoration: "underline" as const,
      marginBottom: s.senderLine.style.marginBottom,
      color: s.senderLine.style.color,
    },
    recipientBlock: {
      marginBottom: s.recipient.style.marginBottom,
    },
    metaBlock: {
      flexDirection: "row",
      justifyContent: "space-between",
      marginBottom: s.meta.style.marginBottom,
    },
    recipientSection: {
      width: "55%",
      fontFamily: resolvePdfFont(
        s.recipient.style.fontFamily,
        s.recipient.style.bold,
        s.recipient.style.italic
      ),
      fontSize: s.recipient.style.fontSize,
      color: s.recipient.style.color,
    },
    metaSection: {
      width: "40%",
    },
    metaRow: {
      flexDirection: "row",
      marginBottom: 2,
    },
    metaLabel: {
      fontSize: s.meta.style.fontSize,
      width: "55%",
      color: s.meta.style.color,
    },
    metaValue: {
      fontSize: s.meta.style.fontSize,
      width: "45%",
      color: s.meta.style.color,
    },
    title: {
      fontSize: s.title.style.fontSize,
      fontFamily: resolvePdfFont(
        s.title.style.fontFamily,
        s.title.style.bold,
        s.title.style.italic
      ),
      textAlign: s.title.style.textAlign,
      color: s.title.style.color,
      marginBottom: s.title.style.marginBottom,
    },
    objectLine: {
      marginBottom: s.objectLine.style.marginBottom,
      fontFamily: resolvePdfFont(
        s.objectLine.style.fontFamily,
        s.objectLine.style.bold,
        s.objectLine.style.italic
      ),
      fontSize: s.objectLine.style.fontSize,
      color: s.objectLine.style.color,
    },
    table: {
      marginBottom: s.costTable.style.marginBottom,
    },
    tableHeader: {
      flexDirection: "row",
      borderBottomWidth: 1,
      borderBottomColor: "#000",
      paddingBottom: 4,
      marginBottom: 4,
      fontFamily: resolvePdfFont(s.costTable.style.fontFamily, true, false),
      fontSize: s.costTable.style.fontSize,
      color: s.costTable.style.color,
    },
    tableRow: {
      flexDirection: "row",
      paddingVertical: 2,
      fontSize: s.costTable.style.fontSize,
      color: s.costTable.style.color,
    },
    tableSumRow: {
      flexDirection: "row",
      borderTopWidth: 1,
      borderTopColor: "#000",
      paddingTop: 4,
      marginTop: 4,
      fontFamily: resolvePdfFont(s.costTable.style.fontFamily, true, false),
      fontSize: s.costTable.style.fontSize,
      color: s.costTable.style.color,
    },
    colKostenart: { width: "35%" },
    colKosten: { width: "20%", textAlign: "right" },
    colSchluessel: { width: "25%", textAlign: "center" },
    colAnteil: { width: "20%", textAlign: "right" },
    prepaymentRow: {
      flexDirection: "row",
      marginTop: s.prepayment.style.marginTop,
      marginBottom: s.prepayment.style.marginBottom,
      paddingVertical: 4,
      fontFamily: resolvePdfFont(
        s.prepayment.style.fontFamily,
        s.prepayment.style.bold,
        s.prepayment.style.italic
      ),
      fontSize: s.prepayment.style.fontSize,
      color: s.prepayment.style.color,
    },
    prepaymentLabel: { width: "80%" },
    prepaymentValue: { width: "20%", textAlign: "right" },
    resultRow: {
      flexDirection: "row",
      marginTop: s.result.style.marginTop,
      paddingVertical: 6,
      borderTopWidth: 2,
      borderTopColor: "#000",
      borderBottomWidth: 2,
      borderBottomColor: "#000",
      fontFamily: resolvePdfFont(
        s.result.style.fontFamily,
        s.result.style.bold,
        s.result.style.italic
      ),
      fontSize: s.result.style.fontSize,
      color: s.result.style.color,
    },
    resultLabel: { width: "80%" },
    resultValue: { width: "20%", textAlign: "right" },
    distributionSection: {
      marginTop: s.distributionKeys.style.marginTop,
      marginBottom: s.distributionKeys.style.marginBottom,
    },
    distributionTitle: {
      fontFamily: resolvePdfFont(s.distributionKeys.style.fontFamily, true, false),
      marginBottom: 6,
      fontSize: s.distributionKeys.style.fontSize + 1,
      color: s.distributionKeys.style.color,
    },
    distributionRow: {
      flexDirection: "row",
      paddingVertical: 2,
      fontSize: s.distributionKeys.style.fontSize,
      color: s.distributionKeys.style.color,
    },
    distributionKey: { width: "20%" },
    distributionDesc: { width: "55%" },
    distributionValues: { width: "25%", textAlign: "right" },
    bankSection: {
      marginTop: s.bankInfo.style.marginTop,
      marginBottom: s.bankInfo.style.marginBottom,
    },
    bankTitle: {
      fontFamily: resolvePdfFont(s.bankInfo.style.fontFamily, true, false),
      marginBottom: 4,
      fontSize: s.bankInfo.style.fontSize,
      color: s.bankInfo.style.color,
    },
    closingSection: {
      marginTop: s.closing.style.marginTop,
      fontFamily: resolvePdfFont(
        s.closing.style.fontFamily,
        s.closing.style.bold,
        s.closing.style.italic
      ),
      fontSize: s.closing.style.fontSize,
      color: s.closing.style.color,
    },
    closingText: { marginBottom: 8 },
    greeting: { marginBottom: 4 },
    sectionSpacing: { marginBottom: 6 },
  });
}

// --- PDF Document Component ---

interface BillingPdfProps {
  landlord: {
    name: string;
    street: string;
    zip: string;
    city: string;
    phone: string | null;
    email: string | null;
    bankName: string | null;
    iban: string | null;
    accountHolder: string | null;
  };
  property: {
    street: string;
    zip: string;
    city: string;
    totalShares: number;
  };
  billingPeriod: {
    startDate: Date;
    endDate: Date;
    billingDate: Date | null;
  };
  unit: {
    name: string;
    floor: string;
    shares: number;
  };
  tenant: {
    salutation: string;
    firstName: string;
    lastName: string;
    salutation2: string | null;
    firstName2: string | null;
    lastName2: string | null;
  };
  costs: Array<{
    categoryName: string;
    distributionKey: string;
    totalAmount: number;
    unitAmount: number;
  }>;
  totalCosts: number;
  totalUnitCosts: number;
  totalPrepayment: number;
  templateConfig: PdfTemplateConfig;
}

function BillingPdf({
  landlord,
  property,
  billingPeriod,
  unit,
  tenant,
  costs,
  totalCosts,
  totalUnitCosts,
  totalPrepayment,
  templateConfig,
}: BillingPdfProps) {
  const styles = createStyles(templateConfig);
  const sec = templateConfig.sections;

  const startDate = new Date(billingPeriod.startDate);
  const endDate = new Date(billingPeriod.endDate);
  const billingDate = billingPeriod.billingDate
    ? new Date(billingPeriod.billingDate)
    : null;
  const totalDays = daysBetween(startDate, endDate);
  const year = startDate.getFullYear();

  const difference = totalPrepayment - totalUnitCosts;
  const isNachzahlung = difference < 0;
  const resultLabel = isNachzahlung ? "Nachzahlung:" : "Erstattung:";
  const resultAmount = Math.abs(difference);

  const tenantLine1 = `${tenant.salutation}`;
  const tenantLine2 = `${tenant.firstName} ${tenant.lastName}`;
  const hasTenant2 = tenant.firstName2 && tenant.lastName2;
  const tenantLine3 = hasTenant2
    ? `${tenant.salutation2 ?? ""} ${tenant.firstName2} ${tenant.lastName2}`.trim()
    : null;

  function distributionKeyText(key: string, shares: number): string {
    if (key === "MEA") return `${shares} MEA`;
    if (key === "laut Bescheid") return "laut Bescheid";
    if (key === "siehe Anlage") return "siehe Anlage";
    return key;
  }

  const usedKeys = Array.from(new Set(costs.map((c) => c.distributionKey)));

  const titleText = (sec.title.texts.title || "Neben-/Betriebskostenabrechnung {{year}}").replace(
    "{{year}}",
    String(year)
  );
  const closingText =
    sec.closing.texts.text ||
    "Falls Sie Fragen zu der Abrechnung haben oder Belege einsehen möchten, stehe ich Ihnen gerne zur Verfügung.";
  const greetingText = sec.closing.texts.greeting || "Viele Grüße";
  const paymentNoteText =
    sec.bankInfo.texts.paymentNote ||
    "Nachzahlungen sind, sofern nicht anders vereinbart, einen Monat nach Zugang der Abrechnung fällig.";

  return (
    <Document>
      <Page size="A4" style={styles.page}>
        {/* Header */}
        {sec.header.visible && (
          <View style={styles.header}>
            <View style={styles.headerLeft}>
              <Text style={styles.headerName}>{landlord.name}</Text>
              <Text>{landlord.street}</Text>
              <Text>
                {landlord.zip} {landlord.city}
              </Text>
            </View>
            <View style={styles.headerRight}>
              {landlord.phone && <Text>{landlord.phone}</Text>}
              {landlord.email && <Text>{landlord.email}</Text>}
            </View>
          </View>
        )}

        {/* Sender line */}
        {sec.senderLine.visible && (
          <Text style={styles.senderLine}>
            {landlord.name} - {landlord.street} - {landlord.zip} {landlord.city}
          </Text>
        )}

        {/* Recipient and Meta side by side */}
        <View style={styles.metaBlock}>
          {sec.recipient.visible && (
            <View style={styles.recipientSection}>
              <Text>{tenantLine1}</Text>
              <Text>{tenantLine2}</Text>
              {tenantLine3 && <Text>{tenantLine3}</Text>}
              <Text>{property.street}</Text>
              <Text>
                {property.zip} {property.city}
              </Text>
            </View>
          )}

          {sec.meta.visible && (
            <View style={styles.metaSection}>
              <View style={styles.metaRow}>
                <Text style={styles.metaLabel}>Abrechnungszeitraum:</Text>
                <Text style={styles.metaValue}>
                  {formatDate(startDate)} bis {formatDate(endDate)}
                </Text>
              </View>
              <View style={styles.metaRow}>
                <Text style={styles.metaLabel}>Kalendertage gesamt:</Text>
                <Text style={styles.metaValue}>{totalDays}</Text>
              </View>
              <View style={styles.metaRow}>
                <Text style={styles.metaLabel}>Ihr Abrechnungszeitraum:</Text>
                <Text style={styles.metaValue}>
                  {formatDate(startDate)} bis {formatDate(endDate)}
                </Text>
              </View>
              <View style={styles.metaRow}>
                <Text style={styles.metaLabel}>Geschoss:</Text>
                <Text style={styles.metaValue}>{unit.floor}</Text>
              </View>
              {billingDate && (
                <View style={styles.metaRow}>
                  <Text style={styles.metaLabel}>Datum der Abrechnung:</Text>
                  <Text style={styles.metaValue}>{formatDate(billingDate)}</Text>
                </View>
              )}
            </View>
          )}
        </View>

        {/* Title */}
        {sec.title.visible && <Text style={styles.title}>{titleText}</Text>}

        {/* Object line */}
        {sec.objectLine.visible && (
          <Text style={styles.objectLine}>
            Objekt: {property.street}, {property.zip} {property.city}
          </Text>
        )}

        {/* Cost table */}
        {sec.costTable.visible && (
          <View style={styles.table}>
            <View style={styles.tableHeader}>
              <Text style={styles.colKostenart}>Kostenart</Text>
              <Text style={styles.colKosten}>Kosten</Text>
              <Text style={styles.colSchluessel}>Verteilerschlüssel</Text>
              <Text style={styles.colAnteil}>Ihr Anteil</Text>
            </View>

            {costs.map((cost, index) => (
              <View style={styles.tableRow} key={index}>
                <Text style={styles.colKostenart}>{cost.categoryName}</Text>
                <Text style={styles.colKosten}>
                  {formatCurrency(cost.totalAmount)} EUR
                </Text>
                <Text style={styles.colSchluessel}>
                  {distributionKeyText(cost.distributionKey, unit.shares)}
                </Text>
                <Text style={styles.colAnteil}>
                  {formatCurrency(cost.unitAmount)} EUR
                </Text>
              </View>
            ))}

            <View style={styles.tableSumRow}>
              <Text style={styles.colKostenart}>gesamte Kosten:</Text>
              <Text style={styles.colKosten}>
                {formatCurrency(totalCosts)} EUR
              </Text>
              <Text style={styles.colSchluessel}>Ihr Anteil:</Text>
              <Text style={styles.colAnteil}>
                {formatCurrency(totalUnitCosts)} EUR
              </Text>
            </View>
          </View>
        )}

        {/* Prepayment */}
        {sec.prepayment.visible && (
          <View style={styles.prepaymentRow}>
            <Text style={styles.prepaymentLabel}>
              Ihre Vorauszahlungen Neben-/Betriebskosten:
            </Text>
            <Text style={styles.prepaymentValue}>
              {formatCurrency(totalPrepayment)} EUR
            </Text>
          </View>
        )}

        {/* Result */}
        {sec.result.visible && (
          <View style={styles.resultRow}>
            <Text style={styles.resultLabel}>{resultLabel}</Text>
            <Text style={styles.resultValue}>
              {formatCurrency(resultAmount)} EUR
            </Text>
          </View>
        )}

        {/* Distribution key explanation */}
        {sec.distributionKeys.visible && (
          <View style={styles.distributionSection}>
            <Text style={styles.distributionTitle}>
              Erläuterung der Verteilerschlüssel:
            </Text>
            {usedKeys.includes("MEA") && (
              <View style={styles.distributionRow}>
                <Text style={styles.distributionKey}>MEA</Text>
                <Text style={styles.distributionDesc}>
                  Miteigentumsanteile laut Teilungserklärung
                </Text>
                <Text style={styles.distributionValues}>
                  {unit.shares} / {property.totalShares}
                </Text>
              </View>
            )}
            {usedKeys.includes("laut Bescheid") && (
              <View style={styles.distributionRow}>
                <Text style={styles.distributionKey}>laut Bescheid</Text>
                <Text style={styles.distributionDesc}>
                  laut Grundsteuerbescheid
                </Text>
                <Text style={styles.distributionValues}></Text>
              </View>
            )}
            {usedKeys.includes("siehe Anlage") && (
              <View style={styles.distributionRow}>
                <Text style={styles.distributionKey}>siehe Anlage</Text>
                <Text style={styles.distributionDesc}>
                  Abrechnung des Dienstleisters oder Gebührenbescheid
                </Text>
                <Text style={styles.distributionValues}></Text>
              </View>
            )}
          </View>
        )}

        {/* Bank info */}
        {sec.bankInfo.visible && (
          <View style={styles.bankSection}>
            <Text style={styles.bankTitle}>
              Bankverbindung und Zahlungsdetails
            </Text>
            {landlord.accountHolder && landlord.iban && (
              <Text>
                {landlord.accountHolder}, IBAN: {landlord.iban}
                {landlord.bankName ? `, ${landlord.bankName}` : ""}
              </Text>
            )}
            <Text style={{ marginTop: 4, fontSize: 9 }}>{paymentNoteText}</Text>
          </View>
        )}

        {/* Closing */}
        {sec.closing.visible && (
          <View style={styles.closingSection}>
            <Text style={styles.closingText}>{closingText}</Text>
            <Text style={styles.greeting}>{greetingText}</Text>
            <Text>{landlord.name}</Text>
          </View>
        )}
      </Page>
    </Document>
  );
}

// --- Route Handler ---

export async function GET(
  request: Request,
  { params: paramsPromise }: { params: Promise<{ id: string }> }
) {
  try {
    const session = await auth();
    if (!session?.user?.id) {
      return new Response(JSON.stringify({ error: "Nicht angemeldet" }), {
        status: 401,
        headers: { "Content-Type": "application/json" },
      });
    }

    const { id } = await paramsPromise;

    // Load PDF template config
    const templateRow = await prisma.pdfTemplate.findFirst({
      orderBy: { updatedAt: "desc" },
    });
    const templateConfig: PdfTemplateConfig = templateRow
      ? JSON.parse(templateRow.config)
      : getDefaultConfig();

    // Fetch billing period with all related data
    const billingPeriod = await prisma.billingPeriod.findUnique({
      where: { id },
      include: {
        property: {
          include: {
            units: {
              include: {
                tenants: true,
                prepayments: {
                  where: { billingPeriodId: id },
                },
              },
            },
          },
        },
        costs: {
          include: {
            costCategory: true,
          },
          orderBy: {
            costCategory: {
              sortOrder: "asc",
            },
          },
        },
      },
    });

    if (!billingPeriod) {
      return new Response(
        JSON.stringify({ error: "Billing period not found" }),
        { status: 404, headers: { "Content-Type": "application/json" } }
      );
    }

    const landlord = await prisma.landlordInfo.findFirst();

    if (!landlord) {
      return new Response(
        JSON.stringify({ error: "Landlord info not configured" }),
        { status: 400, headers: { "Content-Type": "application/json" } }
      );
    }

    const property = billingPeriod.property;
    const startDate = new Date(billingPeriod.startDate);
    const endDate = new Date(billingPeriod.endDate);

    let targetUnit = null;
    let activeTenant = null;

    for (const unit of property.units) {
      const t = unit.tenants.find(
        (t: { moveInDate: Date; moveOutDate: Date | null }) => {
          const moveIn = new Date(t.moveInDate);
          const moveOut = t.moveOutDate ? new Date(t.moveOutDate) : null;
          return moveIn <= endDate && (moveOut === null || moveOut >= startDate);
        }
      );

      if (t) {
        targetUnit = unit;
        activeTenant = t;
        break;
      }
    }

    if (!targetUnit || !activeTenant) {
      return new Response(
        JSON.stringify({
          error: "No active tenant found for this billing period",
        }),
        { status: 400, headers: { "Content-Type": "application/json" } }
      );
    }

    const costs = billingPeriod.costs.map(
      (cost: {
        costCategory: { name: string; distributionKey: string };
        totalAmount: number;
        unitAmount: number | null;
      }) => ({
        categoryName: cost.costCategory.name,
        distributionKey: cost.costCategory.distributionKey,
        totalAmount: cost.totalAmount,
        unitAmount: cost.unitAmount ?? 0,
      })
    );

    const totalCosts = costs.reduce(
      (sum: number, c: { totalAmount: number }) => sum + c.totalAmount,
      0
    );
    const totalUnitCosts = costs.reduce(
      (sum: number, c: { unitAmount: number }) => sum + c.unitAmount,
      0
    );

    const prepayment = targetUnit.prepayments.find(
      (p: { billingPeriodId: string }) => p.billingPeriodId === id
    );
    const months = daysBetween(startDate, endDate) / 30.44;
    const totalPrepayment = prepayment
      ? prepayment.monthlyAmount * Math.round(months)
      : 0;

    const year = startDate.getFullYear();

    const buffer = await renderToBuffer(
      <BillingPdf
        landlord={landlord}
        property={property}
        billingPeriod={billingPeriod}
        unit={targetUnit}
        tenant={activeTenant}
        costs={costs}
        totalCosts={totalCosts}
        totalUnitCosts={totalUnitCosts}
        totalPrepayment={totalPrepayment}
        templateConfig={templateConfig}
      />
    );

    return new Response(new Uint8Array(buffer), {
      headers: {
        "Content-Type": "application/pdf",
        "Content-Disposition": `attachment; filename="Betriebskostenabrechnung-${year}.pdf"`,
      },
    });
  } catch (error) {
    console.error("Failed to generate billing PDF:", error);
    return new Response(
      JSON.stringify({ error: "Failed to generate PDF" }),
      { status: 500, headers: { "Content-Type": "application/json" } }
    );
  }
}
