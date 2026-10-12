import { describe, expect, it } from "vitest";
import { render } from "./markdown";

describe("markdown", () => {
  it("BUB-4.2 renders Markdown with highlighted code", () => {
    const html = render("**bold**\n\n```js\nconst a = 1;\n```");
    expect(html).toContain("<strong>bold</strong>");
    expect(html).toContain('class="hljs-keyword"');
  });

  it("BUB-4.2 raw HTML in model output is shown as text, never rendered", () => {
    const html = render('<script>alert(1)</script> <img src=x onerror="alert(1)"> <iframe></iframe>');
    expect(html).not.toMatch(/<script|<img|<iframe/);
    expect(html).toContain("&lt;script&gt;");
  });

  it("BUB-4.2 only safe links become links", () => {
    expect(render("[docs](https://example.com)")).toContain('<a href="https://example.com">docs</a>');
    expect(render("see https://example.com")).toContain('href="https://example.com"');
    for (const bad of ["javascript:alert(1)", "vbscript:x", "file:///C:/secret.txt", "data:text/html,<b>x</b>"]) {
      expect(render(`[x](${bad})`)).not.toContain("href=");
    }
  });

  it("BUB-4.2 images are not loaded, they show as text", () => {
    const html = render("![tracker](https://evil.example/pixel.png)");
    expect(html).not.toContain("<img");
    expect(html).toContain("tracker");
  });
});
