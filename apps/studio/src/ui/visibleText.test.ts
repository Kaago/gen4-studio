// @vitest-environment jsdom
import { afterEach, describe, expect, it } from "vitest";
import { visibleWords } from "./visibleText";

describe("visibleWords", () => {
  afterEach(() => {
    document.body.innerHTML = "";
  });

  const host = (html: string) => {
    const div = document.createElement("div");
    div.innerHTML = html;
    document.body.append(div);
    return div;
  };

  it("joins_the_text_of_every_element", () => {
    expect(visibleWords(host("<p>Zweiblattdorf</p><ul><li>Route 201</li><li> Raum  1 </li></ul>"))).toBe("Zweiblattdorf Route 201 Raum  1");
  });

  it("leaves_out_hidden_aria_hidden_and_display_none_with_their_children", () => {
    const root = host(
      '<p>Sichtbar</p><div hidden><span>Versteckt</span></div><span aria-hidden="true">›</span><span aria-hidden="false">Auch sichtbar</span>' +
        '<div style="display: none"><b>Weg</b></div><div style="display: block">Da</div>',
    );
    expect(visibleWords(root)).toBe("Sichtbar Auch sichtbar Da");
  });

  it("leaves_out_text_under_visibility_hidden_or_collapse_but_reads_a_child_that_is_visible_again", () => {
    // Dockview keeps an inactive panel mounted under an overlay with `visibility: hidden`.
    const root = host(
      '<p>Schau</p><div style="visibility: hidden"><span>Raster</span><input placeholder="Matrix"><b style="visibility: visible">Wieder da</b></div>' +
        '<table><tr style="visibility: collapse"><td>Header</td></tr></table>',
    );
    expect(visibleWords(root)).toBe("Schau Wieder da");
  });

  it("returns_nothing_for_a_concealed_root", () => {
    expect(visibleWords(host("<p>Raster</p>").appendChild(Object.assign(document.createElement("div"), { hidden: true, textContent: "Matrix" })))).toBe("");
  });

  it("does_not_read_tooltips_accessible_names_or_field_values_but_placeholders", () => {
    const root = host('<button title="Header 0003" aria-label="Matrix 0">Ort</button><input value="T01" placeholder="Ort suchen"><textarea placeholder="Notiz">Text</textarea>');
    expect(visibleWords(root)).toBe("Ort Ort suchen Notiz");
  });

  it("reads_only_the_chosen_option_of_a_select", () => {
    const root = host('<select><option value="0">Header</option><option value="1" selected>Gebiet</option></select>');
    expect(visibleWords(root)).toBe("Gebiet");
  });

  it("leaves_out_scripts_and_styles", () => {
    expect(visibleWords(host("<style>.Raster{}</style><script>var Matrix</script><p>Karte</p>"))).toBe("Karte");
  });
});
