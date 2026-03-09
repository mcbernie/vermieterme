"use client";

import { useRef } from "react";

interface OptionalDateInputProps {
  value: string;
  onChange: (value: string) => void;
  label?: string;
  placeholder?: string;
}

export function OptionalDateInput({
  value,
  onChange,
  placeholder = "Nicht gesetzt",
}: OptionalDateInputProps) {
  const inputRef = useRef<HTMLInputElement>(null);

  if (!value) {
    return (
      <button
        type="button"
        onClick={() => {
          // Set today as starting point, then let user change
          const today = new Date().toISOString().slice(0, 10);
          onChange(today);
          // Focus the input after it renders
          setTimeout(() => inputRef.current?.showPicker?.(), 50);
        }}
        className="w-full rounded-lg border border-dashed border-zinc-300 px-3 py-2 text-left text-sm text-zinc-400 hover:border-zinc-400 hover:text-zinc-500"
      >
        {placeholder}
      </button>
    );
  }

  return (
    <div className="flex gap-1">
      <input
        ref={inputRef}
        type="date"
        value={value}
        onChange={(e) => onChange(e.target.value)}
        className="flex-1 rounded-lg border border-zinc-300 px-3 py-2 text-sm focus:border-zinc-500 focus:outline-none focus:ring-1 focus:ring-zinc-500"
      />
      <button
        type="button"
        onClick={() => onChange("")}
        className="rounded-lg border border-zinc-300 px-2 text-zinc-400 hover:bg-zinc-50 hover:text-zinc-600"
        title="Datum entfernen"
      >
        &times;
      </button>
    </div>
  );
}
