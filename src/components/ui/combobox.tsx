"use client";

import * as React from "react";
import { cn } from "@/lib/utils";
import { CheckIcon, ChevronsUpDownIcon, SearchIcon } from "lucide-react";

export interface ComboboxOption {
  value: string;
  label: string;
  group?: string;
}

interface ComboboxProps {
  options: ComboboxOption[];
  value: string;
  onChange: (value: string) => void;
  placeholder?: string;
  searchPlaceholder?: string;
  emptyText?: string;
  className?: string;
  required?: boolean;
}

export function Combobox({
  options,
  value,
  onChange,
  placeholder = "Bitte wählen...",
  searchPlaceholder = "Suchen...",
  emptyText = "Keine Ergebnisse.",
  className,
  required,
}: ComboboxProps) {
  const [open, setOpen] = React.useState(false);
  const [search, setSearch] = React.useState("");
  const containerRef = React.useRef<HTMLDivElement>(null);
  const inputRef = React.useRef<HTMLInputElement>(null);

  const selectedOption = options.find((o) => o.value === value);

  const filteredOptions = React.useMemo(() => {
    if (!search) return options;
    const lower = search.toLowerCase();
    return options.filter(
      (o) =>
        o.label.toLowerCase().includes(lower) ||
        (o.group && o.group.toLowerCase().includes(lower))
    );
  }, [options, search]);

  // Group options
  const grouped = React.useMemo(() => {
    const groups: Record<string, ComboboxOption[]> = {};
    filteredOptions.forEach((opt) => {
      const key = opt.group || "";
      if (!groups[key]) groups[key] = [];
      groups[key].push(opt);
    });
    return groups;
  }, [filteredOptions]);

  // Close on outside click
  React.useEffect(() => {
    function handleClickOutside(e: MouseEvent) {
      if (
        containerRef.current &&
        !containerRef.current.contains(e.target as Node)
      ) {
        setOpen(false);
        setSearch("");
      }
    }
    if (open) {
      document.addEventListener("mousedown", handleClickOutside);
      return () =>
        document.removeEventListener("mousedown", handleClickOutside);
    }
  }, [open]);

  // Focus search input when opened
  React.useEffect(() => {
    if (open && inputRef.current) {
      inputRef.current.focus();
    }
  }, [open]);

  return (
    <div ref={containerRef} className={cn("relative", className)}>
      {/* Hidden input for form validation */}
      {required && (
        <input
          type="text"
          required
          value={value}
          onChange={() => {}}
          className="sr-only"
          tabIndex={-1}
          aria-hidden="true"
        />
      )}
      <button
        type="button"
        onClick={() => setOpen(!open)}
        className={cn(
          "flex w-full items-center justify-between rounded-lg border border-zinc-300 bg-white px-3 py-2 text-left text-sm transition-colors hover:bg-zinc-50 focus:border-zinc-500 focus:outline-none focus:ring-1 focus:ring-zinc-500",
          !selectedOption && "text-zinc-400"
        )}
      >
        <span className="truncate">
          {selectedOption ? selectedOption.label : placeholder}
        </span>
        <ChevronsUpDownIcon className="ml-2 h-4 w-4 shrink-0 text-zinc-400" />
      </button>

      {open && (
        <div className="absolute z-50 mt-1 w-full overflow-hidden rounded-lg border border-zinc-200 bg-white shadow-lg">
          <div className="flex items-center border-b border-zinc-100 px-3">
            <SearchIcon className="mr-2 h-4 w-4 shrink-0 text-zinc-400" />
            <input
              ref={inputRef}
              type="text"
              value={search}
              onChange={(e) => setSearch(e.target.value)}
              placeholder={searchPlaceholder}
              className="w-full py-2.5 text-sm outline-none placeholder:text-zinc-400"
            />
          </div>
          <div className="max-h-60 overflow-y-auto p-1">
            {filteredOptions.length === 0 ? (
              <div className="px-3 py-6 text-center text-sm text-zinc-500">
                {emptyText}
              </div>
            ) : (
              Object.entries(grouped).map(([group, opts]) => (
                <div key={group}>
                  {group && (
                    <div className="px-2 py-1.5 text-xs font-medium text-zinc-500">
                      {group}
                    </div>
                  )}
                  {opts.map((opt) => (
                    <button
                      key={opt.value}
                      type="button"
                      onClick={() => {
                        onChange(opt.value);
                        setOpen(false);
                        setSearch("");
                      }}
                      className={cn(
                        "flex w-full items-center rounded-md px-2 py-1.5 text-sm transition-colors hover:bg-zinc-100",
                        opt.value === value && "bg-zinc-50"
                      )}
                    >
                      <CheckIcon
                        className={cn(
                          "mr-2 h-4 w-4 shrink-0",
                          opt.value === value
                            ? "text-zinc-900 opacity-100"
                            : "opacity-0"
                        )}
                      />
                      <span className="truncate">{opt.label}</span>
                    </button>
                  ))}
                </div>
              ))
            )}
          </div>
        </div>
      )}
    </div>
  );
}
