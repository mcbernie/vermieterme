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

// --- Styles ---

const styles = StyleSheet.create({
  page: {
    fontFamily: "Helvetica",
    fontSize: 10,
    paddingTop: 40,
    paddingBottom: 40,
    paddingLeft: 50,
    paddingRight: 50,
  },
  header: {
    flexDirection: "row",
    justifyContent: "space-between",
    marginBottom: 20,
  },
  headerLeft: {},
  headerRight: {
    textAlign: "right",
  },
  headerName: {
    fontSize: 12,
    fontFamily: "Helvetica-Bold",
    marginBottom: 2,
  },
  smallText: {
    fontSize: 8,
  },
  senderLine: {
    fontSize: 7,
    textDecoration: "underline",
    marginBottom: 4,
    color: "#555",
  },
  recipientBlock: {
    marginBottom: 10,
  },
  metaBlock: {
    flexDirection: "row",
    justifyContent: "space-between",
    marginBottom: 20,
  },
  recipientSection: {
    width: "55%",
  },
  metaSection: {
    width: "40%",
  },
  metaRow: {
    flexDirection: "row",
    marginBottom: 2,
  },
  metaLabel: {
    fontSize: 8,
    width: "55%",
  },
  metaValue: {
    fontSize: 8,
    width: "45%",
  },
  title: {
    fontSize: 14,
    fontFamily: "Helvetica-Bold",
    textAlign: "center",
    marginBottom: 12,
  },
  objectLine: {
    marginBottom: 12,
    fontFamily: "Helvetica-Bold",
  },
  // Table styles
  table: {
    marginBottom: 12,
  },
  tableHeader: {
    flexDirection: "row",
    borderBottomWidth: 1,
    borderBottomColor: "#000",
    paddingBottom: 4,
    marginBottom: 4,
    fontFamily: "Helvetica-Bold",
  },
  tableRow: {
    flexDirection: "row",
    paddingVertical: 2,
  },
  tableSumRow: {
    flexDirection: "row",
    borderTopWidth: 1,
    borderTopColor: "#000",
    paddingTop: 4,
    marginTop: 4,
    fontFamily: "Helvetica-Bold",
  },
  colKostenart: {
    width: "35%",
  },
  colKosten: {
    width: "20%",
    textAlign: "right",
  },
  colSchluessel: {
    width: "25%",
    textAlign: "center",
  },
  colAnteil: {
    width: "20%",
    textAlign: "right",
  },
  prepaymentRow: {
    flexDirection: "row",
    marginTop: 8,
    marginBottom: 4,
    paddingVertical: 4,
  },
  prepaymentLabel: {
    width: "80%",
  },
  prepaymentValue: {
    width: "20%",
    textAlign: "right",
  },
  resultRow: {
    flexDirection: "row",
    marginTop: 4,
    paddingVertical: 6,
    borderTopWidth: 2,
    borderTopColor: "#000",
    borderBottomWidth: 2,
    borderBottomColor: "#000",
    fontFamily: "Helvetica-Bold",
  },
  resultLabel: {
    width: "80%",
  },
  resultValue: {
    width: "20%",
    textAlign: "right",
  },
  // Distribution key explanation table
  distributionSection: {
    marginTop: 20,
    marginBottom: 16,
  },
  distributionTitle: {
    fontFamily: "Helvetica-Bold",
    marginBottom: 6,
    fontSize: 10,
  },
  distributionRow: {
    flexDirection: "row",
    paddingVertical: 2,
    fontSize: 9,
  },
  distributionKey: {
    width: "20%",
  },
  distributionDesc: {
    width: "55%",
  },
  distributionValues: {
    width: "25%",
    textAlign: "right",
  },
  // Bank info
  bankSection: {
    marginTop: 16,
    marginBottom: 16,
  },
  bankTitle: {
    fontFamily: "Helvetica-Bold",
    marginBottom: 4,
  },
  // Closing
  closingSection: {
    marginTop: 20,
  },
  closingText: {
    marginBottom: 8,
  },
  greeting: {
    marginBottom: 4,
  },
  sectionSpacing: {
    marginBottom: 6,
  },
});

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
}: BillingPdfProps) {
  const startDate = new Date(billingPeriod.startDate);
  const endDate = new Date(billingPeriod.endDate);
  const billingDate = billingPeriod.billingDate
    ? new Date(billingPeriod.billingDate)
    : new Date();
  const totalDays = daysBetween(startDate, endDate);
  const year = startDate.getFullYear();

  const difference = totalPrepayment - totalUnitCosts;
  const isNachzahlung = difference < 0;
  const resultLabel = isNachzahlung ? "Nachzahlung:" : "Erstattung:";
  const resultAmount = Math.abs(difference);

  // Build tenant name lines
  const tenantLine1 = `${tenant.salutation}`;
  const tenantLine2 = `${tenant.firstName} ${tenant.lastName}`;
  const hasTenant2 =
    tenant.firstName2 && tenant.lastName2;
  const tenantLine3 = hasTenant2
    ? `${tenant.salutation2 ?? ""} ${tenant.firstName2} ${tenant.lastName2}`.trim()
    : null;

  // Distribution key display
  function distributionKeyText(key: string, shares: number): string {
    if (key === "MEA") return `${shares} MEA`;
    if (key === "laut Bescheid") return "laut Bescheid";
    if (key === "siehe Anlage") return "siehe Anlage";
    return key;
  }

  // Collect unique distribution keys for explanation table
  const usedKeys = Array.from(new Set(costs.map((c) => c.distributionKey)));

  return (
    <Document>
      <Page size="A4" style={styles.page}>
        {/* Header */}
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

        {/* Sender line */}
        <Text style={styles.senderLine}>
          {landlord.name} - {landlord.street} - {landlord.zip} {landlord.city}
        </Text>

        {/* Recipient and Meta side by side */}
        <View style={styles.metaBlock}>
          <View style={styles.recipientSection}>
            <Text>{tenantLine1}</Text>
            <Text>{tenantLine2}</Text>
            {tenantLine3 && <Text>{tenantLine3}</Text>}
            <Text>{property.street}</Text>
            <Text>
              {property.zip} {property.city}
            </Text>
          </View>

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
            <View style={styles.metaRow}>
              <Text style={styles.metaLabel}>Datum der Abrechnung:</Text>
              <Text style={styles.metaValue}>{formatDate(billingDate)}</Text>
            </View>
          </View>
        </View>

        {/* Title */}
        <Text style={styles.title}>
          Neben-/Betriebskostenabrechnung {year}
        </Text>

        {/* Object line */}
        <Text style={styles.objectLine}>
          Objekt: {property.street}, {property.zip} {property.city}
        </Text>

        {/* Cost table */}
        <View style={styles.table}>
          {/* Table header */}
          <View style={styles.tableHeader}>
            <Text style={styles.colKostenart}>Kostenart</Text>
            <Text style={styles.colKosten}>Kosten</Text>
            <Text style={styles.colSchluessel}>Verteilerschlüssel</Text>
            <Text style={styles.colAnteil}>Ihr Anteil</Text>
          </View>

          {/* Cost rows */}
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

          {/* Sum row */}
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

        {/* Prepayment */}
        <View style={styles.prepaymentRow}>
          <Text style={styles.prepaymentLabel}>
            Ihre Vorauszahlungen Neben-/Betriebskosten:
          </Text>
          <Text style={styles.prepaymentValue}>
            {formatCurrency(totalPrepayment)} EUR
          </Text>
        </View>

        {/* Result */}
        <View style={styles.resultRow}>
          <Text style={styles.resultLabel}>{resultLabel}</Text>
          <Text style={styles.resultValue}>
            {formatCurrency(resultAmount)} EUR
          </Text>
        </View>

        {/* Distribution key explanation */}
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

        {/* Bank info */}
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
          <Text style={{ marginTop: 4, fontSize: 9 }}>
            Nachzahlungen sind, sofern nicht anders vereinbart, einen Monat nach
            Zugang der Abrechnung fällig.
          </Text>
        </View>

        {/* Closing */}
        <View style={styles.closingSection}>
          <Text style={styles.closingText}>
            Falls Sie Fragen zu der Abrechnung haben oder Belege einsehen
            möchten, stehe ich Ihnen gerne zur Verfügung.
          </Text>
          <Text style={styles.greeting}>Viele Grüße</Text>
          <Text>{landlord.name}</Text>
        </View>
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
      return new Response(JSON.stringify({ error: "Nicht angemeldet" }), { status: 401, headers: { "Content-Type": "application/json" } });
    }

    const { id } = await paramsPromise;

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
      return new Response(JSON.stringify({ error: "Billing period not found" }), {
        status: 404,
        headers: { "Content-Type": "application/json" },
      });
    }

    // Fetch landlord info
    const landlord = await prisma.landlordInfo.findFirst();

    if (!landlord) {
      return new Response(
        JSON.stringify({ error: "Landlord info not configured" }),
        {
          status: 400,
          headers: { "Content-Type": "application/json" },
        }
      );
    }

    const property = billingPeriod.property;
    const startDate = new Date(billingPeriod.startDate);
    const endDate = new Date(billingPeriod.endDate);

    // Find the first unit with an active tenant
    let targetUnit = null;
    let activeTenant = null;

    for (const unit of property.units) {
      const tenant = unit.tenants.find((t: { moveInDate: Date; moveOutDate: Date | null }) => {
        const moveIn = new Date(t.moveInDate);
        const moveOut = t.moveOutDate ? new Date(t.moveOutDate) : null;
        return moveIn <= endDate && (moveOut === null || moveOut >= startDate);
      });

      if (tenant) {
        targetUnit = unit;
        activeTenant = tenant;
        break;
      }
    }

    if (!targetUnit || !activeTenant) {
      return new Response(
        JSON.stringify({ error: "No active tenant found for this billing period" }),
        {
          status: 400,
          headers: { "Content-Type": "application/json" },
        }
      );
    }

    // Build cost data
    const costs = billingPeriod.costs.map((cost: { costCategory: { name: string; distributionKey: string }; totalAmount: number; unitAmount: number | null }) => ({
      categoryName: cost.costCategory.name,
      distributionKey: cost.costCategory.distributionKey,
      totalAmount: cost.totalAmount,
      unitAmount: cost.unitAmount ?? 0,
    }));

    const totalCosts = costs.reduce((sum: number, c: { totalAmount: number }) => sum + c.totalAmount, 0);
    const totalUnitCosts = costs.reduce((sum: number, c: { unitAmount: number }) => sum + c.unitAmount, 0);

    // Calculate total prepayment for this unit
    const prepayment = targetUnit.prepayments.find(
      (p: { billingPeriodId: string }) => p.billingPeriodId === id
    );
    const months = daysBetween(startDate, endDate) / 30.44; // approximate months
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
      {
        status: 500,
        headers: { "Content-Type": "application/json" },
      }
    );
  }
}
