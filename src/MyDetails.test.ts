// @ts-expect-error type error without @types/node package
import { readFileSync } from "node:fs";
import { expect, it } from "vitest";
import { details, SECTIONS } from "./MyDetails";

// Rust leaves an empty box named like one of the tab's without asking a model, so the two lists must agree.
it("FIL-2.3 the tab's boxes are fill::FORM's names, in its order", () => {
  const rust: string = readFileSync(new URL("../src-tauri/src/fill/mod.rs", import.meta.url), "utf8");
  const form = rust.match(/const FORM: \[&str; \d+\] = \[([^\]]+)\]/)![1];
  expect([...form.matchAll(/"([^"]+)"/g)].map((m) => m[1])).toEqual(SECTIONS.flatMap(([, names]) => names));
});

it("FIL-1.2 the details are the boxes with a value, the address lines joined, then More details' lines", () => {
  const boxes: Record<string, string> = { Email: " a@b.c ", "Address line 1": "12 MG Road", "Address line 2": "Indiranagar" };
  const more = "Blood group: O+\r\nno colon here\n\nLinkedIn : https://in.example/p\nhttps://github.com/priya\n: no name\nEmpty:  ";
  expect(details((label) => boxes[label] ?? "", more)).toEqual([
    { label: "Email", value: "a@b.c" },
    { label: "Address line 1", value: "12 MG Road" },
    { label: "Address line 2", value: "Indiranagar" },
    { label: "Address", value: "12 MG Road, Indiranagar" },
    { label: "Blood group", value: "O+" },
    { label: "LinkedIn", value: "https://in.example/p" },
  ]);
  expect(details(() => "", "")).toEqual([]);
});
