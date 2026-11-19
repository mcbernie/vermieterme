import { fireEvent, render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import { PropertyCard } from "@/features/properties/property-card";

const property = { id: "a", street: "Hauptstr. 1", zip: "12345", city: "Berlin", totalShares: 100, _count: { units: 0 } };

function setup(overrides: Partial<Parameters<typeof PropertyCard>[0]> = {}) {
  const handlers = {
    onToggle: vi.fn(),
    onEdit: vi.fn(),
    onDelete: vi.fn(),
    onAddUnit: vi.fn(),
    onEditUnit: vi.fn(),
    onDeleteUnit: vi.fn(),
  };
  render(
    <PropertyCard
      property={property}
      expanded={false}
      detail={null}
      detailFailed={false}
      rentChanges={[]}
      {...handlers}
      {...overrides}
    />,
  );
  return handlers;
}

describe("PropertyCard", () => {
  it("toggles when the header (text or free area) is clicked", () => {
    const h = setup();
    fireEvent.click(screen.getByText("Hauptstr. 1"));
    fireEvent.click(screen.getByText(/Anteile/));
    expect(h.onToggle).toHaveBeenCalledTimes(2);
  });

  it("toggles via the keyboard-reachable title button", () => {
    const h = setup();
    fireEvent.click(screen.getByRole("button", { name: /Hauptstr. 1/ }));
    expect(h.onToggle).toHaveBeenCalledTimes(1);
  });

  it("does not toggle when an action button is clicked", () => {
    const h = setup();
    fireEvent.click(screen.getByRole("button", { name: "Bearbeiten" }));
    fireEvent.click(screen.getByRole("button", { name: "Löschen" }));
    expect(h.onEdit).toHaveBeenCalledTimes(1);
    expect(h.onDelete).toHaveBeenCalledTimes(1);
    expect(h.onToggle).not.toHaveBeenCalled();
  });

  it("shows loading, then an error notice for the units", () => {
    const { unmount } = render(
      <PropertyCard
        property={property}
        expanded
        detail={null}
        detailFailed={false}
        rentChanges={[]}
        onToggle={vi.fn()}
        onEdit={vi.fn()}
        onDelete={vi.fn()}
        onAddUnit={vi.fn()}
        onEditUnit={vi.fn()}
        onDeleteUnit={vi.fn()}
      />,
    );
    expect(screen.queryByText(/konnten nicht geladen/)).toBeNull();
    unmount();
    setup({ expanded: true, detailFailed: true });
    expect(screen.getByText(/konnten nicht geladen/)).toBeInTheDocument();
  });
});
