import { describe, expect, it } from "vitest";
import { readCss, ruleBody } from "../cssSource";

const baseCss = readCss("./base.css", import.meta.url);
const shellCss = readCss("../frame/Shell.module.css", import.meta.url);
const galleryCss = readCss("../gallery/Gallery.module.css", import.meta.url);

describe("the document never scrolls", () => {
  it("html, body and #root are exactly viewport high and clip their overflow", () => {
    for (const selector of ["html", "body", "#root"]) {
      const body = ruleBody(baseCss, selector);
      expect(body, selector).toMatch(/height:\s*100%/);
      expect(body, selector).toMatch(/overflow:\s*hidden/);
    }
  });

  it("the shell clips what hidden dock panels position outside it (clip cannot be scrolled by scrollIntoView)", () => {
    const body = ruleBody(shellCss, ".shell");
    expect(body).toMatch(/position:\s*relative/);
    expect(body).toMatch(/overflow:\s*clip/);
    expect(body).toMatch(/height:\s*100%/);
  });

  it("the gallery scrolls its own content pane instead of the document", () => {
    expect(ruleBody(galleryCss, ".gallery")).toMatch(/(?:^|[;\s])height:\s*100vh/);
    expect(ruleBody(galleryCss, ".page")).toMatch(/overflow:\s*auto/);
    expect(ruleBody(galleryCss, ".nav")).toMatch(/overflow:\s*auto/);
  });
});
