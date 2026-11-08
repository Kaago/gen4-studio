import * as THREE from "three";
import { describe, expect, it } from "vitest";
import type { TileBehavior } from "../../api";
import { PATTERN_CODE, patternOf, setPatternAt, withPatterns } from "./patterns";

const behavior = (name: string, cls: string): TileBehavior => ({ value: 0, name, class: cls });

describe("patternOf", () => {
  it("blocked_cells_are_hatched", () => {
    expect(patternOf(0, 0x80, undefined)).toEqual({ pattern: "hatch", dir: 0 });
    // Collision wins over the class, as with the colour.
    expect(patternOf(2, 0x80, behavior("TALL_GRASS", "grass"))).toEqual({ pattern: "hatch", dir: 0 });
    expect(patternOf(0, 0x81, behavior("X", "water")).pattern).toBe("hatch");
  });

  it("water_waves_grass_dots_ice_crosshatch", () => {
    expect(patternOf(0x10, 0, behavior("DEEP_WATER", "water")).pattern).toBe("waves");
    expect(patternOf(2, 0, behavior("TALL_GRASS", "grass")).pattern).toBe("dots");
    expect(patternOf(0x20, 0, behavior("ICE", "ice")).pattern).toBe("crosshatch");
  });

  it("ledges_point_where_they_jump", () => {
    expect(patternOf(0x38, 0, behavior("JUMP_EAST", "ledge"))).toEqual({ pattern: "arrow", dir: 1 });
    expect(patternOf(0x3a, 0, behavior("JUMP_NORTH", "ledge"))).toEqual({ pattern: "arrow", dir: 0 });
    expect(patternOf(0x3b, 0, behavior("JUMP_SOUTH", "ledge"))).toEqual({ pattern: "arrow", dir: 2 });
    expect(patternOf(0x5c, 0, behavior("JUMP_WEST_TWICE", "ledge"))).toEqual({ pattern: "arrow", dir: 3 });
    // A ledge carries 0x80 but jumps: it stays an arrow.
    expect(patternOf(0x39, 0x80, behavior("JUMP_WEST", "ledge"))).toEqual({ pattern: "arrow", dir: 3 });
    // No direction in the name: north.
    expect(patternOf(0x77, 0, behavior("LEDGE", "ledge"))).toEqual({ pattern: "arrow", dir: 0 });
  });

  it("unknown_classes_have_none", () => {
    expect(patternOf(0, 0, undefined)).toEqual({ pattern: "none", dir: 0 });
    expect(patternOf(0x69, 0, behavior("DOOR", "door")).pattern).toBe("none");
    expect(patternOf(6, 0, behavior("HEADBUTT", "other")).pattern).toBe("none");
  });

  it("numbers_the_patterns_as_the_shader_reads_them", () => {
    expect(PATTERN_CODE).toEqual({ none: 0, hatch: 1, waves: 2, dots: 3, arrow: 4, crosshatch: 5 });
  });
});

describe("withPatterns / setPatternAt", () => {
  const mesh = () => new THREE.InstancedMesh(new THREE.PlaneGeometry(1, 1), new THREE.MeshBasicMaterial({ transparent: true, opacity: 0.42 }), 4);

  it("with_patterns_adds_the_attribute_and_the_shader_hook", () => {
    const m = mesh();
    withPatterns(m);
    const attr = m.geometry.getAttribute("aPattern") as THREE.InstancedBufferAttribute;
    expect(attr).toBeInstanceOf(THREE.InstancedBufferAttribute);
    expect(attr.itemSize).toBe(2);
    expect(attr.count).toBe(4);
    expect([...attr.array]).toEqual([0, 0, 0, 0, 0, 0, 0, 0]);

    const material = m.material as THREE.MeshBasicMaterial;
    const shader = {
      uniforms: {},
      vertexShader: "#include <common>\nvoid main() {\n#include <begin_vertex>\n}",
      fragmentShader: "#include <common>\nvoid main() {\n#include <color_fragment>\n}",
    } as unknown as THREE.WebGLProgramParametersWithUniforms;
    material.onBeforeCompile(shader, {} as THREE.WebGLRenderer);
    expect(shader.vertexShader).toContain("attribute vec2 aPattern;");
    expect(shader.vertexShader).toMatch(/vPattern = aPattern;/);
    expect(shader.fragmentShader).toContain("#include <color_fragment>");
    // The hook follows the colour chunk: the pattern pixels take the cell colour × 0.55 at twice the opacity.
    expect(shader.fragmentShader.indexOf("0.55")).toBeGreaterThan(shader.fragmentShader.indexOf("#include <color_fragment>"));
    expect(shader.fragmentShader).toContain("min(1.0, 2.0 * opacity)");
    expect(shader.fragmentShader).toContain("fract((uv.x + uv.y) * 4.0) < 0.22");
    // Its own program: an overlay with patterns does not share the shader of a plain basic material.
    expect(material.customProgramCacheKey()).not.toBe(new THREE.MeshBasicMaterial().customProgramCacheKey());
  });

  it("set_pattern_at_writes_code_and_direction_of_one_instance", () => {
    const m = mesh();
    withPatterns(m);
    const attr = m.geometry.getAttribute("aPattern") as THREE.InstancedBufferAttribute;
    setPatternAt(m, 2, { pattern: "arrow", dir: 3 });
    expect(attr.getX(2)).toBe(PATTERN_CODE.arrow);
    expect(attr.getY(2)).toBe(3);
    expect(attr.getX(1)).toBe(0);
  });
});
