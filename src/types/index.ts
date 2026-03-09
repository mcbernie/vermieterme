// === Property ===

export interface Property {
  id: string;
  street: string;
  zip: string;
  city: string;
  totalShares: number;
}

export interface PropertyWithCount extends Property {
  _count: { units: number };
}

export interface PropertyWithUnits extends Property {
  units: UnitWithTenants[];
}

// === Unit ===

export interface Unit {
  id: string;
  propertyId: string;
  name: string;
  floor: string;
  shares: number;
}

export interface UnitWithTenants extends Unit {
  tenants: Tenant[];
}

// === Tenant ===

export interface Tenant {
  id: string;
  unitId: string;
  salutation: string;
  firstName: string;
  lastName: string;
  salutation2?: string | null;
  firstName2?: string | null;
  lastName2?: string | null;
  moveInDate: string;
  moveOutDate: string | null;
}

export interface TenantWithUnit extends Tenant {
  unit: Unit & {
    property?: Property;
  };
}

// === Billing ===

export interface BillingPeriod {
  id: string;
  propertyId: string;
  startDate: string;
  endDate: string;
  billingDate: string | null;
}

export interface BillingPeriodWithProperty extends BillingPeriod {
  property: Property;
  _count?: { costs: number };
}

export interface BillingPeriodDetail extends BillingPeriod {
  property: PropertyWithUnits;
  costs: Cost[];
  prepayments: Prepayment[];
}

export interface Cost {
  id: string;
  billingPeriodId: string;
  costCategoryId: string;
  totalAmount: number;
  unitAmount: number | null;
}

export interface Prepayment {
  id: string;
  billingPeriodId: string;
  unitId: string;
  monthlyAmount: number;
}

// === Settings ===

export interface CostCategory {
  id: string;
  name: string;
  distributionKey: string;
  sortOrder: number;
}

export interface LandlordInfo {
  id?: string;
  name: string;
  street: string;
  zip: string;
  city: string;
  phone: string;
  email: string;
  bankName: string;
  iban: string;
  accountHolder: string;
}

// === User ===

export interface AppUser {
  id: string;
  name: string | null;
  email: string | null;
  image?: string | null;
}
