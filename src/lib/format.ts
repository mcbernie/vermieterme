export function formatCurrency(amount: number): string {
  return new Intl.NumberFormat("de-DE", {
    style: "currency",
    currency: "EUR",
  }).format(amount);
}

export function formatDate(date: Date | string): string {
  return new Intl.DateTimeFormat("de-DE").format(new Date(date));
}

export function formatDateISO(date: Date | string): string {
  const d = new Date(date);
  return d.toISOString().split("T")[0];
}
