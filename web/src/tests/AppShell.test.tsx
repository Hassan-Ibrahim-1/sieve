import { fireEvent, render, screen } from "@testing-library/react";
import { vi } from "vitest";
import { AppShell } from "../app/AppShell";

describe("AppShell", () => {
  it("switches between the dependency and proof-step views from the top bar", () => {
    const onView = vi.fn();
    render(<AppShell theme="light" view="sieve" onTheme={vi.fn()} onView={onView}
      controls={<div />} inspector={<div />}><div /></AppShell>);

    expect(screen.getByRole("combobox", { name: "View" })).toHaveValue("sieve");
    fireEvent.change(screen.getByRole("combobox", { name: "View" }), { target: { value: "proof" } });
    expect(onView).toHaveBeenCalledWith("proof");
  });
});
