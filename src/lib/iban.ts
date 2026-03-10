/**
 * Validates an IBAN using the ISO 13616 standard.
 * Returns true if valid, false if invalid or empty.
 */
export function validateIBAN(iban: string): boolean {
  if (!iban) return false;

  // Remove spaces and convert to uppercase
  const cleaned = iban.replace(/\s/g, "").toUpperCase();

  // Check basic format: 2 letter country code + 2 digits + up to 30 alphanumeric characters
  if (!/^[A-Z]{2}\d{2}[A-Z0-9]{4,30}$/.test(cleaned)) return false;

  // Check country-specific length
  const lengths: Record<string, number> = {
    DE: 22, AT: 20, CH: 21, LI: 21, LU: 20, FR: 27, NL: 18,
    BE: 16, IT: 27, ES: 24, PT: 25, PL: 28, CZ: 24, GB: 22,
    IE: 22, DK: 18, SE: 24, NO: 15, FI: 18, GR: 27, HU: 28,
    SK: 24, SI: 19, HR: 21, RO: 24, BG: 22, EE: 20, LT: 20,
    LV: 21, MT: 31, CY: 28,
  };

  const countryCode = cleaned.substring(0, 2);
  if (lengths[countryCode] && cleaned.length !== lengths[countryCode]) return false;

  // Move first 4 characters to end
  const rearranged = cleaned.substring(4) + cleaned.substring(0, 4);

  // Convert letters to numbers (A=10, B=11, ..., Z=35)
  let numStr = "";
  for (const char of rearranged) {
    if (char >= "A" && char <= "Z") {
      numStr += (char.charCodeAt(0) - 55).toString();
    } else {
      numStr += char;
    }
  }

  // Calculate modulo 97 using string-based division (number too large for JS)
  let remainder = 0;
  for (const digit of numStr) {
    remainder = (remainder * 10 + parseInt(digit)) % 97;
  }

  return remainder === 1;
}

/**
 * Formats an IBAN with spaces every 4 characters.
 */
export function formatIBAN(iban: string): string {
  const cleaned = iban.replace(/\s/g, "").toUpperCase();
  return cleaned.replace(/(.{4})/g, "$1 ").trim();
}
